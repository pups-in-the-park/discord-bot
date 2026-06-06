use poise::serenity_prelude as serenity;

use crate::context::{Context, Error};
use crate::ids::cid_mod_ban_modal;
use crate::permissions::validate_target;

/// Ban this user from the server (staff only).
#[poise::command(context_menu_command = "Ban User", guild_only, default_member_permissions = "BAN_MEMBERS")]
pub async fn ban_user(ctx: Context<'_>, target: serenity::User) -> Result<(), Error> {
    validate_target(&ctx, &target).await?;

    crate::util::modal_response(
        ctx,
        serenity::CreateModal::new(cid_mod_ban_modal(target.id.get()), "🔨 Ban User").components(
            vec![
                crate::util::modal_input(
                    "Reason for ban",
                    "reason",
                    false,
                    true,
                    Some("Be specific about the violation"),
                    None,
                ),
                crate::util::ban_duration_select("Ban length", "duration", 0),
                crate::util::delete_window_select("Delete their messages", "delete_messages", 0),
                crate::util::single_checkbox("Appeals", "appealable", "Allow them to appeal", true),
            ],
        ),
    )
    .await?;
    Ok(())
}
