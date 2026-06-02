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

async fn on_error(error: poise::FrameworkError<'_, Arc<BotData>, BotError>) {
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

        Setup { error, .. } => {
            tracing::error!("Framework setup error: {:?}", error);
        }

        EventHandler { error, event, .. } => {
            tracing::warn!(
                "Event handler error ({}): {:?}",
                event.snake_case_name(),
                error
            );
        }

        error => {
            poise::builtins::on_error(error)
                .await
                .unwrap_or_else(|e| tracing::error!("Unhandled framework error: {:?}", e));
        }
    }
}

// ── Combined event + interaction dispatcher ───────────────────────────────────

async fn event_handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _fw: poise::FrameworkContext<'_, Arc<BotData>, BotError>,
    data: &Arc<BotData>,
) -> Result<(), BotError> {
    // Dispatch message / member-join / log stream events.
    events::handle_event(ctx, event, data).await?;

    // Poise handles ApplicationCommand interactions automatically for registered
    // commands.  We manually dispatch the two interaction types it ignores.
    if let serenity::FullEvent::InteractionCreate { interaction } = event {
        match interaction {
            serenity::Interaction::Component(ci) => {
                handlers::dispatch::component(ctx, data, ci).await;
            }
            serenity::Interaction::Modal(mi) => {
                handlers::dispatch::modal(ctx, data, mi).await;
            }
            _ => {}
        }
    }

    Ok(())
}

// ── Auto-close background task ────────────────────────────────────────────────

async fn auto_close_task(http: Arc<serenity::Http>, data: Arc<BotData>) {
    // Resolve the bot's own UserId once so close messages attribute correctly.
    let bot_id = http
        .get_current_user()
        .await
        .map(|u| u.id)
        .unwrap_or(serenity::UserId::new(1));

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

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    // 5. Build Poise framework
    let framework = {
        let bot_data = bot_data.clone();
        poise::Framework::builder()
            .options(poise::FrameworkOptions {
                commands: vec![
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
                    features::appeals::appeal(),
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
                ],
                on_error: |error| Box::pin(on_error(error)),
                event_handler: |ctx, event, fw, data| {
                    Box::pin(event_handler(ctx, event, fw, data))
                },
                ..Default::default()
            })
            .setup(move |ctx, ready, framework| {
                let data = bot_data.clone();
                let http = ctx.http.clone();
                Box::pin(async move {
                    info!("Connected as {} ({})", ready.user.name, ready.user.id);
                    info!("Registering commands in guild {}", guild_id);
                    poise::builtins::register_in_guild(
                        ctx,
                        &framework.options().commands,
                        guild_id,
                    )
                    .await?;
                    info!("Guild commands registered");

                    // Spawn the auto-close background task — http is available now.
                    tokio::spawn(auto_close_task(http, data.clone()));

                    Ok(data)
                })
            })
            .build()
    };

    // 6. Build Serenity client
    let intents = serenity::GatewayIntents::GUILDS
        | serenity::GatewayIntents::GUILD_MESSAGES
        | serenity::GatewayIntents::MESSAGE_CONTENT
        | serenity::GatewayIntents::GUILD_MEMBERS
        | serenity::GatewayIntents::GUILD_MODERATION
        | serenity::GatewayIntents::DIRECT_MESSAGES;

    let mut client = serenity::ClientBuilder::new(&config.bot.token, intents)
        .framework(framework)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to build Serenity client: {}", e))?;

    // 7. Start the gateway connection
    info!("pip is online — serving guild {}", guild_id);
    client
        .start()
        .await
        .map_err(|e| anyhow::anyhow!("Client stopped: {}", e))?;

    Ok(())
}
