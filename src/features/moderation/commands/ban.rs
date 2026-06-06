use poise::serenity_prelude as serenity;

use super::{BanDuration, DeleteMessages};
use crate::context::{colours, Context, Error};
use crate::features::moderation::{
    service::{ban_expiry, send_action_dm, ModActionDm},
    view::log_action,
};
use crate::permissions::validate_target;

/// Ban a user from the server.
#[poise::command(slash_command, guild_only, default_member_permissions = "BAN_MEMBERS")]
pub async fn ban(
    ctx: Context<'_>,
    #[description = "User to ban"] user: serenity::User,
    #[description = "Reason"] reason: String,
    #[description = "Ban length (default: permanent)"] duration: Option<BanDuration>,
    #[description = "Delete messages"] delete_messages: Option<DeleteMessages>,
    #[description = "Allow appeals (default: true)"] appealable: Option<bool>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let gid = guild_id.to_string();
    validate_target(&ctx, &user).await?;

    let delete_secs = delete_messages.as_ref().map(|d| d.as_secs()).unwrap_or(0);
    let appealable = appealable.unwrap_or(true);
    let (dur, expires_at, until_ts) = ban_expiry(duration.as_ref().map(|d| d.as_secs()).unwrap_or(0));

    let mod_cfg = ctx.data().db.get_or_create_mod_config(&gid).await?;

    guild_id
        .ban(&ctx.serenity_context().http, user.id, delete_secs as u32, Some(&reason))
        .await
        .map_err(|e| Error::user(format!("Failed to ban user: {}", e)))?;

    let infraction = ctx
        .data()
        .db
        .create_infraction(
            &gid,
            &user.id.to_string(),
            &ctx.author().id.to_string(),
            "ban",
            &reason,
            dur,
            appealable,
            expires_at.as_deref(),
        )
        .await?;

    if mod_cfg.dm_on_ban {
        let appeal_info = if appealable { Some((infraction.id, guild_id)) } else { None };
        send_action_dm(
            &ctx.serenity_context().http,
            &user,
            guild_id,
            ModActionDm::Ban { reason: &reason, until: until_ts },
            appeal_info,
        )
        .await;
    }

    let length = match until_ts {
        Some(ts) => format!("Temporary — lifts <t:{}:R>", ts),
        None => "Permanent".to_string(),
    };
    log_action(
        ctx.serenity_context(),
        &ctx.data(),
        guild_id,
        "ban",
        Some(infraction.id),
        &user,
        ctx.author(),
        &reason,
        Some(&length),
    )
    .await;

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::DARK_RED)
                .title("🔨 Member Banned")
                .description(format!(
                    "<@{}> has been banned.\nReason: {}\nLength: {}\nAppealable: {}",
                    user.id,
                    reason,
                    length,
                    if appealable { "Yes" } else { "No" }
                )),
        ),
    )
    .await?;
    Ok(())
}
