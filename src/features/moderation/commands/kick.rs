use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};
use crate::features::moderation::{service::send_action_dm, view::log_action};
use crate::permissions::validate_target;

/// Kick a user from the server.
#[poise::command(slash_command, guild_only, default_member_permissions = "KICK_MEMBERS")]
pub async fn kick(
    ctx: Context<'_>,
    #[description = "User to kick"] user: serenity::User,
    #[description = "Reason"] reason: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let gid = guild_id.to_string();
    validate_target(&ctx, &user).await?;

    let mod_cfg = ctx.data().db.get_or_create_mod_config(&gid).await?;
    if mod_cfg.dm_on_kick {
        send_action_dm(&ctx.serenity_context().http, &user, guild_id, "👢 Kicked", &reason, None)
            .await;
    }

    guild_id
        .kick_with_reason(&ctx, user.id, &reason)
        .await
        .map_err(|e| Error::user(format!("Failed to kick user: {}", e)))?;

    ctx.data()
        .db
        .create_infraction(
            &gid,
            &user.id.to_string(),
            &ctx.author().id.to_string(),
            "kick",
            &reason,
            None,
            false,
            None,
        )
        .await?;

    log_action(
        ctx.serenity_context(),
        ctx.data(),
        guild_id,
        "👢 Member Kicked",
        &user,
        ctx.author(),
        &reason,
        None,
    )
    .await;

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::ORANGE)
                .title("👢 Member Kicked")
                .description(format!("<@{}> has been kicked.\nReason: {}", user.id, reason)),
        ),
    )
    .await?;
    Ok(())
}
