use poise::serenity_prelude as serenity;

use crate::context::{Context, Error};
use crate::ids::cid_report_user_modal;

/// Report this user to the moderation team.
#[poise::command(context_menu_command = "Report User", guild_only)]
pub async fn report_user(ctx: Context<'_>, target: serenity::User) -> Result<(), Error> {
    if target.bot() {
        return Err(Error::user("You can't report a bot."));
    }
    if target.id == ctx.author().id {
        return Err(Error::user("You can't report yourself."));
    }

    crate::util::modal_response(
        ctx,
        super::super::view::build_report_modal(
            cid_report_user_modal(target.id.get()),
            format!("🚨 Report {}", target.name),
        ),
    )
    .await?;
    Ok(())
}
