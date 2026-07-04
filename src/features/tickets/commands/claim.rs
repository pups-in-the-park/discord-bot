use crate::context::{Context, Error};
use crate::permissions::require_mod_staff;

/// Claim this ticket.
#[poise::command(slash_command, guild_only)]
pub async fn claim(ctx: Context<'_>) -> Result<(), Error> {
    require_mod_staff(&ctx).await?;
    let channel_id = ctx.channel_id();

    let ticket = ctx
        .data()
        .db
        .get_ticket_by_thread(&channel_id.to_string())
        .await?
        .ok_or_else(|| Error::user("This command can only be used inside a ticket thread."))?;

    // Don't let a second claim silently overwrite the first — mirror the card's
    // Claim button, which refuses an already-claimed ticket.
    if let Some(ref claimer) = ticket.claimed_by {
        return Err(Error::user(format!(
            "This ticket is already claimed by <@{}>. They can `/ticket unclaim` it first.",
            claimer
        )));
    }

    if !ctx.data().db.claim_ticket(ticket.id, &ctx.author().id.to_string()).await? {
        return Err(Error::user("This ticket was just claimed by someone else."));
    }

    // Plain text to match the Claim button's identical announcement (which posts
    // a plain follow-up) — same event, same paradigm.
    ctx.send(
        poise::CreateReply::default()
            .content(format!("✋ <@{}> has claimed this ticket.", ctx.author().id)),
    )
    .await?;

    // Reflect the claim on the in-thread ticket card (after responding, to stay
    // within the 3s window).
    if let Ok(Some(fresh)) = ctx.data().db.get_ticket_by_id(ticket.id).await {
        super::super::service::refresh_ticket_card(&ctx.serenity_context().http, &ctx.data(), &fresh)
            .await
            .ok();
    }
    Ok(())
}
