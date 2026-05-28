use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::util::respond_ephemeral;

/// "Dismiss" button on a report card (`from_thread = false`) or inside the
/// investigation thread (`from_thread = true`).
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
    report_id: i64,
    from_thread: bool,
) -> Result<(), anyhow::Error> {
    let Some(guild_id) = ci.guild_id else {
        return Ok(());
    };
    if !crate::permissions::is_mod_staff(ctx, data, guild_id, ci.user.id).await {
        respond_ephemeral(ctx, ci, "Only staff can manage reports.").await;
        return Ok(());
    }

    let report = data
        .db
        .get_report_by_id(report_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Report not found"))?;

    if report.status != "open" {
        respond_ephemeral(ctx, ci, "This report has already been resolved.").await;
        return Ok(());
    }

    data.db
        .resolve_report(report_id, "no_action", &ci.user.id.to_string())
        .await?;

    if let Ok(guild_cfg) = data.db.get_or_create_guild(&guild_id.to_string()).await {
        if let (Some(card_msg_str), Some(reports_ch_str)) =
            (&report.card_message_id, &guild_cfg.reports_channel_id)
        {
            if let (Ok(msg_id), Ok(ch_id)) =
                (card_msg_str.parse::<u64>(), reports_ch_str.parse::<u64>())
            {
                super::super::view::mark_card_dismissed(
                    ctx,
                    serenity::ChannelId::new(ch_id),
                    serenity::MessageId::new(msg_id),
                    ci.user.id,
                )
                .await;
            }
        }
    }

    if from_thread {
        ci.channel_id
            .edit_thread(ctx, serenity::EditThread::new().archived(true))
            .await
            .ok();
    }

    super::super::view::notify_reporter_no_action(ctx, &report).await;

    respond_ephemeral(ctx, ci, "Report dismissed. Reporter has been notified.").await;
    Ok(())
}
