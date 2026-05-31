use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};

/// Give a role to a user.
#[poise::command(slash_command, guild_only, default_member_permissions = "MANAGE_ROLES")]
pub async fn give(
    ctx: Context<'_>,
    #[description = "User"] user: serenity::User,
    #[description = "Role to give"] role: serenity::Role,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    guild_id
        .member(&ctx, user.id)
        .await
        .map_err(|_| Error::user("User is not in this server."))?
        .add_role(&ctx, role.id)
        .await
        .map_err(|e| Error::user(format!("Failed to add role: {}", e)))?;

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::GREEN)
                .title("Role Added")
                .description(format!("<@&{}> given to <@{}>.", role.id, user.id)),
        ),
    )
    .await?;
    Ok(())
}
