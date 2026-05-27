use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::{parse_close_modal, CLOSE_REASON_FIELD};
use crate::util::modal_field;

use super::super::service::execute_close;

/// Close-reason modal submitted (`m:close:{ticket_id}`): close the ticket.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let ticket_id =
        parse_close_modal(&mi.data.custom_id).ok_or_else(|| anyhow::anyhow!("Invalid ticket ID"))?;

    let reason = modal_field(&mi.data.components, CLOSE_REASON_FIELD).filter(|s| !s.is_empty());

    if let Some(ticket) = data.db.get_ticket_by_id(ticket_id).await? {
        execute_close(&ctx.http, data, &ticket, mi.user.id, reason).await?;
    }

    mi.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge).await?;
    Ok(())
}
