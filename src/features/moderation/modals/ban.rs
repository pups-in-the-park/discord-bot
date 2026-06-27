use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::features::moderation::service::{send_action_dm, ModActionDm};
use crate::features::moderation::view::{confirm_embed, log_action};
use crate::ids::parse_mod_ban_modal;
use crate::util::{modal_field, respond_ephemeral_modal_embed};

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

    // Fetch the target and config *before* the irreversible ban: a transient
    // failure here should abort harmlessly, not leave the interaction unanswered
    // after the user is already banned (which would prompt a duplicate retry).
    let target = serenity::UserId::new(target_id).to_user(ctx).await?;
    let mod_cfg = data.db.get_or_create_mod_config(&guild_id.to_string()).await?;

    // A new ban supersedes any active one — deactivate older ban infractions
    // first, or an earlier temp ban's expiry would lift this ban.
    data.db
        .deactivate_active_bans(&guild_id.to_string(), &target_id.to_string())
        .await
        .ok();
    // Record and DM *before* the ban: once the member is removed they share no
    // guild with the bot, so the ban/appeal notice can no longer be delivered
    // (mirrors the kick handler). If the ban itself fails below, the handler
    // returns the error and the dispatcher reports it to the moderator rather
    // than falsely confirming a ban that never happened.
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

    if mod_cfg.dm_on_ban {
        let appeal_info = if appealable { Some((infraction.id, guild_id)) } else { None };
        send_action_dm(&ctx.http, &target, guild_id, ModActionDm::Ban { reason: &reason, until: until_ts }, appeal_info)
            .await;
    }

    guild_id
        .ban(
            &ctx.http,
            serenity::UserId::new(target_id),
            delete_secs,
            Some(&reason),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to ban user: {e}"))?;

    let length = match until_ts {
        Some(ts) => format!("Temporary — lifts <t:{}:R>", ts),
        None => "Permanent".to_string(),
    };
    log_action(
        &ctx.http,
        data,
        guild_id,
        "ban",
        Some(infraction.id),
        &target,
        mi.user.id,
        &reason,
        Some(&length),
    )
    .await;

    let length_phrase = match until_ts {
        Some(ts) => format!("until <t:{ts}:F> (<t:{ts}:R>)"),
        None => "permanently".to_string(),
    };
    respond_ephemeral_modal_embed(
        ctx,
        mi,
        confirm_embed(
            "ban",
            Some(infraction.id),
            format!(
                "<@{}> is banned {}.\n**Reason:** {}\n**Appeals:** {}",
                target_id,
                length_phrase,
                reason,
                if appealable { "Allowed" } else { "Not allowed" }
            ),
        ),
    )
    .await;
    Ok(())
}
