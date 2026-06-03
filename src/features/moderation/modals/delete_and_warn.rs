use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::features::moderation::service::send_action_dm;
use crate::ids::parse_mod_dw_modal;
use crate::util::modal_field;

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

    serenity::ChannelId::new(chan_id)
        .widen()
        .delete_message(&ctx.http, serenity::MessageId::new(msg_id), None)
        .await
        .ok();

    let target = serenity::UserId::new(author_id).to_user(ctx).await?;
    let infraction = data
        .db
        .create_infraction(
            &guild_id.to_string(),
            &author_id.to_string(),
            &mi.user.id.to_string(),
            "warn",
            &format!("[Message deleted] {}", reason),
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
            "⚠️ Warning (message deleted)",
            &reason,
            Some((infraction.id, guild_id)),
        )
        .await;
    }

    mi.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content(format!("🗑️ Message deleted and <@{}> warned.", author_id)),
        ),
    )
    .await?;
    Ok(())
}
