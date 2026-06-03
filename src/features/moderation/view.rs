//! Moderation log embed posted to the configured mod-log channel.

use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::{colours, BotData};

/// Emoji, title, and accent colour for a moderation action kind.
fn action_style(kind: &str) -> (&'static str, &'static str, serenity::Colour) {
    match kind {
        "ban" => ("🔨", "Member Banned", colours::RED),
        "kick" => ("👢", "Member Kicked", colours::ORANGE),
        "warn" => ("⚠️", "Member Warned", colours::YELLOW),
        "timeout" => ("⏱️", "Member Timed Out", colours::YELLOW),
        "untimeout" => ("⏱️", "Timeout Removed", colours::GREEN),
        "unban" => ("🔓", "Member Unbanned", colours::GREEN),
        _ => ("•", "Moderation Action", colours::ORANGE),
    }
}

/// Post a structured moderation-action embed to the mod-log channel, if configured.
/// `kind` drives the title + per-action colour; `case_id` is the infraction id;
/// `extra` carries action-specific detail (duration/expiry, appealable, jump link).
#[allow(clippy::too_many_arguments)]
pub async fn log_action(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    guild_id: serenity::GuildId,
    kind: &str,
    case_id: Option<i64>,
    target: &serenity::User,
    moderator: &serenity::User,
    reason: &str,
    extra: Option<&str>,
) {
    let Some(ch) = crate::events::get_mod_log(data, guild_id).await else {
        return;
    };

    let (emoji, label, colour) = action_style(kind);
    let title = match case_id {
        Some(id) => format!("{emoji} {label} · Case #{id}"),
        None => format!("{emoji} {label}"),
    };

    let mut embed = serenity::CreateEmbed::new()
        .colour(colour)
        .title(title)
        .thumbnail(target.face())
        .field(
            "User",
            format!("<@{}> · {} · `{}`", target.id, target.name, target.id),
            false,
        )
        .field("Moderator", format!("<@{}>", moderator.id), true);

    if let Some(e) = extra {
        embed = embed.field("Info", e, true);
    }

    embed = embed
        .field("Reason", reason, false)
        .timestamp(serenity::Timestamp::now());

    ch.widen()
        .send_message(&ctx.http, serenity::CreateMessage::new().embed(embed))
        .await
        .ok();
}
