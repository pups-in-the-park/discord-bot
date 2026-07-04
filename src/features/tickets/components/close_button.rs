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
    let is_owner = ticket.owner_id == ci.user.id.to_string();
    let allowed = match ci.guild_id {
        Some(gid) => is_owner || crate::permissions::is_mod_staff(ctx, data, gid, ci.user.id).await,
        None => is_owner,
    };
    if !allowed {
        crate::util::respond_ephemeral(
            ctx,
            ci,
            "Only staff or the ticket owner can close this ticket.",
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
