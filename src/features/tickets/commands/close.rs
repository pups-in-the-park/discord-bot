use poise::serenity_prelude as serenity;

use crate::context::{Context, Error};
use crate::ids::{cid_close_modal, CLOSE_REASON_FIELD};

use super::super::service::execute_close;

/// Close the current ticket (must be used inside a ticket thread).
#[poise::command(slash_command, guild_only)]
pub async fn close(
    ctx: Context<'_>,
    #[description = "Reason for closing"] reason: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let channel_id = ctx.channel_id();

    let ticket = ctx
        .data()
        .db
        .get_ticket_by_thread(&channel_id.to_string())
        .await?
        .ok_or_else(|| Error::user("This command can only be used inside a ticket thread."))?;

    if ticket.status == crate::db::TicketStatus::Closed {
        return Err(Error::user("This ticket is already closed."));
    }

    let is_staff =
        crate::permissions::is_mod_staff(ctx.serenity_context(), ctx.data(), guild_id, ctx.author().id).await;

    let is_owner = ticket.owner_id == ctx.author().id.to_string();
    if !is_staff && !is_owner {
        return Err(Error::user("Only staff or the ticket owner can close this ticket."));
    }

    if let Some(reason) = reason {
        execute_close(&ctx.serenity_context().http, ctx.data(), &ticket, ctx.author().id, Some(&reason)).await?;
    } else {
        // Open close-reason modal
        crate::util::modal_response(
            ctx,
            serenity::CreateModal::new(cid_close_modal(ticket.id), "🔒 Close Ticket").components(vec![
                serenity::CreateActionRow::InputText(
                    serenity::CreateInputText::new(
                        serenity::InputTextStyle::Paragraph,
                        "Why are you closing this?",
                        CLOSE_REASON_FIELD,
                    )
                    .required(false)
                    .placeholder("e.g. Issue resolved, user no longer responsive"),
                ),
            ]),
        )
        .await?;
    }

    Ok(())
}
