use poise::serenity_prelude as serenity;

use crate::context::{Context, Error};
use crate::features::moderation::{
    service::{send_action_dm, ModActionDm},
    view::{confirm_embed, log_action},
};

/// Remove a timeout from a user.
#[poise::command(slash_command, guild_only, default_member_permissions = "MODERATE_MEMBERS")]
pub async fn untimeout(
    ctx: Context<'_>,
    #[description = "User"] user: serenity::User,
    #[description = "Reason"] reason: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    guild_id
        .edit_member(
            &ctx.serenity_context().http,
            user.id,
            serenity::EditMember::new().enable_communication(),
        )
        .await
        .map_err(|e| Error::user(format!("Failed to remove timeout: {}", e)))?;

    ctx.data()
        .db
        .create_infraction(
            &guild_id.to_string(),
            &user.id.to_string(),
            &ctx.author().id.to_string(),
            "untimeout",
            &reason,
            None,
            false,
            None,
        )
        .await?;

    send_action_dm(&ctx.serenity_context().http, &user, guild_id, ModActionDm::Untimeout, None).await;

    log_action(
        &ctx.serenity_context().http,
        &ctx.data(),
        guild_id,
        "untimeout",
        None,
        &user,
        ctx.author().id,
        &reason,
        None,
    )
    .await;

    ctx.send(poise::CreateReply::default().ephemeral(true).embed(confirm_embed(
        "untimeout",
        None,
        format!("<@{}> can speak again.\n**Reason:** {}", user.id, reason),
    )))
    .await?;
    Ok(())
}
