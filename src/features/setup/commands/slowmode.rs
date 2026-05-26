use super::super::view::build_setup_slowmode_form;
use crate::context::{Context, Error};
use crate::ui::slash_respond;

/// Configure the automatic channel slowmode trigger.
#[poise::command(slash_command, guild_only, rename = "slowmode")]
pub async fn slowmode(ctx: Context<'_>) -> Result<(), Error> {
    let g = ctx.guild_id().unwrap().to_string();
    let cfg = ctx.data().db.get_or_create_slowmode_config(&g).await?;
    slash_respond(ctx, &build_setup_slowmode_form(&cfg)).await?;
    Ok(())
}
