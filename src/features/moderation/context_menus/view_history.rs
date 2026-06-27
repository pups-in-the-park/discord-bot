use poise::serenity_prelude as serenity;

use crate::context::{Context, Error};
use crate::features::moderation::view::history_embed;

/// View moderation history for this user (staff only).
#[poise::command(context_menu_command = "View History", guild_only, default_member_permissions = "MODERATE_MEMBERS")]
pub async fn view_history(ctx: Context<'_>, target: serenity::User) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let infractions = ctx
        .data()
        .db
        .get_infractions(&guild_id.to_string(), &target.id.to_string())
        .await?;

    ctx.send(
        poise::CreateReply::default()
            .ephemeral(true)
            .embed(history_embed(&target.name, &infractions)),
    )
    .await?;
    Ok(())
}
