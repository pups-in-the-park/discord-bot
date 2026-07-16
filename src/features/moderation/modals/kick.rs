use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::features::moderation::service::{send_action_dm, ModActionDm};
use crate::features::moderation::view::{confirm_embed, log_action};
use crate::ids::parse_mod_kick_modal;
use crate::util::{modal_field, respond_ephemeral_modal_embed};

/// "Kick User" context-menu modal submitted (`m:kick:{target_id}`).
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let target_id = parse_mod_kick_modal(&mi.data.custom_id).unwrap_or(0);
    let Some(guild_id) = mi.guild_id else {
        return Ok(());
    };
    let reason = modal_field(&mi.data.components, "reason")
        .unwrap_or("No reason given")
        .to_string();

    let target = serenity::UserId::new(target_id).to_user(ctx).await?;
    let mod_cfg = data.db.get_or_create_mod_config(&guild_id.to_string()).await?;

    // DM before the kick — once they're out of the guild we can't DM them (50007).
    if mod_cfg.dm_on_kick {
        send_action_dm(&ctx.http, &target, guild_id, ModActionDm::Kick { reason: &reason }, None)
            .await;
    }

    guild_id
        .kick(&ctx.http, serenity::UserId::new(target_id), Some(&reason))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to kick user: {e}"))?;

    let infraction = data
        .db
        .create_infraction(
            &guild_id.to_string(),
            &target_id.to_string(),
            &mi.user.id.to_string(),
            "kick",
            &reason,
            None,
            false,
            None,
        )
        .await?;

    log_action(
        &ctx.http,
        data,
        guild_id,
        "kick",
        Some(infraction.id),
        &target,
        mi.user.id,
        &reason,
        None,
    )
    .await;

    respond_ephemeral_modal_embed(
        ctx,
        mi,
        confirm_embed(
            "kick",
            Some(infraction.id),
            format!(
                "<@{}> has been removed from the server. They can rejoin with an invite.\n**Reason:** {}",
                target_id, reason
            ),
        ),
    )
    .await;
    Ok(())
}
