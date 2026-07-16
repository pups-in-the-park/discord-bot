use poise::serenity_prelude as serenity;

use super::{BanDuration, DeleteMessages};
use crate::context::{Context, Error};
use crate::features::moderation::{
    service::ban_expiry,
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

    let infraction = crate::features::moderation::service::apply_ban(
        &ctx.serenity_context().http,
        &ctx.data(),
        guild_id,
        &user,
        ctx.author().id,
        &reason,
        delete_secs as u32,
        dur,
        expires_at.as_deref(),
        until_ts,
        appealable,
        mod_cfg.dm_on_ban,
    )
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
