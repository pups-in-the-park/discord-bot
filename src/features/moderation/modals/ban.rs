use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::features::moderation::service::{send_action_dm, ModActionDm};
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
        crate::util::modal_secs(&mi.data.components, "delete_messages").unwrap_or(0) as u32;
    let appealable = crate::util::modal_checked(&mi.data.components, "appealable");
    // 0 = permanent; otherwise a temporary ban that the expiry task lifts.
    let duration_secs = crate::util::modal_secs(&mi.data.components, "duration").unwrap_or(0);
    let (dur, expires_at, until_ts) = crate::features::moderation::service::ban_expiry(duration_secs);

    guild_id
        .ban(
            &ctx.http,
            serenity::UserId::new(target_id),
            delete_secs,
            Some(&reason),
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
            dur,
            appealable,
            expires_at.as_deref(),
        )
        .await?;

    let target = serenity::UserId::new(target_id).to_user(ctx).await?;
    let mod_cfg = data.db.get_or_create_mod_config(&guild_id.to_string()).await?;
    if mod_cfg.dm_on_ban {
        let appeal_info = if appealable { Some((infraction.id, guild_id)) } else { None };
        send_action_dm(&ctx.http, &target, guild_id, ModActionDm::Ban { reason: &reason, until: until_ts }, appeal_info)
            .await;
    }

    let suffix = until_ts.map(|ts| format!(" Lifts <t:{}:R>.", ts)).unwrap_or_default();
    mi.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content(format!("🔨 <@{}> banned.{}", target_id, suffix)),
        ),
    )
    .await?;
    Ok(())
}
