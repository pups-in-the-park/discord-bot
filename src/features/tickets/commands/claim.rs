use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};
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

    ctx.data().db.claim_ticket(ticket.id, &ctx.author().id.to_string()).await?;

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .colour(colours::GREEN)
                .description(format!("✋ <@{}> has claimed this ticket.", ctx.author().id)),
        ),
    )
    .await?;
    Ok(())
}
