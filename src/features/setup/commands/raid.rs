use super::super::view::build_setup_raid_form;
use crate::context::{Context, Error};
use crate::ui::slash_respond;

/// Configure anti-raid detection sensitivity and slowmode response.
#[poise::command(slash_command, guild_only, rename = "raid")]
pub async fn raid(ctx: Context<'_>) -> Result<(), Error> {
    let g = ctx.guild_id().unwrap().to_string();
    let cfg = ctx.data().db.get_or_create_raid_config(&g).await?;
    slash_respond(ctx, &build_setup_raid_form(&cfg)).await?;
    Ok(())
}
