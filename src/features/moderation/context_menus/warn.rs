use poise::serenity_prelude as serenity;

use crate::context::{Context, Error};
use crate::ids::cid_mod_warn_modal;
use crate::permissions::validate_target;

/// Issue a warning to this user (staff only).
#[poise::command(context_menu_command = "Warn User", guild_only, default_member_permissions = "MODERATE_MEMBERS")]
pub async fn warn_user(ctx: Context<'_>, target: serenity::User) -> Result<(), Error> {
    validate_target(&ctx, &target).await?;

    crate::util::modal_response(
        ctx,
        serenity::CreateModal::new(cid_mod_warn_modal(target.id.get()), "⚠️ Warn User").components(
            vec![serenity::CreateActionRow::InputText(
                serenity::CreateInputText::new(
                    serenity::InputTextStyle::Short,
                    "Reason for warning",
                    "reason",
                )
                .required(true)
                .placeholder("e.g. Violation of rule #3 (respect)"),
            )],
        ),
    )
    .await?;
    Ok(())
}
