//! `/appeal` parent group; `accept`/`deny` live one-per-file and share
//! [`resolve_appeal`], which is run from inside an appeal thread.

mod accept;
mod deny;

pub use accept::accept;
pub use deny::deny;

use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};

/// Manage appeals (must be used inside an appeal thread).
#[poise::command(slash_command, guild_only, subcommands("accept", "deny"))]
pub async fn appeal(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Shared accept/deny logic: resolve the appeal (DB, card, thread, unban, DM) via
/// [`super::service::resolve`], then confirm to the moderator. Runs inside a thread.
pub(crate) async fn resolve_appeal(ctx: Context<'_>, status: &str, response: String) -> Result<(), Error> {
    let channel_id = ctx.channel_id();

    let appeal = ctx
        .data()
        .db
        .get_appeal_by_thread(&channel_id.to_string())
        .await?
        .ok_or_else(|| Error::user("This command must be used inside an appeal thread."))?;

    if appeal.status != "pending" {
        return Err(Error::user("This appeal has already been resolved."));
    }

    let accept = status == "accepted";
    super::service::resolve(
        ctx.serenity_context(),
        &ctx.data(),
        &appeal,
        accept,
        &response,
        ctx.author().id,
    )
    .await?;

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(if accept { colours::GREEN } else { colours::RED })
                .title(if accept { "✅ Appeal Accepted" } else { "❌ Appeal Denied" })
                .description("Response sent to user. Thread archived."),
        ),
    )
    .await?;
    Ok(())
}
