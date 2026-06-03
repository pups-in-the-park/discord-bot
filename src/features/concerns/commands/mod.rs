//! `/concerns` admin command group.

mod stats;

use crate::context::{Context, Error};

/// Accountability concerns (admin only).
#[poise::command(
    slash_command,
    guild_only,
    subcommands("stats::stats"),
    subcommand_required,
    default_member_permissions = "MANAGE_GUILD"
)]
pub async fn concerns(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}
