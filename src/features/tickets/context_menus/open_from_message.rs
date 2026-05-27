use poise::serenity_prelude as serenity;

use crate::context::{Context, Error};

/// Open a ticket for the author of this message (staff only).
#[poise::command(context_menu_command = "Open Ticket From Message", guild_only, default_member_permissions = "MODERATE_MEMBERS")]
pub async fn open_ticket_from_message(ctx: Context<'_>, msg: serenity::Message) -> Result<(), Error> {
    if msg.author.bot {
        return Err(Error::user("You cannot open a ticket for a bot."));
    }
    super::open_ticket_for_user(ctx, msg.author.id).await
}
