use poise::serenity_prelude as serenity;

use crate::context::{Context, Error};
use crate::ids::cid_mod_timeout_modal;
use crate::permissions::validate_target;

/// Timeout this user (staff only).
#[poise::command(context_menu_command = "Timeout User", guild_only, default_member_permissions = "MODERATE_MEMBERS")]
pub async fn timeout_user(ctx: Context<'_>, target: serenity::User) -> Result<(), Error> {
    validate_target(&ctx, &target).await?;

    crate::util::modal_response(
        ctx,
        serenity::CreateModal::new(cid_mod_timeout_modal(target.id.get()), "⏱️ Timeout User")
            .components(vec![
                serenity::CreateActionRow::InputText(
                    serenity::CreateInputText::new(
                        serenity::InputTextStyle::Short,
                        "Reason for timeout",
                        "reason",
                    )
                    .required(true)
                    .placeholder("What did they do?"),
                ),
                serenity::CreateActionRow::InputText(
                    serenity::CreateInputText::new(
                        serenity::InputTextStyle::Short,
                        "How long to mute",
                        "duration",
                    )
                    .required(true)
                    .placeholder("60s / 5min / 10min / 1h / 1d / 1w")
                    .value("1h"),
                ),
            ]),
    )
    .await?;
    Ok(())
}
