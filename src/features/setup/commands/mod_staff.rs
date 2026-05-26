use super::super::view::build_setup_mod_form;
use crate::context::{Context, Error};
use crate::ui::slash_respond;

/// Configure moderation staff roles and DM notification settings.
#[poise::command(slash_command, guild_only, rename = "mod")]
pub async fn mod_staff(ctx: Context<'_>) -> Result<(), Error> {
    let g = ctx.guild_id().unwrap().to_string();
    let cfg = ctx.data().db.get_or_create_mod_config(&g).await?;
    slash_respond(ctx, &build_setup_mod_form(&cfg)).await?;
    Ok(())
}
