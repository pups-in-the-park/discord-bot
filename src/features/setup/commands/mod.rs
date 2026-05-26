//! `/setup` parent group. Each subcommand opens its settings form as an ephemeral
//! CV2 message; selects/toggles save immediately, numeric fields open a small modal.

mod appeals;
mod logs;
mod mod_staff;
mod raid;
mod slowmode;
mod tickets;

pub use appeals::appeals;
pub use logs::logs;
pub use mod_staff::mod_staff;
pub use raid::raid;
pub use slowmode::slowmode;
pub use tickets::tickets;

use crate::context::{Context, Error};

/// Configure bot settings for this server.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MANAGE_GUILD",
    subcommands("logs", "tickets", "mod_staff", "appeals", "raid", "slowmode")
)]
pub async fn setup(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}
