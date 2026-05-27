use poise::serenity_prelude as serenity;

use crate::context::{Context, Error};

/// Open a ticket for this user (staff only).
#[poise::command(context_menu_command = "Open Ticket With User", guild_only, default_member_permissions = "MODERATE_MEMBERS")]
pub async fn open_ticket_with_user(ctx: Context<'_>, target: serenity::User) -> Result<(), Error> {
    if target.bot {
        return Err(Error::user("You cannot open a ticket for a bot."));
    }
    super::open_ticket_for_user(ctx, target.id).await
}
