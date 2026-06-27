mod config;
mod context;
mod db;
mod events;
mod features;
mod handlers;
mod ids;
mod permissions;
mod ui;
mod util;

use std::sync::Arc;

use poise::serenity_prelude as serenity;
use tracing::info;

use crate::config::AppConfig;
use crate::context::{BotData, BotError};
use crate::db::Database;

// ── Framework error handler ───────────────────────────────────────────────────

async fn on_error(error: poise::FrameworkError<'_, BotData, BotError>) {
    use poise::FrameworkError::*;
    match error {
        Command { error, ctx, .. } => {
            let is_dev = ctx.data().config.bot.environment.is_dev();
            let reply = match &error {
                // User-facing errors are always shown verbatim.
                BotError::User(msg) => msg.clone(),
                _ => {
                    if is_dev {
                        format!("**Error (dev):** `{:?}`", error)
                    } else {
                        let code = uuid::Uuid::new_v4()
                            .to_string()
                            .replace('-', "")
                            .chars()
                            .take(8)
                            .collect::<String>();
                        tracing::error!("Command error [ref:{}]: {:?}", code, error);
                        format!("Something went wrong. Reference code: `{}`", code)
                    }
                }
            };
            ctx.send(poise::CreateReply::default().ephemeral(true).content(reply))
                .await
                .ok();
        }

        error => {
            poise::builtins::on_error(error)
                .await
                .unwrap_or_else(|e| tracing::error!("Unhandled framework error: {:?}", e));
        }
    }
}

// ── Serenity event handler ────────────────────────────────────────────────────
//
// poise `serenity-next` removed the `FrameworkOptions::event_handler` hook and the
// `.setup()` builder step, so non-command events, startup command registration, and
// manual component/modal dispatch all live in our own `serenity::EventHandler`,
// registered alongside the poise framework on the client.

struct Handler {
    data: Arc<BotData>,
    commands: Vec<poise::Command<BotData, BotError>>,
    guild_id: serenity::GuildId,
    started: std::sync::atomic::AtomicBool,
}

#[serenity::async_trait]
impl serenity::EventHandler for Handler {
    async fn dispatch(&self, ctx: &serenity::Context, event: &serenity::FullEvent) {
        // Message / member-join / ban / log-stream events.
        if let Err(e) = events::handle_event(ctx, event, &self.data).await {
            tracing::warn!("Event handler error: {:?}", e);
        }

        match event {
            // One-shot startup work (guild command registration + background task).
            serenity::FullEvent::Ready { data_about_bot, .. } => {
                if self
                    .started
                    .swap(true, std::sync::atomic::Ordering::SeqCst)
                {
                    return;
                }
                info!(
                    "Connected as {} ({})",
                    data_about_bot.user.name, data_about_bot.user.id
                );
                info!("Registering commands in guild {}", self.guild_id);
                if let Err(e) =
                    poise::builtins::register_in_guild(&ctx.http, &self.commands, self.guild_id)
                        .await
                {
                    tracing::error!("Command registration failed: {:?}", e);
                } else {
                    info!("Guild commands registered");
                }
                let bot_id = data_about_bot.user.id;
                tokio::spawn(auto_close_task(ctx.http.clone(), self.data.clone(), bot_id));
                tokio::spawn(expire_bans_task(ctx.http.clone(), self.data.clone(), bot_id));
            }

            // poise handles ApplicationCommand interactions; we dispatch the two
            // interaction types it ignores.
            serenity::FullEvent::InteractionCreate { interaction, .. } => match interaction {
                serenity::Interaction::Component(ci) => {
                    handlers::dispatch::component(ctx, &self.data, ci).await;
                }
                serenity::Interaction::Modal(mi) => {
                    handlers::dispatch::modal(ctx, &self.data, mi).await;
                }
                _ => {}
            },

            _ => {}
        }
    }
}

// ── Auto-close background task ────────────────────────────────────────────────

async fn auto_close_task(http: Arc<serenity::Http>, data: Arc<BotData>, bot_id: serenity::UserId) {
    // 30-minute poll cycle; skip the first immediate tick.
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30 * 60));
    interval.tick().await;

    loop {
        interval.tick().await;
        tracing::debug!("Auto-close: scanning for stale tickets…");

        match data.db.get_stale_tickets().await {
            Ok(tickets) => {
                if !tickets.is_empty() {
                    info!("Auto-closing {} stale ticket(s)", tickets.len());
                }
                for ticket in &tickets {
                    if let Err(e) = features::tickets::service::execute_close(
                        &http,
                        &data,
                        ticket,
                        bot_id,
                        Some("Auto-closed due to inactivity"),
                    )
                    .await
                    {
                        tracing::warn!(
                            "Auto-close failed for ticket #{}: {:?}",
                            ticket.ticket_number,
                            e
                        );
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Auto-close DB error: {:?}", e);
            }
        }
    }
}

// ── Temporary-ban expiry background task ──────────────────────────────────────

async fn expire_bans_task(http: Arc<serenity::Http>, data: Arc<BotData>, bot_id: serenity::UserId) {
    // 5-minute poll cycle; skip the first immediate tick.
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5 * 60));
    interval.tick().await;

    loop {
        interval.tick().await;

        let bans = match data.db.get_active_temp_bans().await {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!("Temp-ban expiry DB error: {:?}", e);
                continue;
            }
        };
        let now = serenity::Timestamp::now().unix_timestamp();
        for inf in &bans {
            let expired = inf
                .expires_at
                .as_deref()
                .and_then(|e| chrono::DateTime::parse_from_rfc3339(e).ok())
                .map(|dt| dt.timestamp() <= now)
                .unwrap_or(false);
            if !expired {
                continue;
            }
            let (Ok(gid), Ok(uid)) = (inf.guild_id.parse::<u64>(), inf.user_id.parse::<u64>())
            else {
                data.db.deactivate_infraction(inf.id).await.ok();
                continue;
            };
            let guild_id = serenity::GuildId::new(gid);
            let user_id = serenity::UserId::new(uid);
            info!("Lifting expired temporary ban for {} in {}", uid, gid);
            if let Err(e) = features::moderation::service::lift_ban(
                &http,
                &data,
                guild_id,
                user_id,
                bot_id,
                "Temporary ban expired",
            )
            .await
            {
                // Already unbanned manually, or a transient API error — deactivate
                // the row either way so the task doesn't retry it every cycle.
                tracing::warn!("Temp-ban expiry for {} in {}: {:?}", uid, gid, e);
                data.db.deactivate_active_bans(&inf.guild_id, &inf.user_id).await.ok();
            }
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 0. Install a rustls CryptoProvider. serenity-next and sqlx pull in multiple
    //    rustls backends, so rustls can't pick one automatically.
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("Failed to install rustls crypto provider"))?;

    // 1. Load config.toml
    let config = AppConfig::load()?;

    // 2. Initialise tracing
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.bot.log_level));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    info!("Starting pip ({:?})", config.bot.environment);

    // 3. Connect to SQLite and run migrations
    let db = Database::new(&config.db_url()).await?;
    db.migrate().await?;
    info!("Database ready at {}", config.database.path);

    // 4. Shared bot state — one Arc shared between Poise and the event handler.
    let bot_data = Arc::new(BotData::new(db, config.clone()));
    let guild_id = serenity::GuildId::new(config.guild.id);

    // 5. Build the poise framework (commands only — events/setup are in `Handler`).
    let framework = poise::Framework::new(poise::FrameworkOptions {
        commands: commands(),
        on_error: |error| Box::pin(on_error(error)),
        ..Default::default()
    });

    // 6. Our serenity event handler owns a second copy of the command list (used
    //    once at Ready to register), plus the shared data + guild id.
    let handler = Handler {
        data: bot_data.clone(),
        commands: commands(),
        guild_id,
        started: std::sync::atomic::AtomicBool::new(false),
    };

    // 7. Build the serenity client: data + event handler + poise framework.
    let intents = serenity::GatewayIntents::GUILDS
        | serenity::GatewayIntents::GUILD_MESSAGES
        | serenity::GatewayIntents::MESSAGE_CONTENT
        | serenity::GatewayIntents::GUILD_MEMBERS
        | serenity::GatewayIntents::GUILD_MODERATION
        | serenity::GatewayIntents::DIRECT_MESSAGES;

    let token = config
        .bot
        .token
        .parse::<serenity::Token>()
        .map_err(|e| anyhow::anyhow!("Invalid Discord token: {}", e))?;

    let mut client = serenity::ClientBuilder::new(token, intents)
        .data(bot_data.clone())
        .event_handler(Arc::new(handler))
        .framework(Box::new(framework))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to build Serenity client: {}", e))?;

    // 8. Start the gateway connection
    info!("pip is online — serving guild {}", guild_id);
    client
        .start()
        .await
        .map_err(|e| anyhow::anyhow!("Client stopped: {}", e))?;

    Ok(())
}

/// The full application command list. Built fresh on each call so the framework
/// and the registration step can each own an independent copy.
fn commands() -> Vec<poise::Command<BotData, BotError>> {
    vec![
        // ── General ──────────────────────────────────────────────
        features::general::ping(),
        features::general::help(),
        // ── Setup ────────────────────────────────────────────────
        features::setup::setup(),
        // ── Tickets ──────────────────────────────────────────────
        features::tickets::ticket(),
        // ── Blocklist ────────────────────────────────────────────
        features::blocklist::blocklist(),
        // ── Moderation ───────────────────────────────────────────
        features::moderation::warn(),
        features::moderation::timeout(),
        features::moderation::untimeout(),
        features::moderation::kick(),
        features::moderation::ban(),
        features::moderation::unban(),
        features::moderation::history(),
        // ── Concerns ─────────────────────────────────────────────
        features::concerns::concerns(),
        // ── Roles ─────────────────────────────────────────────────
        features::roles::role(),
        // ── Reports ──────────────────────────────────────────────
        features::reports::report(),
        // ── Message context menus ─────────────────────────────────
        features::reports::report_message(),
        features::tickets::open_ticket_from_message(),
        features::moderation::delete_and_warn(),
        // ── User context menus ────────────────────────────────────
        features::reports::report_user(),
        features::moderation::view_history(),
        features::moderation::warn_user(),
        features::moderation::timeout_user(),
        features::moderation::kick_user(),
        features::moderation::ban_user(),
        features::tickets::open_ticket_with_user(),
    ]
}
