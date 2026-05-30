use poise::serenity_prelude as serenity;

use super::autocomplete_scope;
use crate::context::{colours, Context, Error};

/// Add a user to the blocklist.
#[poise::command(slash_command, guild_only, default_member_permissions = "MANAGE_GUILD")]
pub async fn add(
    ctx: Context<'_>,
    #[description = "User to blocklist"] user: serenity::User,
    #[description = "Scope: global or a specific category name"]
    #[autocomplete = "autocomplete_scope"]
    scope: String,
    #[description = "Reason"] reason: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let added = ctx
        .data()
        .db
        .blocklist_add(
            &guild_id.to_string(),
            &user.id.to_string(),
            &scope,
            &reason,
            &ctx.author().id.to_string(),
        )
        .await?;

    if !added {
        return Err(Error::user(format!(
            "{} is already on the blocklist for scope '{}'.",
            user.name, scope
        )));
    }

    // Record as infraction
    ctx.data()
        .db
        .create_infraction(
            &guild_id.to_string(),
            &user.id.to_string(),
            &ctx.author().id.to_string(),
            "blocklist",
            &reason,
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
                    "<@{}> added to blocklist (scope: `{}`)\nReason: {}",
                    user.id, scope, reason
                )),
        ),
    )
    .await?;
    Ok(())
}
