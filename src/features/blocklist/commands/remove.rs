use poise::serenity_prelude as serenity;

use super::autocomplete_scope;
use crate::context::{colours, Context, Error};

/// Remove a user from the blocklist.
#[poise::command(slash_command, guild_only, default_member_permissions = "MANAGE_GUILD")]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "User to remove"] user: serenity::User,
    #[description = "Scope"]
    #[autocomplete = "autocomplete_scope"]
    scope: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let removed = ctx
        .data()
        .db
        .blocklist_remove(&guild_id.to_string(), &user.id.to_string(), &scope)
        .await?;

    if !removed {
        return Err(Error::user(format!(
            "{} is not on the blocklist for scope '{}'.",
            user.name, scope
        )));
    }

    ctx.data()
        .db
        .create_infraction(
            &guild_id.to_string(),
            &user.id.to_string(),
            &ctx.author().id.to_string(),
            "unblocklist",
            &format!("Removed from blocklist (scope: {})", scope),
            None,
            false,
            None,
        )
        .await?;

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::GREEN)
                .title("Blocklist Updated")
                .description(format!(
                    "<@{}> removed from blocklist (scope: `{}`)",
                    user.id, scope
                )),
        ),
    )
    .await?;
    Ok(())
}
