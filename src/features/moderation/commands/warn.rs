use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};
use crate::features::moderation::{
    service::{send_action_dm, ModActionDm},
    view::log_action,
};
use crate::permissions::validate_target;

/// Issue a warning to a user.
#[poise::command(slash_command, guild_only, default_member_permissions = "MODERATE_MEMBERS")]
pub async fn warn(
    ctx: Context<'_>,
    #[description = "User to warn"] user: serenity::User,
    #[description = "Reason"] reason: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let gid = guild_id.to_string();
    validate_target(&ctx, &user).await?;

    let infraction = ctx
        .data()
        .db
        .create_infraction(
            &gid,
            &user.id.to_string(),
            &ctx.author().id.to_string(),
            "warn",
            &reason,
            None,
            true,
            None,
        )
        .await?;

    let mod_cfg = ctx.data().db.get_or_create_mod_config(&gid).await?;
    if mod_cfg.dm_on_warn {
        send_action_dm(
            &ctx.serenity_context().http,
            &user,
            guild_id,
            ModActionDm::Warn { reason: &reason },
            Some((infraction.id, guild_id)),
        )
        .await;
    }

    log_action(
        ctx.serenity_context(),
        &ctx.data(),
        guild_id,
        "warn",
        Some(infraction.id),
        &user,
        ctx.author(),
        &reason,
        None,
    )
    .await;

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::YELLOW)
                .title("⚠️ Warning Issued")
                .description(format!("<@{}> has been warned.\nReason: {}", user.id, reason)),
        ),
    )
    .await?;
    Ok(())
}
