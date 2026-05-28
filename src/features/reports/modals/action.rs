use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::parse_report_action_modal;
use crate::util::modal_field;

/// A report "take action" modal was submitted (`m:ract:{action}:{report_id}:{target_id}`).
/// Applies the infraction, resolves the report, archives the thread, DMs the reporter.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let (action, report_id, target_id) = parse_report_action_modal(&mi.data.custom_id)
        .ok_or_else(|| anyhow::anyhow!("Invalid report action modal id"))?;

    let Some(guild_id) = mi.guild_id else {
        return Ok(());
    };
    if !crate::permissions::is_mod_staff(ctx, data, guild_id, mi.user.id).await {
        ephemeral_reply(ctx, mi, "Only staff can take action on reports.").await?;
        return Ok(());
    }
    let reason = modal_field(&mi.data.components, "reason")
        .unwrap_or("No reason given")
        .to_string();

    let report = data
        .db
        .get_report_by_id(report_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Report not found"))?;

    if report.status != "open" {
        ephemeral_reply(ctx, mi, "This report has already been resolved.").await?;
        return Ok(());
    }
    let reporter_id = report.reporter_id.clone();

    match action.as_str() {
        "warn" => {
            let target = serenity::UserId::new(target_id).to_user(ctx).await?;
            let infraction = data
                .db
                .create_infraction(
                    &guild_id.to_string(),
                    &target_id.to_string(),
                    &mi.user.id.to_string(),
                    "warn",
                    &reason,
                    None,
                    true,
                    None,
                )
                .await?;
            let mod_cfg = data.db.get_or_create_mod_config(&guild_id.to_string()).await?;
            if mod_cfg.dm_on_warn {
                crate::features::moderation::service::send_action_dm(
                    &ctx.http,
                    &target,
                    guild_id,
                    "⚠️ Warning",
                    &reason,
                    Some((infraction.id, guild_id)),
                )
                .await;
            }
        }
        "dw" => {
            if let Some(ref msg_url) = report.message_url {
                let url_parts: Vec<&str> = msg_url.split('/').collect();
                let n = url_parts.len();
                if n >= 2 {
                    if let (Ok(ch_id), Ok(m_id)) =
                        (url_parts[n - 2].parse::<u64>(), url_parts[n - 1].parse::<u64>())
                    {
                        serenity::ChannelId::new(ch_id)
                            .delete_message(ctx, serenity::MessageId::new(m_id))
                            .await
                            .ok();
                    }
                }
            }
            let target = serenity::UserId::new(target_id).to_user(ctx).await?;
            let infraction = data
                .db
                .create_infraction(
                    &guild_id.to_string(),
                    &target_id.to_string(),
                    &mi.user.id.to_string(),
                    "warn",
                    &format!("[Message deleted] {}", reason),
                    None,
                    true,
                    None,
                )
                .await?;
            let mod_cfg = data.db.get_or_create_mod_config(&guild_id.to_string()).await?;
            if mod_cfg.dm_on_warn {
                crate::features::moderation::service::send_action_dm(
                    &ctx.http,
                    &target,
                    guild_id,
                    "⚠️ Warning (message deleted)",
                    &reason,
                    Some((infraction.id, guild_id)),
                )
                .await;
            }
        }
        "timeout" => {
            let duration_str = modal_field(&mi.data.components, "duration").unwrap_or("3600");
            let secs: i64 = duration_str.trim().parse().unwrap_or(3600);
            let until = serenity::Timestamp::from_unix_timestamp(
                serenity::Timestamp::now().unix_timestamp() + secs,
            )
            .map_err(|_| anyhow::anyhow!("Invalid timestamp"))?;
            let until_str = until
                .to_rfc3339()
                .ok_or_else(|| anyhow::anyhow!("Failed to format timestamp"))?;
            guild_id
                .edit_member(
                    ctx,
                    serenity::UserId::new(target_id),
                    serenity::EditMember::new().disable_communication_until(until_str),
                )
                .await
                .ok();
            let expires_at = until.to_rfc3339();
            data.db
                .create_infraction(
                    &guild_id.to_string(),
                    &target_id.to_string(),
                    &mi.user.id.to_string(),
                    "timeout",
                    &reason,
                    Some(secs),
                    true,
                    expires_at.as_deref(),
                )
                .await?;
        }
        "kick" => {
            guild_id
                .kick_with_reason(ctx, serenity::UserId::new(target_id), &reason)
                .await
                .ok();
            data.db
                .create_infraction(
                    &guild_id.to_string(),
                    &target_id.to_string(),
                    &mi.user.id.to_string(),
                    "kick",
                    &reason,
                    None,
                    false,
                    None,
                )
                .await?;
        }
        "ban" => {
            let appealable = modal_field(&mi.data.components, "appealable")
                .map(|s| s.trim().to_lowercase() != "no")
                .unwrap_or(true);
            guild_id
                .ban_with_reason(ctx, serenity::UserId::new(target_id), 0, &reason)
                .await
                .ok();
            let infraction = data
                .db
                .create_infraction(
                    &guild_id.to_string(),
                    &target_id.to_string(),
                    &mi.user.id.to_string(),
                    "ban",
                    &reason,
                    None,
                    appealable,
                    None,
                )
                .await?;
            let target = serenity::UserId::new(target_id).to_user(ctx).await?;
            let mod_cfg = data.db.get_or_create_mod_config(&guild_id.to_string()).await?;
            if mod_cfg.dm_on_ban {
                let appeal_info = if appealable {
                    Some((infraction.id, guild_id))
                } else {
                    None
                };
                crate::features::moderation::service::send_action_dm(
                    &ctx.http,
                    &target,
                    guild_id,
                    "🔨 Banned",
                    &reason,
                    appeal_info,
                )
                .await;
            }
        }
        _ => {}
    }

    data.db
        .resolve_report(report_id, "action_taken", &mi.user.id.to_string())
        .await?;
    mi.channel_id
        .edit_thread(ctx, serenity::EditThread::new().archived(true))
        .await
        .ok();
    super::super::view::notify_reporter_action_taken(ctx, &reporter_id).await;

    mi.create_response(
        ctx,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content("Action taken. Report resolved and thread archived."),
        ),
    )
    .await?;
    Ok(())
}

/// Reply to a modal submission with a short ephemeral message.
async fn ephemeral_reply(
    ctx: &serenity::Context,
    mi: &serenity::ModalInteraction,
    msg: &str,
) -> Result<(), anyhow::Error> {
    mi.create_response(
        ctx,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content(msg),
        ),
    )
    .await?;
    Ok(())
}
