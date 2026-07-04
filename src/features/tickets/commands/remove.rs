use poise::serenity_prelude as serenity;

use crate::context::{Context, Error};

/// Remove a user from this ticket.
#[poise::command(slash_command, guild_only)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "User to remove"] user: serenity::User,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let channel_id = ctx.channel_id();

    let ticket = ctx
        .data()
        .db
        .get_ticket_by_thread(&channel_id.to_string())
        .await?
        .ok_or_else(|| Error::user("This command can only be used inside a ticket thread."))?;

    let is_staff =
        crate::permissions::is_mod_staff(ctx.serenity_context(), &ctx.data(), guild_id, ctx.author().id).await;
    let is_owner = ticket.owner_id == ctx.author().id.to_string();
    if !is_staff && !is_owner {
        return Err(Error::user("Only staff or the ticket owner can remove users."));
    }

    ctx.serenity_context()
        .http
        .remove_thread_channel_member(channel_id.expect_thread(), user.id)
        .await
        .map_err(|e| Error::user(format!("Failed to remove user: {}", e)))?;

    ctx.data().db.remove_member(ticket.id, &user.id.to_string()).await?;

    ctx.send(
        poise::CreateReply::default()
            .content(format!("➖ <@{}> has been removed from the ticket.", user.id)),
    )
    .await?;
    Ok(())
}
