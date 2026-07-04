use crate::context::{Context, Error};
use crate::permissions::require_mod_staff;

/// Release your claim on this ticket.
#[poise::command(slash_command, guild_only)]
pub async fn unclaim(ctx: Context<'_>) -> Result<(), Error> {
    require_mod_staff(&ctx).await?;
    let channel_id = ctx.channel_id();

    let ticket = ctx
        .data()
        .db
        .get_ticket_by_thread(&channel_id.to_string())
        .await?
        .ok_or_else(|| Error::user("This command can only be used inside a ticket thread."))?;

    if ticket.claimed_by.as_deref() != Some(&ctx.author().id.to_string()) {
        return Err(Error::user("You haven't claimed this ticket."));
    }

    ctx.data().db.unclaim_ticket(ticket.id).await?;

    ctx.send(poise::CreateReply::default().content("✋ Ticket unclaimed.")).await?;

    // Re-enable the Claim button on the in-thread ticket card.
    if let Ok(Some(fresh)) = ctx.data().db.get_ticket_by_id(ticket.id).await {
        super::super::service::refresh_ticket_card(&ctx.serenity_context().http, &ctx.data(), &fresh)
            .await
            .ok();
    }
    Ok(())
}
