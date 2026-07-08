use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::cid_report_action_modal;
use crate::util::respond_ephemeral;

/// The ephemeral action-picker select fired: either delete inline, or open the
/// reason modal for the chosen moderation action.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
    report_id: i64,
) -> Result<(), anyhow::Error> {
    let Some(guild_id) = ci.guild_id else {
        return Ok(());
    };
    if !crate::permissions::is_mod_staff(ctx, data, guild_id, ci.user.id).await {
        respond_ephemeral(ctx, ci, "Only staff can manage reports.").await;
        return Ok(());
    }

    let serenity::ComponentInteractionDataKind::StringSelect { values } = &ci.data.kind else {
        return Ok(());
    };
    let action = values.first().map(|s| s.as_str()).unwrap_or("");

    let report = data
        .db
        .get_report_by_id(report_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Report not found"))?;

    if report.status != "open" {
        respond_ephemeral(ctx, ci, "This report has already been resolved.").await;
        return Ok(());
    }
    if super::super::acting_on_own_report(ci.user.id, &report) {
        respond_ephemeral(ctx, ci, super::super::SELF_ACTION_REFUSAL).await;
        return Ok(());
    }
    let target_id: u64 = report.target_user_id.parse().unwrap_or(0);

    // "del" is handled inline — no modal needed. Deleting the message, archiving the
    // thread, and DMing the reporter can exceed the 3-second window, so acknowledge
    // first and confirm via follow-up. (The other actions open a modal instead, which
    // must remain the initial response and therefore can't be deferred.)
    if action == "del" {
        ci.create_response(
            &ctx.http,
            serenity::CreateInteractionResponse::Defer(
                serenity::CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await?;
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
        data.db
            .resolve_report(report_id, "action_taken", &ci.user.id.to_string())
            .await?;
        ci.channel_id
            .expect_thread()
            .edit(&ctx.http, serenity::EditThread::new().archived(true))
            .await
            .ok();
        super::super::view::notify_reporter_action_taken(ctx, &report.reporter_id).await;
        ci.create_followup(
            &ctx.http,
            serenity::CreateInteractionResponseFollowup::new()
                .ephemeral(true)
                .content("Message deleted. Report resolved and thread archived."),
        )
        .await
        .ok();
        return Ok(());
    }

    let modal_id = cid_report_action_modal(action, report_id, target_id);
    let (title, components): (&str, Vec<serenity::CreateModalComponent>) = match action {
        "warn" | "kick" => (
            if action == "warn" {
                "⚠️ Warn User"
            } else {
                "👢 Kick User"
            },
            vec![crate::util::modal_input(
                "Reason for action",
                "reason",
                false,
                true,
                Some("What rule was violated?"),
                None,
            )],
        ),
        "dw" => (
            "🗑️ Delete & Warn",
            vec![crate::util::modal_input(
                "Reason for warning",
                "reason",
                false,
                true,
                Some("What was the issue with this message?"),
                None,
            )],
        ),
        "timeout" => (
            "⏱️ Timeout User",
            vec![
                crate::util::modal_input(
                    "Reason for timeout",
                    "reason",
                    false,
                    true,
                    Some("What did they do?"),
                    None,
                ),
                crate::util::preset_select("How long to mute", "duration", crate::util::DURATION_PRESETS, 3600),
            ],
        ),
        "ban" => (
            "🔨 Ban User",
            vec![
                crate::util::modal_input(
                    "Reason for ban",
                    "reason",
                    false,
                    true,
                    Some("Be specific about the violation"),
                    None,
                ),
                crate::util::preset_select("Ban length", "duration", crate::util::BAN_DURATION_PRESETS, 0),
                crate::util::single_checkbox("Appeals", "appealable", "Allow them to appeal", true),
            ],
        ),
        _ => {
            respond_ephemeral(ctx, ci, "Unknown action.").await;
            return Ok(());
        }
    };

    ci.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Modal(
            serenity::CreateModal::new(modal_id, title).components(components),
        ),
    )
    .await?;
    Ok(())
}
