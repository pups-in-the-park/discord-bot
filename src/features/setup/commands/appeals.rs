use super::super::view::build_setup_appeals_form;
use crate::context::{Context, Error};
use crate::ui::slash_respond;

/// Configure appeals and concerns channels, and the appeal waiting period.
#[poise::command(slash_command, guild_only, rename = "appeals")]
pub async fn appeals(ctx: Context<'_>) -> Result<(), Error> {
    let g = ctx.guild_id().unwrap().to_string();
    let guild_cfg = ctx.data().db.get_or_create_guild(&g).await?;
    let mod_cfg = ctx.data().db.get_or_create_mod_config(&g).await?;
    slash_respond(ctx, &build_setup_appeals_form(&guild_cfg, mod_cfg.appeal_cooldown_days)).await?;
    Ok(())
}
