//! `/blocklist` parent group; leaf subcommands live one-per-file. The scope
//! autocomplete is shared by `add` and `remove`, so it lives here.

mod add;
mod list;
mod remove;

pub use add::add;
pub use list::list;
pub use remove::remove;

use poise::serenity_prelude as serenity;

use crate::context::{Context, Error};

/// Manage the ticket blocklist.
#[poise::command(slash_command, guild_only, subcommands("add", "remove", "list"))]
pub async fn blocklist(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Suggest "global" plus the guild's category names.
pub(crate) async fn autocomplete_scope(
    ctx: Context<'_>,
    partial: &str,
) -> Vec<serenity::AutocompleteChoice> {
    let Some(guild_id) = ctx.guild_id() else {
        return vec![];
    };
    let partial_lower = partial.to_lowercase();

    let mut choices = vec![serenity::AutocompleteChoice::new("global", "global")];

    if let Ok(types) = ctx.data().db.get_ticket_types(&guild_id.to_string()).await {
        for t in types {
            if partial_lower.is_empty()
                || t.name.to_lowercase().contains(&partial_lower)
                || t.label.to_lowercase().contains(&partial_lower)
            {
                choices.push(serenity::AutocompleteChoice::new(t.label, t.name));
            }
        }
    }

    choices
}
