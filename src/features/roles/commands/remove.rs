use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};

/// Remove a role from a user.
#[poise::command(slash_command, guild_only, default_member_permissions = "MANAGE_ROLES")]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "User"] user: serenity::User,
    #[description = "Role to remove"] role: serenity::Role,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    guild_id
        .member(&ctx, user.id)
        .await
        .map_err(|_| Error::user("User is not in this server."))?
        .remove_role(&ctx, role.id)
        .await
        .map_err(|e| Error::user(format!("Failed to remove role: {}", e)))?;

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::GREEN)
                .title("Role Removed")
                .description(format!("<@&{}> removed from <@{}>.", role.id, user.id)),
        ),
    )
    .await?;
    Ok(())
}
