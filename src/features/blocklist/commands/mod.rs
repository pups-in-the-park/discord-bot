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

/// Suggest the feature scopes plus a `ticket:{name}` entry per category.
pub(crate) async fn autocomplete_scope<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> serenity::CreateAutocompleteResponse<'a> {
    let p = partial.to_lowercase();

    let mut choices: Vec<serenity::AutocompleteChoice> = super::view::FEATURE_SCOPES
        .iter()
        .filter(|s| s.contains(&p))
        .map(|s| serenity::AutocompleteChoice::new(*s, *s))
        .collect();

    if let Some(guild_id) = ctx.guild_id() {
        if let Ok(types) = ctx.data().db.get_ticket_types(&guild_id.to_string()).await {
            for t in types {
                let value = format!("ticket:{}", t.name);
                if p.is_empty()
                    || value.to_lowercase().contains(&p)
                    || t.label.to_lowercase().contains(&p)
                {
                    choices.push(serenity::AutocompleteChoice::new(
                        format!("{} (category)", t.label),
                        value,
                    ));
                }
            }
        }
    }

    serenity::CreateAutocompleteResponse::new().set_choices(choices)
}
