//! `/appeal` parent group; `accept`/`deny` live one-per-file and share
//! [`resolve_appeal`], which is run from inside an appeal thread.

mod accept;
mod deny;

pub use accept::accept;
pub use deny::deny;

use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};

/// Manage appeals (must be used inside an appeal thread).
#[poise::command(slash_command, guild_only, subcommands("accept", "deny"))]
pub async fn appeal(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Shared accept/deny logic: resolve the appeal, update the card, archive the
/// thread, unban if applicable, and DM the appellant.
pub(crate) async fn resolve_appeal(ctx: Context<'_>, status: &str, response: String) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let channel_id = ctx.channel_id();

    let appeal = ctx
        .data()
        .db
        .get_appeal_by_thread(&channel_id.to_string())
        .await?
        .ok_or_else(|| Error::user("This command must be used inside an appeal thread."))?;

    if appeal.status != "pending" {
        return Err(Error::user("This appeal has already been resolved."));
    }

    ctx.data()
        .db
        .resolve_appeal(appeal.id, status, &response, &ctx.author().id.to_string())
        .await?;

    // Update the appeal card in the appeals channel.
    if let (Some(card_msg_id), Some(appeals_ch)) = (
        appeal.card_message_id.as_ref().and_then(|s| s.parse::<u64>().ok()),
        ctx.data()
            .db
            .get_or_create_guild(&guild_id.to_string())
            .await
            .ok()
            .and_then(|g| g.appeals_channel_id)
            .and_then(|s| s.parse::<u64>().ok()),
    ) {
        super::view::mark_appeal_resolved(
            ctx.serenity_context(),
            serenity::ChannelId::new(appeals_ch),
            serenity::MessageId::new(card_msg_id),
            status,
            &appeal.user_id,
            &response,
            ctx.author().id,
        )
        .await;
    }

    channel_id
        .expect_thread()
        .edit(&ctx.serenity_context().http, serenity::EditThread::new().archived(true))
        .await
        .ok();

    let infraction = ctx.data().db.get_infraction_by_id(appeal.infraction_id).await?;
    if status == "accepted" {
        if let Some(inf) = &infraction {
            let uid = inf.user_id.parse::<u64>().map(serenity::UserId::new).ok();
            if inf.kind == "ban" {
                if let Some(uid) = uid {
                    guild_id
                        .unban(&ctx.serenity_context().http, uid, None)
                        .await
                        .ok();
                    ctx.data()
                        .db
                        .create_infraction(
                            &guild_id.to_string(),
                            &uid.to_string(),
                            &ctx.author().id.to_string(),
                            "unban",
                            &format!("Appeal accepted: {}", response),
                            None,
                            false,
                            None,
                        )
                        .await
                        .ok();

                    let invite_url = make_rejoin_invite(&ctx, guild_id).await;
                    if let Ok(user) = uid.to_user(&ctx).await {
                        super::view::notify_appeal_resolved(
                            &ctx.serenity_context().http,
                            &user,
                            status,
                            &response,
                            invite_url.as_deref(),
                        )
                        .await;
                    }
                }
            } else if let Some(uid) = uid {
                if let Ok(user) = uid.to_user(&ctx).await {
                    super::view::notify_appeal_resolved(
                        &ctx.serenity_context().http,
                        &user,
                        status,
                        &response,
                        None,
                    )
                    .await;
                }
            }
        }
    } else if let Some(inf) = &infraction {
        let uid = inf.user_id.parse::<u64>().map(serenity::UserId::new).ok();
        if let Some(uid) = uid {
            if let Ok(user) = uid.to_user(&ctx).await {
                super::view::notify_appeal_denied(
                    &ctx.serenity_context().http,
                    &user,
                    guild_id,
                    &response,
                    appeal.id,
                )
                .await;
            }
        }
    }

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(if status == "accepted" { colours::GREEN } else { colours::RED })
                .title(if status == "accepted" { "✅ Appeal Accepted" } else { "❌ Appeal Denied" })
                .description("Response sent to user. Thread archived."),
        ),
    )
    .await?;
    Ok(())
}

/// Create a one-use, one-week rejoin invite to the first text channel, if any.
async fn make_rejoin_invite(ctx: &Context<'_>, guild_id: serenity::GuildId) -> Option<String> {
    let channels = guild_id.channels(&ctx.serenity_context().http).await.ok()?;
    let channel = channels
        .into_iter()
        .find(|c| c.base.kind == serenity::ChannelType::Text)?;
    let invite = channel
        .id
        .create_invite(&ctx.serenity_context().http, serenity::CreateInvite::new().max_uses(1).max_age(604800))
        .await
        .ok()?;
    Some(format!("https://discord.gg/{}", invite.code))
}
