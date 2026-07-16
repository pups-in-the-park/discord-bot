use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::features::moderation::service::{send_action_dm, ModActionDm};
use crate::features::moderation::view::{confirm_embed, log_action};
use crate::ids::parse_mod_timeout_modal;
use crate::util::{format_duration, modal_field, respond_ephemeral_modal_embed};

/// "Timeout User" context-menu modal submitted (`m:timeout:{target_id}`).
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let target_id = parse_mod_timeout_modal(&mi.data.custom_id).unwrap_or(0);
    let Some(guild_id) = mi.guild_id else {
        return Ok(());
    };
    let reason = modal_field(&mi.data.components, "reason")
        .unwrap_or("No reason given")
        .to_string();
    let secs: i64 = crate::util::modal_secs(&mi.data.components, "duration").unwrap_or(3600);

    let until = serenity::Timestamp::from_unix_timestamp(
        serenity::Timestamp::now().unix_timestamp() + secs,
    )
    .map_err(|_| anyhow::anyhow!("Invalid timestamp"))?;

    let until_str = until.to_rfc3339();

    // Fetch target/config before the timeout — failing after the mute lands
    // shows "interaction failed" and invites a duplicate retry.
    let target = serenity::UserId::new(target_id).to_user(ctx).await?;
    let mod_cfg = data.db.get_or_create_mod_config(&guild_id.to_string()).await?;

    guild_id
        .edit_member(
            &ctx.http,
            serenity::UserId::new(target_id),
            serenity::EditMember::new().disable_communication_until(until),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to time out user: {e}"))?;

    let infraction = data
        .db
        .create_infraction(
            &guild_id.to_string(),
            &target_id.to_string(),
            &mi.user.id.to_string(),
            "timeout",
            &reason,
            Some(secs),
            true,
            Some(until_str.as_str()),
        )
        .await?;

    if mod_cfg.dm_on_timeout {
        send_action_dm(
            &ctx.http,
            &target,
            guild_id,
            ModActionDm::Timeout { reason: &reason, until },
            Some((infraction.id, guild_id)),
        )
        .await;
    }

    let info = format!("{} · ends <t:{}:R>", format_duration(secs), until.unix_timestamp());
    log_action(
        &ctx.http,
        data,
        guild_id,
        "timeout",
        Some(infraction.id),
        &target,
        mi.user.id,
        &reason,
        Some(&info),
    )
    .await;

    respond_ephemeral_modal_embed(
        ctx,
        mi,
        confirm_embed(
            "timeout",
            Some(infraction.id),
            format!(
                "<@{}> is muted for {} — ends <t:{}:R>.\n**Reason:** {}",
                target_id,
                format_duration(secs),
                until.unix_timestamp(),
                reason
            ),
        ),
    )
    .await;
    Ok(())
}
