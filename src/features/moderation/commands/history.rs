use poise::serenity_prelude as serenity;

use crate::context::{Context, Error};
use crate::features::moderation::view::history_embed;

/// View a user's infraction history.
#[poise::command(slash_command, guild_only, default_member_permissions = "MODERATE_MEMBERS")]
pub async fn history(
    ctx: Context<'_>,
    #[description = "User"] user: serenity::User,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let infractions = ctx
        .data()
        .db
        .get_infractions(&guild_id.to_string(), &user.id.to_string())
        .await?;

    ctx.send(
        poise::CreateReply::default()
            .ephemeral(true)
            .embed(history_embed(&user.name, &infractions)),
    )
    .await?;
    Ok(())
}
