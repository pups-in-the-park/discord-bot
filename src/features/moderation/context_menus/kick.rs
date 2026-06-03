use poise::serenity_prelude as serenity;

use crate::context::{Context, Error};
use crate::ids::cid_mod_kick_modal;
use crate::permissions::validate_target;

/// Kick this user from the server (staff only).
#[poise::command(context_menu_command = "Kick User", guild_only, default_member_permissions = "KICK_MEMBERS")]
pub async fn kick_user(ctx: Context<'_>, target: serenity::User) -> Result<(), Error> {
    validate_target(&ctx, &target).await?;

    crate::util::modal_response(
        ctx,
        serenity::CreateModal::new(cid_mod_kick_modal(target.id.get()), "👢 Kick User").components(
            vec![crate::util::modal_input(
                "Reason for kick",
                "reason",
                false,
                true,
                Some("This user can rejoin later"),
                None,
            )],
        ),
    )
    .await?;
    Ok(())
}
