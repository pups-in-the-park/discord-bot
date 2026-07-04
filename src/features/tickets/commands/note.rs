use crate::context::{Context, Error};
use crate::permissions::require_mod_staff;

/// Add a private staff note to this ticket.
#[poise::command(slash_command, guild_only)]
pub async fn note(
    ctx: Context<'_>,
    #[description = "Note content"] content: String,
) -> Result<(), Error> {
    require_mod_staff(&ctx).await?;
    let channel_id = ctx.channel_id();

    let ticket = ctx
        .data()
        .db
        .get_ticket_by_thread(&channel_id.to_string())
        .await?
        .ok_or_else(|| Error::user("This command can only be used inside a ticket thread."))?;

    ctx.data().db.add_note(ticket.id, &ctx.author().id.to_string(), &content).await?;

    ctx.send(
        poise::CreateReply::default()
            .ephemeral(true)
            .content(format!("📝 **Staff note added.**\n{}", content)),
    )
    .await?;
    Ok(())
}
