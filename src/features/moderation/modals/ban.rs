use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
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

    // Fetch target/config before the ban — failing after it lands shows
    // "interaction failed" and invites a duplicate retry.
    let target = serenity::UserId::new(target_id).to_user(ctx).await?;
    let mod_cfg = data.db.get_or_create_mod_config(&guild_id.to_string()).await?;

    let infraction = crate::features::moderation::service::apply_ban(
        &ctx.http,
        data,
        guild_id,
        &target,
        mi.user.id,
        &reason,
        delete_secs,
        dur,
        expires_at.as_deref(),
        until_ts,
        appealable,
        mod_cfg.dm_on_ban,
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
