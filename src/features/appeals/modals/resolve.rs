use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::{parse_appeal_resolve_modal, APPEAL_RESPONSE_FIELD};
use crate::util::modal_field;

/// The appeal-resolve modal was submitted (`m:apr:{a|d}:{appeal_id}`). Resolves the
/// appeal through the shared service (card, thread, unban, DM) and confirms to staff.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let (appeal_id, accept) = parse_appeal_resolve_modal(&mi.data.custom_id)
        .ok_or_else(|| anyhow::anyhow!("Invalid appeal resolve modal id"))?;

    let Some(guild_id) = mi.guild_id else {
        return Ok(());
    };
    if !crate::permissions::is_mod_staff(ctx, data, guild_id, mi.user.id).await {
        ephemeral_reply(ctx, mi, "Only staff can resolve appeals.").await?;
        return Ok(());
    }

    let response = modal_field(&mi.data.components, APPEAL_RESPONSE_FIELD)
        .unwrap_or("")
        .trim()
        .to_string();
    if response.is_empty() {
        ephemeral_reply(ctx, mi, "Please provide a response to send to the user.").await?;
        return Ok(());
    }

    let appeal = data
        .db
        .get_appeal_by_id(appeal_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Appeal not found"))?;
    if appeal.status != "pending" {
        ephemeral_reply(ctx, mi, "This appeal has already been resolved.").await?;
        return Ok(());
    }

    // Unbanning, DMing, editing the card, and archiving the thread exceed the
    // 3-second window — acknowledge first, confirm via follow-up.
    mi.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Defer(
            serenity::CreateInteractionResponseMessage::new().ephemeral(true),
        ),
    )
    .await?;

    super::super::service::resolve(ctx, data, &appeal, accept, &response, mi.user.id).await?;

    mi.create_followup(
        &ctx.http,
        serenity::CreateInteractionResponseFollowup::new().ephemeral(true).content(if accept {
            "✅ Appeal accepted. Response sent to the user and thread archived."
        } else {
            "❌ Appeal denied. Response sent to the user and thread archived."
        }),
    )
    .await?;
    Ok(())
}

/// Reply to a modal submission with a short ephemeral message.
async fn ephemeral_reply(
    ctx: &serenity::Context,
    mi: &serenity::ModalInteraction,
    msg: &str,
) -> Result<(), anyhow::Error> {
    mi.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content(msg),
        ),
    )
    .await?;
    Ok(())
}
