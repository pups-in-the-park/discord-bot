use super::super::view::build_setup_hub;
use crate::context::{Context, Error};
use crate::ui::slash_respond;

/// Setup dashboard: see what's configured and jump into any area. Start here.
#[poise::command(slash_command, guild_only, rename = "overview", default_member_permissions = "MANAGE_GUILD")]
pub async fn overview(ctx: Context<'_>) -> Result<(), Error> {
    let g = ctx.guild_id().unwrap().to_string();
    let db = &ctx.data().db;

    let guild = db.get_or_create_guild(&g).await?;
    let modc = db.get_or_create_mod_config(&g).await?;
    let raid = db.get_or_create_raid_config(&g).await?;
    let slow = db.get_or_create_slowmode_config(&g).await?;

    let num_categories = db.get_ticket_types(&g).await?.len();
    let panels = db.get_panels(&g).await?;
    let num_published = panels.iter().filter(|p| p.message_id.is_some()).count();

    slash_respond(
        ctx,
        &build_setup_hub(
            &guild,
            &modc,
            &raid,
            &slow,
            num_categories,
            num_published,
            panels.len(),
        ),
    )
    .await?;
    Ok(())
}
