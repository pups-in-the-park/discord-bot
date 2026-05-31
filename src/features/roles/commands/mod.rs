//! `/role` parent group; leaf subcommands live one-per-file.

mod give;
mod remove;

pub use give::give;
pub use remove::remove;

use crate::context::{Context, Error};

/// Manage server roles.
#[poise::command(slash_command, guild_only, subcommands("give", "remove"))]
pub async fn role(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}
