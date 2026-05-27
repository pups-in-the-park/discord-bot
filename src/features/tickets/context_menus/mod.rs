//! Context-menu entry points for opening tickets, plus the shared category-picker
//! helper they both use.

mod open_from_message;
mod open_with_user;

pub use open_from_message::open_ticket_from_message;
pub use open_with_user::open_ticket_with_user;

use poise::serenity_prelude as serenity;

use crate::context::{Context, Error};
use crate::ids::{cid_ctx_open_modal, cid_ctx_type_select};

use super::view::build_open_modal;

/// Show a category selector, or open the form modal directly if there's only one.
pub(crate) async fn open_ticket_for_user(
    ctx: Context<'_>,
    target_id: serenity::UserId,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let types = ctx.data().db.get_ticket_types(&guild_id.to_string()).await?;
    let active: Vec<_> = types.into_iter().filter(|t| t.active).collect();

    if active.is_empty() {
        return Err(Error::user("No ticket categories are configured."));
    }

    if active.len() == 1 {
        // Skip the selector and open the form modal directly.
        let modal_id = cid_ctx_open_modal(active[0].id, target_id.get());
        let form_fields = ctx.data().db.get_form_fields(active[0].id).await?;
        let modal = build_open_modal(modal_id, &active[0], &form_fields);
        crate::util::modal_response(ctx, modal).await?;
    } else {
        // Send an ephemeral select-menu so staff can pick the category.
        let options: Vec<serenity::CreateSelectMenuOption> = active
            .iter()
            .map(|t| {
                serenity::CreateSelectMenuOption::new(&t.label, t.id.to_string()).description(
                    t.description.as_deref().unwrap_or("").chars().take(100).collect::<String>(),
                )
            })
            .collect();

        ctx.send(
            poise::CreateReply::default()
                .ephemeral(true)
                .content(format!("Open a ticket for <@{}>. Select a category:", target_id))
                .components(vec![serenity::CreateActionRow::SelectMenu(
                    serenity::CreateSelectMenu::new(
                        cid_ctx_type_select(target_id.get()),
                        serenity::CreateSelectMenuKind::String { options },
                    )
                    .placeholder("Choose a category…"),
                )]),
        )
        .await?;
    }
    Ok(())
}
