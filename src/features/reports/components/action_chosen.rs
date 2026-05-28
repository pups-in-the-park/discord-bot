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
    let target_id: u64 = report.target_user_id.parse().unwrap_or(0);

    // "del" is handled inline — no modal needed.
    if action == "del" {
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
        data.db
            .resolve_report(report_id, "action_taken", &ci.user.id.to_string())
            .await?;
        ci.channel_id
            .edit_thread(ctx, serenity::EditThread::new().archived(true))
            .await
            .ok();
        super::super::view::notify_reporter_action_taken(ctx, &report.reporter_id).await;
        respond_ephemeral(ctx, ci, "Message deleted. Report resolved and thread archived.").await;
        return Ok(());
    }

    let modal_id = cid_report_action_modal(action, report_id, target_id);
    let (title, components): (&str, Vec<serenity::CreateActionRow>) = match action {
        "warn" | "kick" => (
            if action == "warn" {
                "⚠️ Warn User"
            } else {
                "👢 Kick User"
            },
            vec![serenity::CreateActionRow::InputText(
                serenity::CreateInputText::new(
                    serenity::InputTextStyle::Short,
                    "Reason for action",
                    "reason",
                )
                .placeholder("What rule was violated?")
                .required(true),
            )],
        ),
        "dw" => (
            "🗑️ Delete & Warn",
            vec![serenity::CreateActionRow::InputText(
                serenity::CreateInputText::new(
                    serenity::InputTextStyle::Short,
                    "Reason for warning",
                    "reason",
                )
                .placeholder("What was the issue with this message?")
                .required(true),
            )],
        ),
        "timeout" => (
            "⏱️ Timeout User",
            vec![
                serenity::CreateActionRow::InputText(
                    serenity::CreateInputText::new(
                        serenity::InputTextStyle::Short,
                        "Reason for timeout",
                        "reason",
                    )
                    .placeholder("What did they do?")
                    .required(true),
                ),
                serenity::CreateActionRow::InputText(
                    serenity::CreateInputText::new(
                        serenity::InputTextStyle::Short,
                        "How long to mute",
                        "duration",
                    )
                    .placeholder("60s / 5min / 10min / 1h / 1d / 1w")
                    .value("1h")
                    .required(true),
                ),
            ],
        ),
        "ban" => (
            "🔨 Ban User",
            vec![
                serenity::CreateActionRow::InputText(
                    serenity::CreateInputText::new(
                        serenity::InputTextStyle::Short,
                        "Reason for ban",
                        "reason",
                    )
                    .placeholder("Be specific about the violation")
                    .required(true),
                ),
                serenity::CreateActionRow::InputText(
                    serenity::CreateInputText::new(
                        serenity::InputTextStyle::Short,
                        "Allow appeals (yes/no)",
                        "appealable",
                    )
                    .placeholder("Can they appeal this ban?")
                    .value("yes")
                    .required(false),
                ),
            ],
        ),
        _ => {
            respond_ephemeral(ctx, ci, "Unknown action.").await;
            return Ok(());
        }
    };

    ci.create_response(
        ctx,
        serenity::CreateInteractionResponse::Modal(
            serenity::CreateModal::new(modal_id, title).components(components),
        ),
    )
    .await?;
    Ok(())
}
