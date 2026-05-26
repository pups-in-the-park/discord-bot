use super::super::view::build_setup_log_form;
use crate::context::{Context, Error};
use crate::ui::slash_respond;

/// Configure where logs are posted (fallback, moderation, chat, ticket).
#[poise::command(slash_command, guild_only, rename = "logs")]
pub async fn logs(ctx: Context<'_>) -> Result<(), Error> {
    let g = ctx.guild_id().unwrap().to_string();
    let cfg = ctx.data().db.get_or_create_guild(&g).await?;
    slash_respond(ctx, &build_setup_log_form(&cfg)).await?;
    Ok(())
}
