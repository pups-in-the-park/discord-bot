use poise::serenity_prelude as serenity;

use crate::context::{Context, Error};
use crate::ids::{cid_report_user_modal, REPORT_REASON_FIELD};

/// Report this user to the moderation team.
#[poise::command(context_menu_command = "Report User", guild_only)]
pub async fn report_user(ctx: Context<'_>, target: serenity::User) -> Result<(), Error> {
    if target.bot() {
        return Err(Error::user("You can't report a bot."));
    }
    if target.id == ctx.author().id {
        return Err(Error::user("You can't report yourself."));
    }

    let modal_id = cid_report_user_modal(target.id.get());
    crate::util::modal_response(
        ctx,
        serenity::CreateModal::new(modal_id, format!("🚨 Report {}", target.name)).components(
            vec![crate::util::modal_input(
                "Why are you reporting this user?",
                REPORT_REASON_FIELD,
                true,
                false,
                Some("Describe specific behaviors or violations…"),
                None,
            )],
        ),
    )
    .await?;
    Ok(())
}
