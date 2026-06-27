use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::features::moderation::service::{send_action_dm, ModActionDm};
use crate::features::moderation::view::{confirm_embed, log_action};
use crate::ids::parse_mod_dw_modal;
use crate::util::{modal_field, respond_ephemeral_modal_embed};

/// "Delete & Warn" context-menu modal submitted (`m:dw:{msg_id}:{chan_id}:{author_id}`).
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let (msg_id, chan_id, author_id) =
        parse_mod_dw_modal(&mi.data.custom_id).unwrap_or((0, 0, 0));
    let Some(guild_id) = mi.guild_id else {
        return Ok(());
    };
    let reason = modal_field(&mi.data.components, "reason")
        .unwrap_or("No reason given")
        .to_string();

    // The warn should still land even if the message is already gone, so this is
    // best-effort — but we track whether it actually happened so the reason, log,
    // and confirmation don't claim a deletion that never occurred.
    let deleted = serenity::ChannelId::new(chan_id)
        .widen()
        .delete_message(&ctx.http, serenity::MessageId::new(msg_id), None)
        .await
        .is_ok();

    let target = serenity::UserId::new(author_id).to_user(ctx).await?;
    let infraction = data
        .db
        .create_infraction(
            &guild_id.to_string(),
            &author_id.to_string(),
            &mi.user.id.to_string(),
            "warn",
            &if deleted { format!("[Message deleted] {}", reason) } else { reason.clone() },
            None,
            true,
            None,
        )
        .await?;

    let mod_cfg = data.db.get_or_create_mod_config(&guild_id.to_string()).await?;
    if mod_cfg.dm_on_warn {
        send_action_dm(
            &ctx.http,
            &target,
            guild_id,
            ModActionDm::Warn { reason: &reason },
            Some((infraction.id, guild_id)),
        )
        .await;
    }

    log_action(
        &ctx.http,
        data,
        guild_id,
        "warn",
        Some(infraction.id),
        &target,
        mi.user.id,
        &reason,
        Some(if deleted { "Message deleted" } else { "Message could not be deleted" }),
    )
    .await;

    let outcome = if deleted {
        format!("Message deleted and <@{}> warned.", author_id)
    } else {
        format!(
            "<@{}> warned. The message could not be deleted — it may already be gone.",
            author_id
        )
    };
    respond_ephemeral_modal_embed(
        ctx,
        mi,
        confirm_embed(
            "warn",
            Some(infraction.id),
            format!("{}\n**Reason:** {}", outcome, reason),
        ),
    )
    .await;
    Ok(())
}
