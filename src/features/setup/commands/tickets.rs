use super::super::view::build_setup_ticket_form;
use crate::context::{Context, Error};
use crate::ui::slash_respond;

/// Configure the ticket parent channel and reports channel.
#[poise::command(slash_command, guild_only, rename = "tickets")]
pub async fn tickets(ctx: Context<'_>) -> Result<(), Error> {
    let g = ctx.guild_id().unwrap().to_string();
    let cfg = ctx.data().db.get_or_create_guild(&g).await?;
    slash_respond(ctx, &build_setup_ticket_form(&cfg)).await?;
    Ok(())
}
