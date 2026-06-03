use std::sync::Arc;

use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};

use crate::context::{colours, BotData, BotError, CID_RAID_CLEAR};
use crate::ui::{self, Button, ButtonStyle, Container, Spacing};

#[derive(Clone, Copy)]
enum LogStream { Moderation, Chat }

async fn get_log_channel_id(
    data: &Arc<BotData>,
    guild_id: serenity::GuildId,
    stream: LogStream,
) -> Option<serenity::ChannelId> {
    let cfg = data.db.get_or_create_guild(&guild_id.to_string()).await.ok()?;
    let raw = match stream {
        LogStream::Moderation => cfg.mod_log_channel_id.or(cfg.log_channel_id),
        LogStream::Chat => cfg.chat_log_channel_id.or(cfg.log_channel_id),
    }?;
    raw.parse::<u64>().ok().map(serenity::ChannelId::new)
}

/// Public accessor used by mod_cmd.rs to find the moderation log channel.
pub async fn get_mod_log(
    data: &Arc<BotData>,
    guild_id: serenity::GuildId,
) -> Option<serenity::ChannelId> {
    get_log_channel_id(data, guild_id, LogStream::Moderation).await
}

async fn send_log_embed(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    guild_id: serenity::GuildId,
    stream: LogStream,
    embed: serenity::CreateEmbed<'_>,
) {
    let Some(channel_id) = get_log_channel_id(data, guild_id, stream).await else { return; };
    channel_id
        .widen()
        .send_message(&ctx.http, serenity::CreateMessage::new().embed(embed))
        .await
        .ok();
}

/// Clamp a slowmode seconds value into serenity-`next`'s `NonMaxU16`.
fn to_nonmax(secs: u16) -> serenity::nonmax::NonMaxU16 {
    serenity::nonmax::NonMaxU16::new(secs).unwrap_or_default()
}

pub async fn handle_event(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    data: &Arc<BotData>,
) -> Result<(), BotError> {
    match event {
        serenity::FullEvent::Message { new_message, .. } => {
            handle_message(ctx, data, new_message).await;
        }

        serenity::FullEvent::MessageDelete {
            channel_id,
            deleted_message_id,
            guild_id: Some(gid),
            ..
        } => {
            if let Some(log_channel) = get_log_channel_id(data, *gid, LogStream::Chat).await {
                if *channel_id == log_channel.widen() {
                    return Ok(());
                }
            }
            let embed = serenity::CreateEmbed::new()
                .colour(colours::RED)
                .title("🗑️ Message Deleted")
                .description(format!("Channel: <#{}>\nMessage ID: {}", channel_id, deleted_message_id))
                .timestamp(serenity::Timestamp::now());
            send_log_embed(ctx, data, *gid, LogStream::Chat, embed).await;
        }

        serenity::FullEvent::MessageUpdate { old_if_available, event, .. } => {
            let new_msg = &event.message;
            if let Some(gid) = new_msg.guild_id {
                if let Some(log_channel) = get_log_channel_id(data, gid, LogStream::Chat).await {
                    if new_msg.channel_id == log_channel.widen() { return Ok(()); }
                }
                let Some(old_msg) = old_if_available.as_ref() else {
                    return Ok(());
                };
                if new_msg.author.bot() { return Ok(()); }
                let old_content = old_msg.content.to_string();
                let new_content = new_msg.content.to_string();
                if old_content.trim().is_empty() && new_content.trim().is_empty() { return Ok(()); }
                if old_content != new_content {
                    let truncate = |s: String| if s.len() > 500 { format!("{}…", &s[..500]) } else { s };
                    let embed = serenity::CreateEmbed::new()
                        .colour(colours::YELLOW)
                        .title("✏️ Message Edited")
                        .field("Channel", format!("<#{}>", new_msg.channel_id), true)
                        .field("Message ID", new_msg.id.to_string(), true)
                        .field("Before", truncate(old_content), false)
                        .field("After", truncate(new_content), false)
                        .timestamp(serenity::Timestamp::now());
                    send_log_embed(ctx, data, gid, LogStream::Chat, embed).await;
                }
            }
        }

        serenity::FullEvent::GuildMemberAddition { new_member, .. } => {
            handle_member_join(ctx, data, new_member).await;
        }

        serenity::FullEvent::GuildMemberRemoval { guild_id, user, .. } => {
            let embed = serenity::CreateEmbed::new()
                .colour(colours::RED)
                .title("👋 Member Left")
                .description(format!("<@{}> left or was removed", user.id))
                .timestamp(serenity::Timestamp::now());
            send_log_embed(ctx, data, *guild_id, LogStream::Moderation, embed).await;
        }

        serenity::FullEvent::GuildBanAddition { guild_id, banned_user, .. } => {
            let embed = serenity::CreateEmbed::new()
                .colour(colours::DARK_RED)
                .title("🔨 Member Banned")
                .description(format!("<@{}> was banned", banned_user.id))
                .timestamp(serenity::Timestamp::now());
            send_log_embed(ctx, data, *guild_id, LogStream::Moderation, embed).await;
        }

        serenity::FullEvent::GuildBanRemoval { guild_id, unbanned_user, .. } => {
            let embed = serenity::CreateEmbed::new()
                .colour(colours::GREEN)
                .title("🔓 Member Unbanned")
                .description(format!("<@{}> was unbanned", unbanned_user.id))
                .timestamp(serenity::Timestamp::now());
            send_log_embed(ctx, data, *guild_id, LogStream::Moderation, embed).await;
        }

        serenity::FullEvent::ChannelCreate { channel, .. } => {
            let embed = serenity::CreateEmbed::new()
                .colour(colours::GREEN)
                .title("➕ Channel Created")
                .description(format!("{} (<#{}>)", channel.base.name, channel.id))
                .timestamp(serenity::Timestamp::now());
            send_log_embed(ctx, data, channel.base.guild_id, LogStream::Moderation, embed).await;
        }

        serenity::FullEvent::ChannelDelete { channel, .. } => {
            let embed = serenity::CreateEmbed::new()
                .colour(colours::RED)
                .title("➖ Channel Deleted")
                .description(format!("{} ({})", channel.base.name, channel.id))
                .timestamp(serenity::Timestamp::now());
            send_log_embed(ctx, data, channel.base.guild_id, LogStream::Moderation, embed).await;
        }

        serenity::FullEvent::VoiceStateUpdate { old, new, .. } => {
            if let Some(gid) = new.guild_id {
                let action = match (old.as_ref().and_then(|v| v.channel_id), new.channel_id) {
                    (None, Some(to)) => format!("Joined <#{}>", to),
                    (Some(from), None) => format!("Left <#{}>", from),
                    (Some(from), Some(to)) if from != to => format!("Moved <#{}> → <#{}>", from, to),
                    _ => return Ok(()),
                };
                let embed = serenity::CreateEmbed::new()
                    .colour(colours::BLURPLE)
                    .title("🔊 Voice State")
                    .description(format!("<@{}>: {}", new.user_id, action))
                    .timestamp(serenity::Timestamp::now());
                send_log_embed(ctx, data, gid, LogStream::Moderation, embed).await;
            }
        }

        _ => {}
    }

    Ok(())
}

// ── Member join → anti-raid ───────────────────────────────────────────────────

async fn handle_member_join(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    member: &serenity::Member,
) {
    let guild_id = member.guild_id;

    // Log event
    let embed = serenity::CreateEmbed::new()
        .colour(colours::GREEN)
        .title("👋 Member Joined")
        .description(format!("<@{}> joined the server", member.user.id))
        .timestamp(serenity::Timestamp::now());
    send_log_embed(ctx, data, guild_id, LogStream::Moderation, embed).await;

    // Raid scoring
    let raid_cfg = match data.db.get_or_create_raid_config(&guild_id.to_string()).await {
        Ok(c) => c,
        Err(e) => { warn!("Failed to load raid config: {}", e); return; }
    };

    let account_age_days = {
        let created = member.user.id.created_at().unix_timestamp();
        let now = serenity::Timestamp::now().unix_timestamp();
        (now - created).max(0) as f64 / 86400.0
    };

    let (score, triggered) = data
        .raid
        .record_join(guild_id, account_age_days, raid_cfg.decay_rate, raid_cfg.join_threshold, raid_cfg.raid_active)
        .await;

    if triggered {
        info!("Raid detected in guild {} (score={:.2})", guild_id, score);
        if let Err(e) = data.db.set_raid_active(&guild_id.to_string(), true).await {
            error!("Failed to set raid active: {}", e);
        }
        post_raid_alert(ctx, data, guild_id, score).await;
        apply_raid_slowmode(ctx, data, guild_id, raid_cfg.slowmode_secs as u16).await;
    }

    // Auto-recovery check (only when already active)
    if raid_cfg.raid_active {
        let (_, should_clear) = data
            .raid
            .check_decay(guild_id, raid_cfg.decay_rate, raid_cfg.join_threshold)
            .await;
        if should_clear {
            info!("Auto-clearing raid mode in guild {} (score decayed)", guild_id);
            clear_raid_mode(ctx, data, guild_id).await;
        }
    }
}

async fn post_raid_alert(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    guild_id: serenity::GuildId,
    score: f64,
) {
    let Some(ch) = get_log_channel_id(data, guild_id, LogStream::Moderation).await else { return; };

    let clear_btn =
        Button::new(CID_RAID_CLEAR, "Clear Raid Mode", ButtonStyle::Danger).emoji("✅");

    let card = Container::new(vec![
        ui::text(format!(
            "**⚠️ Raid Detected**\nJoin score: `{:.2}`\nSlowmode applied to all text channels.\nClick below when the raid is over.",
            score
        )),
        ui::separator(true, Spacing::Small),
        ui::action_row(vec![clear_btn.into()]),
    ])
    .accent(colours::DARK_RED.0);

    ui::send(&ctx.http, ch, &[card.into()]).await.ok();
}

pub async fn apply_raid_slowmode(
    ctx: &serenity::Context,
    _data: &Arc<BotData>,
    guild_id: serenity::GuildId,
    slowmode_secs: u16,
) {
    let channels = match guild_id.channels(&ctx.http).await {
        Ok(c) => c,
        Err(e) => { warn!("Failed to fetch channels for raid slowmode: {}", e); return; }
    };

    for channel in channels {
        if !matches!(channel.base.kind, serenity::ChannelType::Text | serenity::ChannelType::News) {
            continue;
        }
        channel.id
            .edit(&ctx.http, serenity::EditChannel::new().rate_limit_per_user(to_nonmax(slowmode_secs)))
            .await
            .ok();
    }
}

pub async fn clear_raid_mode(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    guild_id: serenity::GuildId,
) {
    data.raid.reset(guild_id).await;
    if let Err(e) = data.db.set_raid_active(&guild_id.to_string(), false).await {
        error!("Failed to clear raid active flag: {}", e);
    }
    // Remove slowmode from all text channels
    let channels = guild_id.channels(&ctx.http).await.unwrap_or_default();
    for channel in channels {
        if !matches!(channel.base.kind, serenity::ChannelType::Text | serenity::ChannelType::News) {
            continue;
        }
        channel.id
            .edit(&ctx.http, serenity::EditChannel::new().rate_limit_per_user(to_nonmax(0)))
            .await
            .ok();
    }
    info!("Raid mode cleared for guild {}", guild_id);
}

// ── Message → auto-slowmode ───────────────────────────────────────────────────

async fn handle_message(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    msg: &serenity::Message,
) {
    // Only guild text channels
    let Some(guild_id) = msg.guild_id else { return; };

    // Capture message in open ticket threads (for transcript future use).
    // serenity `next` splits threads into a distinct `Channel::GuildThread`.
    let channel = match msg.channel_id.to_channel(ctx, msg.guild_id).await {
        Ok(serenity::Channel::GuildThread(_)) => {
            if let Ok(Some(ticket)) = data.db.get_ticket_by_thread(&msg.channel_id.to_string()).await {
                data.db.touch_ticket(&ticket.thread_id).await.ok();
                let atts = if !msg.attachments.is_empty() {
                    let urls: Vec<String> =
                        msg.attachments.iter().map(|a| a.url.to_string()).collect();
                    serde_json::to_string(&urls).ok()
                } else {
                    None
                };
                let sent_at = msg.timestamp.to_rfc3339();
                data.db.save_message(
                    ticket.id,
                    Some(&msg.id.to_string()),
                    &msg.author.id.to_string(),
                    &msg.author.name,
                    &msg.content,
                    atts.as_deref(),
                    &sent_at,
                ).await.ok();
            }
            return;
        }
        Ok(serenity::Channel::Guild(c)) => c,
        _ => return,
    };

    if !matches!(channel.base.kind, serenity::ChannelType::Text | serenity::ChannelType::News) {
        return;
    }

    // Auto-slowmode
    let slowmode_cfg = match data.db.get_or_create_slowmode_config(&guild_id.to_string()).await {
        Ok(c) => c,
        Err(_) => return,
    };

    if !slowmode_cfg.enabled { return; }

    let excluded: std::collections::HashSet<String> = slowmode_cfg.excluded_channel_ids().into_iter().collect();
    if excluded.contains(&msg.channel_id.to_string()) { return; }

    let target = data.raid
        .record_message(
            msg.channel_id.expect_channel(),
            slowmode_cfg.capacity as f64,
            slowmode_cfg.window_secs as f64,
        )
        .await;

    if let Some(secs) = target {
        msg.channel_id
            .expect_channel()
            .edit(&ctx.http, serenity::EditChannel::new().rate_limit_per_user(to_nonmax(secs)))
            .await
            .ok();
    }
}
