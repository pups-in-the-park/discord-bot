use poise::serenity_prelude as serenity;

use crate::context::{Context, Error};
use crate::ids::cid_mod_dw_modal;
use crate::permissions::validate_target;

/// Delete this message and warn the author (staff only).
#[poise::command(context_menu_command = "Delete & Warn", guild_only, default_member_permissions = "MODERATE_MEMBERS")]
pub async fn delete_and_warn(ctx: Context<'_>, msg: serenity::Message) -> Result<(), Error> {
    validate_target(&ctx, &msg.author).await?;

    let modal_id = cid_mod_dw_modal(msg.id.get(), msg.channel_id.get(), msg.author.id.get());
    crate::util::modal_response(
        ctx,
        serenity::CreateModal::new(modal_id, "⚠️ Delete & Warn").components(vec![
            crate::util::modal_input(
                "What rule was broken?",
                "reason",
                false,
                true,
                Some("e.g. Spam, inappropriate language, harassment"),
                None,
            ),
        ]),
    )
    .await?;
    Ok(())
}
