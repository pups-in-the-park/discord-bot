use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::features::moderation::service::send_action_dm;
use crate::ids::parse_mod_ban_modal;
use crate::util::modal_field;

/// "Ban User" context-menu modal submitted (`m:ban:{target_id}`).
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let target_id = parse_mod_ban_modal(&mi.data.custom_id).unwrap_or(0);
    let Some(guild_id) = mi.guild_id else {
        return Ok(());
    };
    let reason = modal_field(&mi.data.components, "reason")
        .unwrap_or("No reason given")
        .to_string();
    let delete_secs =
        parse_delete_messages(modal_field(&mi.data.components, "delete_messages").unwrap_or("none"));
    let appealable = modal_field(&mi.data.components, "appealable")
        .map(|s| s.trim().to_lowercase() != "no")
        .unwrap_or(true);

    guild_id
        .ban_with_reason(
            ctx,
            serenity::UserId::new(target_id),
            (delete_secs / 86400) as u8,
            &reason,
        )
        .await
        .ok();
    let infraction = data
        .db
        .create_infraction(
            &guild_id.to_string(),
            &target_id.to_string(),
            &mi.user.id.to_string(),
            "ban",
            &reason,
            None,
            appealable,
            None,
        )
        .await?;

    let target = serenity::UserId::new(target_id).to_user(ctx).await?;
    let mod_cfg = data.db.get_or_create_mod_config(&guild_id.to_string()).await?;
    if mod_cfg.dm_on_ban {
        let appeal_info = if appealable { Some((infraction.id, guild_id)) } else { None };
        send_action_dm(&ctx.http, &target, guild_id, "🔨 Banned", &reason, appeal_info).await;
    }

    mi.create_response(
        ctx,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content(format!("🔨 <@{}> banned.", target_id)),
        ),
    )
    .await?;
    Ok(())
}

/// Parse a human-readable "delete messages" value into seconds.
fn parse_delete_messages(s: &str) -> u32 {
    match s.trim().to_lowercase().as_str() {
        "1h" | "hour" => 3600,
        "6h" => 21600,
        "24h" | "day" => 86400,
        "3d" => 259200,
        "7d" | "week" => 604800,
        _ => 0,
    }
}
