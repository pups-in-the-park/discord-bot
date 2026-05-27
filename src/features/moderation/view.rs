//! Moderation log embed posted to the configured mod-log channel.

use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::{colours, BotData};

/// Post a moderation-action embed to the mod-log channel, if one is configured.
#[allow(clippy::too_many_arguments)]
pub async fn log_action(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    guild_id: serenity::GuildId,
    title: &str,
    target: &serenity::User,
    moderator: &serenity::User,
    reason: &str,
    extra: Option<&str>,
) {
    let Some(ch) = crate::events::get_mod_log(data, guild_id).await else {
        return;
    };

    let mut embed = serenity::CreateEmbed::new()
        .colour(colours::ORANGE)
        .title(title)
        .field("User", format!("<@{}> ({})", target.id, target.name), true)
        .field("Moderator", format!("<@{}>", moderator.id), true)
        .field("Reason", reason, false)
        .timestamp(serenity::Timestamp::now());

    if let Some(e) = extra {
        embed = embed.field("Info", e, true);
    }

    ch.send_message(ctx, serenity::CreateMessage::new().embed(embed))
        .await
        .ok();
}
