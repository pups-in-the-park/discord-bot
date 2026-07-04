use super::super::view::build_setup_hub;
use crate::context::{Context, Error};
use crate::ui::slash_respond;

/// Setup dashboard: see what's configured and jump into any area. Start here.
#[poise::command(slash_command, guild_only, rename = "overview", default_member_permissions = "MANAGE_GUILD")]
pub async fn overview(ctx: Context<'_>) -> Result<(), Error> {
    let g = ctx.guild_id().unwrap().to_string();
    let db = &ctx.data().db;

    // Six independent queries — run them concurrently to answer well within
    // Discord's 3-second interaction window.
    let (guild, modc, raid, slow, cats, panels) = tokio::try_join!(
        db.get_or_create_guild(&g),
        db.get_or_create_mod_config(&g),
        db.get_or_create_raid_config(&g),
        db.get_or_create_slowmode_config(&g),
        db.get_ticket_types(&g),
        db.get_panels(&g),
    )?;

    let num_categories = cats.len();
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
