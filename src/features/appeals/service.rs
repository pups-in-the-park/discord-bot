//! Non-UI appeal logic shared by the slash commands (`/appeal accept|deny`) and the
//! Accept/Deny buttons on the staff appeal card. Resolving an appeal updates the DB,
//! rewrites the `#appeals` card, archives the thread, applies the unban (for accepted
//! ban appeals), and DMs the appellant.

use std::sync::Arc;

use anyhow::Result;
use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::db::Appeal;

/// Resolve an appeal as accepted/denied. Performs every side effect except the
/// confirmation shown to the acting moderator (the caller does that, since the
/// surface differs between a slash command and a button followup).
pub async fn resolve(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    appeal: &Appeal,
    accept: bool,
    response: &str,
    moderator: serenity::UserId,
) -> Result<()> {
    let status = if accept { "accepted" } else { "denied" };
    let guild_id = serenity::GuildId::new(appeal.guild_id.parse::<u64>()?);

    data.db
        .resolve_appeal(appeal.id, status, response, &moderator.to_string())
        .await?;

    // Rewrite the staff card in the appeals channel to an accepted/denied summary.
    if let (Some(card_msg_id), Some(appeals_ch)) = (
        appeal.card_message_id.as_ref().and_then(|s| s.parse::<u64>().ok()),
        data.db
            .get_or_create_guild(&appeal.guild_id)
            .await
            .ok()
            .and_then(|g| g.appeals_channel_id)
            .and_then(|s| s.parse::<u64>().ok()),
    ) {
        super::view::mark_appeal_resolved(
            ctx,
            serenity::ChannelId::new(appeals_ch),
            serenity::MessageId::new(card_msg_id),
            status,
            &appeal.user_id,
            response,
            moderator,
        )
        .await;
    }

    // Archive the appeal thread now that it's resolved.
    if let Some(tid) = appeal.thread_id.as_ref().and_then(|s| s.parse::<u64>().ok()) {
        serenity::ThreadId::new(tid)
            .edit(&ctx.http, serenity::EditThread::new().archived(true))
            .await
            .ok();
    }

    let infraction = data.db.get_infraction_by_id(appeal.infraction_id).await?;
    let Some(inf) = infraction else {
        return Ok(());
    };
    let Ok(uid) = inf.user_id.parse::<u64>().map(serenity::UserId::new) else {
        return Ok(());
    };

    if accept {
        let mut invite_url = None;
        if inf.kind == "ban" {
            guild_id.unban(&ctx.http, uid, None).await.ok();
            data.db
                .create_infraction(
                    &guild_id.to_string(),
                    &uid.to_string(),
                    &moderator.to_string(),
                    "unban",
                    &format!("Appeal accepted: {}", response),
                    None,
                    false,
                    None,
                )
                .await
                .ok();
            invite_url = make_rejoin_invite(ctx, guild_id).await;
        }
        if let Ok(user) = uid.to_user(ctx).await {
            super::view::notify_appeal_resolved(
                &ctx.http,
                &user,
                status,
                response,
                invite_url.as_deref(),
            )
            .await;
        }
    } else if let Ok(user) = uid.to_user(ctx).await {
        super::view::notify_appeal_denied(&ctx.http, &user, guild_id, response, appeal.id).await;
    }

    Ok(())
}

/// Create a one-use, one-week rejoin invite to the first text channel, if any.
pub async fn make_rejoin_invite(
    ctx: &serenity::Context,
    guild_id: serenity::GuildId,
) -> Option<String> {
    let channels = guild_id.channels(&ctx.http).await.ok()?;
    let channel = channels
        .into_iter()
        .find(|c| c.base.kind == serenity::ChannelType::Text)?;
    let invite = channel
        .id
        .create_invite(
            &ctx.http,
            serenity::CreateInvite::new().max_uses(1).max_age(604800),
        )
        .await
        .ok()?;
    Some(format!("https://discord.gg/{}", invite.code))
}
