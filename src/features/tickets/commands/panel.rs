use crate::context::{Context, Error};
use crate::ui::slash_respond;

use super::super::view::build_panel_hub;

/// Manage ticket panels.
#[poise::command(slash_command, guild_only, subcommands("manage"))]
pub async fn panel(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Create, configure, and publish ticket panels from one place.
#[poise::command(slash_command, guild_only, rename = "manage", default_member_permissions = "MANAGE_GUILD")]
pub async fn manage(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let panels = ctx.data().db.get_panels(&guild_id.to_string()).await?;
    slash_respond(ctx, &build_panel_hub(&panels)).await?;
    Ok(())
}
