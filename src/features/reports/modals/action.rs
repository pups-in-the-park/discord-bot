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
        crate::util::respond_ephemeral_modal(ctx, mi, "Only staff can take action on reports.").await;
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
        crate::util::respond_ephemeral_modal(ctx, mi, "This report has already been resolved.").await;
        return Ok(());
    }
    if super::super::acting_on_own_report(mi.user.id, &report) {
        crate::util::respond_ephemeral_modal(ctx, mi, super::super::SELF_ACTION_REFUSAL).await;
        return Ok(());
    }
    let reporter_id = report.reporter_id.clone();
    let via_report = format!("Via report #{}", report_id);

    // Applying the action, DMing the target, archiving the thread, and notifying the
    // reporter exceeds the 3-second window — acknowledge first, confirm via follow-up.
    mi.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Defer(
            serenity::CreateInteractionResponseMessage::new().ephemeral(true),
        ),
    )
    .await?;

    // Resolve the target and mod config once, up front — before any irreversible
    // action. A transient failure here aborts before we've touched anything, so
    // the report stays open for a clean retry; doing it per-arm *after* the
    // Discord action (as before) would skip the shared resolve/archive/notify
    // block below and leave the report stuck "open".
    let target = serenity::UserId::new(target_id).to_user(ctx).await?;
    let mod_cfg = data.db.get_or_create_mod_config(&guild_id.to_string()).await?;

    match action.as_str() {
        "warn" => {
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
            if mod_cfg.dm_on_warn {
                crate::features::moderation::service::send_action_dm(
                    &ctx.http,
                    &target,
                    guild_id,
                    crate::features::moderation::service::ModActionDm::Warn { reason: &reason },
                    Some((infraction.id, guild_id)),
                )
                .await;
            }
            crate::features::moderation::view::log_action(
                &ctx.http,
                data,
                guild_id,
                "warn",
                Some(infraction.id),
                &target,
                mi.user.id,
                &reason,
                Some(&via_report),
            )
            .await;
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
                            .widen()
                            .delete_message(&ctx.http, serenity::MessageId::new(m_id), None)
                            .await
                            .ok();
                    }
                }
            }
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
            if mod_cfg.dm_on_warn {
                crate::features::moderation::service::send_action_dm(
                    &ctx.http,
                    &target,
                    guild_id,
                    crate::features::moderation::service::ModActionDm::Warn { reason: &reason },
                    Some((infraction.id, guild_id)),
                )
                .await;
            }
            crate::features::moderation::view::log_action(
                &ctx.http,
                data,
                guild_id,
                "warn",
                Some(infraction.id),
                &target,
                mi.user.id,
                &reason,
                Some(&format!("Message deleted · via report #{}", report_id)),
            )
            .await;
        }
        "timeout" => {
            let secs: i64 =
                crate::util::modal_secs(&mi.data.components, "duration").unwrap_or(3600);
            let until = serenity::Timestamp::from_unix_timestamp(
                serenity::Timestamp::now().unix_timestamp() + secs,
            )
            .map_err(|_| anyhow::anyhow!("Invalid timestamp"))?;
            let until_str = until.to_rfc3339();
            guild_id
                .edit_member(
                    &ctx.http,
                    serenity::UserId::new(target_id),
                    serenity::EditMember::new().disable_communication_until(until),
                )
                .await
                .ok();
            let infraction = data
                .db
                .create_infraction(
                    &guild_id.to_string(),
                    &target_id.to_string(),
                    &mi.user.id.to_string(),
                    "timeout",
                    &reason,
                    Some(secs),
                    true,
                    Some(until_str.as_str()),
                )
                .await?;
            if mod_cfg.dm_on_timeout {
                crate::features::moderation::service::send_action_dm(
                    &ctx.http,
                    &target,
                    guild_id,
                    crate::features::moderation::service::ModActionDm::Timeout {
                        reason: &reason,
                        until,
                    },
                    Some((infraction.id, guild_id)),
                )
                .await;
            }
            let info = format!(
                "{} · ends <t:{}:R> · via report #{}",
                crate::util::format_duration(secs),
                until.unix_timestamp(),
                report_id
            );
            crate::features::moderation::view::log_action(
                &ctx.http,
                data,
                guild_id,
                "timeout",
                Some(infraction.id),
                &target,
                mi.user.id,
                &reason,
                Some(&info),
            )
            .await;
        }
        "kick" => {
            // DM before the kick — once they're out of the guild we can't reach them.
            if mod_cfg.dm_on_kick {
                crate::features::moderation::service::send_action_dm(
                    &ctx.http,
                    &target,
                    guild_id,
                    crate::features::moderation::service::ModActionDm::Kick { reason: &reason },
                    None,
                )
                .await;
            }
            guild_id
                .kick(&ctx.http, serenity::UserId::new(target_id), Some(&reason))
                .await
                .ok();
            let infraction = data
                .db
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
            crate::features::moderation::view::log_action(
                &ctx.http,
                data,
                guild_id,
                "kick",
                Some(infraction.id),
                &target,
                mi.user.id,
                &reason,
                Some(&via_report),
            )
            .await;
        }
        "ban" => {
            let appealable = crate::util::modal_checked(&mi.data.components, "appealable");
            let duration_secs = crate::util::modal_secs(&mi.data.components, "duration").unwrap_or(0);
            let (dur, expires_at, until_ts) =
                crate::features::moderation::service::ban_expiry(duration_secs);
            guild_id
                .ban(&ctx.http, serenity::UserId::new(target_id), 0, Some(&reason))
                .await
                .ok();
            // A new ban supersedes any active one — deactivate older ban
            // infractions first, or an earlier temp ban's expiry would lift it.
            // Best-effort so a transient failure can't skip recording the new
            // ban's expiry (which would leave the user banned forever).
            data.db
                .deactivate_active_bans(&guild_id.to_string(), &target_id.to_string())
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
                    dur,
                    appealable,
                    expires_at.as_deref(),
                )
                .await?;
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
                    crate::features::moderation::service::ModActionDm::Ban { reason: &reason, until: until_ts },
                    appeal_info,
                )
                .await;
            }
            let length = match until_ts {
                Some(ts) => format!("Temporary — lifts <t:{}:R> · via report #{}", ts, report_id),
                None => format!("Permanent · via report #{}", report_id),
            };
            crate::features::moderation::view::log_action(
                &ctx.http,
                data,
                guild_id,
                "ban",
                Some(infraction.id),
                &target,
                mi.user.id,
                &reason,
                Some(&length),
            )
            .await;
        }
        _ => {}
    }

    data.db
        .resolve_report(report_id, "action_taken", &mi.user.id.to_string())
        .await?;
    mi.channel_id
        .expect_thread()
        .edit(&ctx.http, serenity::EditThread::new().archived(true))
        .await
        .ok();
    super::super::view::notify_reporter_action_taken(ctx, &reporter_id).await;

    mi.create_followup(
        &ctx.http,
        serenity::CreateInteractionResponseFollowup::new()
            .ephemeral(true)
            .content(format!(
                "Done — action taken, report #{} resolved, and the thread archived.",
                report_id
            )),
    )
    .await
    .ok();
    Ok(())
}

