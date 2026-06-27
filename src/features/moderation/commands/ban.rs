use poise::serenity_prelude as serenity;

use super::{BanDuration, DeleteMessages};
use crate::context::{Context, Error};
use crate::features::moderation::{
    service::{ban_expiry, send_action_dm, ModActionDm},
    view::{confirm_embed, log_action},
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

    // A new ban supersedes any active one — deactivate older ban infractions
    // first, or an earlier temp ban's expiry would lift this ban.
    ctx.data().db.deactivate_active_bans(&gid, &user.id.to_string()).await.ok();
    // Record and DM *before* the ban: once the member is removed they share no
    // guild with the bot, so the ban/appeal notice can no longer be delivered.
    // If the ban itself fails below, the command aborts with the error rather
    // than confirming a ban that never happened.
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

    guild_id
        .ban(&ctx.serenity_context().http, user.id, delete_secs as u32, Some(&reason))
        .await
        .map_err(|e| Error::user(format!("Failed to ban user: {}", e)))?;

    let length = match until_ts {
        Some(ts) => format!("Temporary — lifts <t:{}:R>", ts),
        None => "Permanent".to_string(),
    };
    log_action(
        &ctx.serenity_context().http,
        &ctx.data(),
        guild_id,
        "ban",
        Some(infraction.id),
        &user,
        ctx.author().id,
        &reason,
        Some(&length),
    )
    .await;

    let length_phrase = match until_ts {
        Some(ts) => format!("until <t:{ts}:F> (<t:{ts}:R>)"),
        None => "permanently".to_string(),
    };
    ctx.send(poise::CreateReply::default().ephemeral(true).embed(confirm_embed(
        "ban",
        Some(infraction.id),
        format!(
            "<@{}> is banned {}.\n**Reason:** {}\n**Appeals:** {}",
            user.id,
            length_phrase,
            reason,
            if appealable { "Allowed" } else { "Not allowed" }
        ),
    )))
    .await?;
    Ok(())
}
