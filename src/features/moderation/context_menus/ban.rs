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
                serenity::CreateActionRow::InputText(
                    serenity::CreateInputText::new(
                        serenity::InputTextStyle::Short,
                        "Reason for ban",
                        "reason",
                    )
                    .required(true)
                    .placeholder("Be specific about the violation"),
                ),
                serenity::CreateActionRow::InputText(
                    serenity::CreateInputText::new(
                        serenity::InputTextStyle::Short,
                        "Delete their messages",
                        "delete_messages",
                    )
                    .required(false)
                    .placeholder("none / 1h / 6h / 24h / 3d / 7d")
                    .value("none"),
                ),
                serenity::CreateActionRow::InputText(
                    serenity::CreateInputText::new(
                        serenity::InputTextStyle::Short,
                        "Allow appeals (yes/no)",
                        "appealable",
                    )
                    .required(false)
                    .placeholder("Can they appeal this ban later?")
                    .value("yes"),
                ),
            ],
        ),
    )
    .await?;
    Ok(())
}
