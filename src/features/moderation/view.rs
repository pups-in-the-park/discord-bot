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
        "untimeout" => ("✅", "Timeout Removed", colours::GREEN),
        "unban" => ("🔓", "Member Unbanned", colours::GREEN),
        _ => ("•", "Moderation Action", colours::ORANGE),
    }
}

/// Ephemeral confirmation embed shown to the acting moderator, shared by slash
/// commands and context-menu modal handlers so both surfaces read identically.
/// `kind` reuses [`action_style`] for emoji/colour; `case_id` appends "· Case #N"
/// so the moderator can find the matching mod-log entry.
pub fn confirm_embed(
    kind: &str,
    case_id: Option<i64>,
    description: String,
) -> serenity::CreateEmbed<'static> {
    let (emoji, _, colour) = action_style(kind);
    let title = match kind {
        "warn" => "Warning issued",
        "timeout" => "Timeout applied",
        "untimeout" => "Timeout lifted",
        "kick" => "Member kicked",
        "ban" => "Member banned",
        "unban" => "Ban lifted",
        _ => "Action taken",
    };
    let title = match case_id {
        Some(id) => format!("{emoji} {title} · Case #{id}"),
        None => format!("{emoji} {title}"),
    };
    serenity::CreateEmbed::new().colour(colour).title(title).description(description)
}

/// Infraction-history embed for a user, shared by `/history` and the
/// "View History" context menu so the two surfaces stay identical. Shows the
/// most recent 15 entries (the [`crate::util::format_infraction_history`] cap).
pub fn history_embed(
    name: &str,
    infractions: &[crate::db::Infraction],
) -> serenity::CreateEmbed<'static> {
    let title = format!("📋 History — {}", name);
    if infractions.is_empty() {
        return serenity::CreateEmbed::new()
            .colour(colours::GREY)
            .title(title)
            .description("No infractions on record — clean slate.");
    }
    let n = infractions.len();
    let footer = if n > 15 {
        format!("{} infractions on record · showing most recent 15", n)
    } else {
        format!("{} infraction{} on record", n, if n == 1 { "" } else { "s" })
    };
    serenity::CreateEmbed::new()
        .colour(colours::ORANGE)
        .title(title)
        .description(crate::util::format_infraction_history(infractions).join("\n\n"))
        .footer(serenity::CreateEmbedFooter::new(footer))
}

/// Post a structured moderation-action embed to the mod-log channel, if configured.
/// `kind` drives the title + per-action colour; `case_id` is the infraction id;
/// `extra` carries action-specific detail (duration/expiry, appealable, jump link).
#[allow(clippy::too_many_arguments)]
pub async fn log_action(
    http: &serenity::Http,
    data: &Arc<BotData>,
    guild_id: serenity::GuildId,
    kind: &str,
    case_id: Option<i64>,
    target: &serenity::User,
    moderator_id: serenity::UserId,
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
        .field("Moderator", format!("<@{}>", moderator_id), true);

    if let Some(e) = extra {
        embed = embed.field("Info", e, true);
    }

    embed = embed
        .field("Reason", reason, false)
        .timestamp(serenity::Timestamp::now());

    ch.widen()
        .send_message(http, serenity::CreateMessage::new().embed(embed))
        .await
        .ok();
}
