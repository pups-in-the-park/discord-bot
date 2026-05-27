use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};
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

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .colour(colours::GREY)
                .description("✋ Ticket unclaimed."),
        ),
    )
    .await?;
    Ok(())
}
