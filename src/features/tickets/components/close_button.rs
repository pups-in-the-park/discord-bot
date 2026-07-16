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

    // Pre-check so an unauthorized user gets a clear message instead of a modal
    // that fails on submit. The modal handler re-checks authoritatively.
    if !super::super::service::user_may_close(ctx, data, ci.guild_id, &ticket, ci.user.id).await {
        crate::util::respond_ephemeral(
            ctx,
            ci,
            super::super::service::close_denied_message(&ticket),
        )
        .await;
        return Ok(());
    }

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
