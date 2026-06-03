use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::{cid_close_modal, CLOSE_REASON_FIELD};

/// "Close" button on a ticket card: open the close-reason modal.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
) -> Result<(), anyhow::Error> {
    let ticket = data
        .db
        .get_ticket_by_thread(&ci.channel_id.to_string())
        .await?
        .ok_or_else(|| anyhow::anyhow!("Not a ticket thread"))?;

    ci.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Modal(
            serenity::CreateModal::new(cid_close_modal(ticket.id), "🔒 Close Ticket").components(
                vec![crate::util::modal_input(
                    "Why are you closing this?",
                    CLOSE_REASON_FIELD,
                    true,
                    false,
                    Some("e.g. Issue resolved, user no longer responsive"),
                    None,
                )],
            ),
        ),
    )
    .await?;
    Ok(())
}
