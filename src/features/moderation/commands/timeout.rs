use poise::serenity_prelude as serenity;

use super::TimeoutDuration;
use crate::context::{colours, Context, Error};
use crate::features::moderation::{
    service::{send_action_dm, ModActionDm},
    view::log_action,
};
use crate::permissions::validate_target;
use crate::util::format_duration;

/// Timeout (mute) a user for a duration.
#[poise::command(slash_command, guild_only, default_member_permissions = "MODERATE_MEMBERS")]
pub async fn timeout(
    ctx: Context<'_>,
    #[description = "User to timeout"] user: serenity::User,
    #[description = "Duration"] duration: TimeoutDuration,
    #[description = "Reason"] reason: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let gid = guild_id.to_string();
    validate_target(&ctx, &user).await?;

    let secs = duration.as_secs();
    let until = serenity::Timestamp::from_unix_timestamp(
        serenity::Timestamp::now().unix_timestamp() + secs,
    )
    .map_err(|_| Error::user("Failed to compute timeout timestamp."))?;

    let until_str = until.to_rfc3339();

    guild_id
        .edit_member(
            &ctx.serenity_context().http,
            user.id,
            serenity::EditMember::new().disable_communication_until(until),
        )
        .await
        .map_err(|e| Error::user(format!("Failed to timeout user: {}", e)))?;

    let expires_at = until_str;
    let infraction = ctx
        .data()
        .db
        .create_infraction(
            &gid,
            &user.id.to_string(),
            &ctx.author().id.to_string(),
            "timeout",
            &reason,
            Some(secs),
            true,
            Some(expires_at.as_str()),
        )
        .await?;

    let mod_cfg = ctx.data().db.get_or_create_mod_config(&gid).await?;
    if mod_cfg.dm_on_timeout {
        send_action_dm(
            &ctx.serenity_context().http,
            &user,
            guild_id,
            ModActionDm::Timeout { reason: &reason, until },
            Some((infraction.id, guild_id)),
        )
        .await;
    }

    let info = format!("{} · ends <t:{}:R>", format_duration(secs), until.unix_timestamp());
    log_action(
        ctx.serenity_context(),
        &ctx.data(),
        guild_id,
        "timeout",
        Some(infraction.id),
        &user,
        ctx.author(),
        &reason,
        Some(&info),
    )
    .await;

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::YELLOW)
                .title("⏱️ Timeout Applied")
                .description(format!(
                    "<@{}> timed out for {}.\nReason: {}",
                    user.id,
                    format_duration(secs),
                    reason
                )),
        ),
    )
    .await?;
    Ok(())
}
