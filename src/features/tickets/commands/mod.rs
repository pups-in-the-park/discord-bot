//! `/ticket` parent group. Leaf commands live one-per-file; `panel`, `category`,
//! and `tag` are cohesive subcommand groups in their own modules.

mod add;
mod category;
mod claim;
mod close;
mod info;
mod list;
mod note;
mod panel;
mod priority;
mod remove;
mod tag;
mod transfer;
mod unclaim;

use crate::context::{Context, Error};

/// Ticket management.
#[poise::command(
    slash_command,
    guild_only,
    subcommands(
        "category::category",
        "panel::panel",
        "close::close",
        "claim::claim",
        "unclaim::unclaim",
        "priority::priority",
        "tag::tag",
        "note::note",
        "transfer::transfer",
        "add::add",
        "remove::remove",
        "info::info",
        "list::list",
    )
)]
pub async fn ticket(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}
