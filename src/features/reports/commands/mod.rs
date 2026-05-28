//! `/report` command group. The leaf subcommands live one-per-file; this module
//! only assembles them into the parent group.

mod message;
mod user;

pub use message::message;
pub use user::user;

use crate::context::{Context, Error};

/// Submit a report.
#[poise::command(slash_command, guild_only, subcommands("user", "message"))]
pub async fn report(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}
