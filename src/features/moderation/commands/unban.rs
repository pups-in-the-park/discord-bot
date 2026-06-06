use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};
use crate::features::moderation::service::{send_action_dm, ModActionDm};

/// Unban a user.
#[poise::command(slash_command, guild_only, default_member_permissions = "BAN_MEMBERS")]
pub async fn unban(
    ctx: Context<'_>,
    #[description = "User ID to unban"] user_id: String,
    #[description = "Reason"] reason: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let uid = user_id
        .parse::<u64>()
        .map(serenity::UserId::new)
        .map_err(|_| Error::user("Invalid user ID."))?;

    guild_id
        .unban(&ctx.serenity_context().http, uid, Some(&reason))
        .await
        .map_err(|e| Error::user(format!("Failed to unban: {}", e)))?;

    ctx.data()
        .db
        .create_infraction(
            &guild_id.to_string(),
            &uid.to_string(),
            &ctx.author().id.to_string(),
            "unban",
            &reason,
            None,
            false,
            None,
        )
        .await?;

    // Clear any active ban infraction so the temp-ban expiry task won't re-process it.
    ctx.data().db.deactivate_active_bans(&guild_id.to_string(), &uid.to_string()).await.ok();

    // Best-effort courtesy DM (the user shares no guild with us once unbanned, so
    // this may not deliver).
    if let Ok(user) = uid.to_user(&ctx.serenity_context().http).await {
        send_action_dm(&ctx.serenity_context().http, &user, guild_id, ModActionDm::Unban, None).await;
    }

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::GREEN)
                .title("🔓 Member Unbanned")
                .description(format!("<@{}> has been unbanned.\nReason: {}", uid, reason)),
        ),
    )
    .await?;
    Ok(())
}
