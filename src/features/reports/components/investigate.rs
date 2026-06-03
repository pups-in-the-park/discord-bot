use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::util::respond_ephemeral;

/// "Investigate" button: open a private thread and post the investigation cards.
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
    // Already investigated — point to the existing thread rather than make a duplicate.
    if let Some(tid) = report.thread_id.as_deref().and_then(|s| s.parse::<u64>().ok()) {
        respond_ephemeral(ctx, ci, &format!("This report is already being investigated: <#{tid}>")).await;
        return Ok(());
    }

    // Creating the thread, forwarding the message, and posting the context cards far
    // exceeds Discord's 3-second response window, so acknowledge the click first and
    // send the confirmation as a follow-up.
    ci.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Defer(
            serenity::CreateInteractionResponseMessage::new().ephemeral(true),
        ),
    )
    .await?;

    let guild_cfg = data.db.get_or_create_guild(&guild_id.to_string()).await?;
    let reports_ch = guild_cfg
        .reports_channel_id
        .and_then(|s| s.parse::<u64>().ok())
        .map(serenity::ChannelId::new)
        .ok_or_else(|| anyhow::anyhow!("Reports channel not configured"))?;

    let target_user = serenity::UserId::new(report.target_user_id.parse::<u64>().unwrap_or(0))
        .to_user(ctx)
        .await
        .ok();
    let target_name = target_user.as_ref().map(|u| u.name.as_str()).unwrap_or("user");
    let thread_name = format!(
        "report-{:04}-{}",
        report_id,
        crate::util::sanitise_name(target_name)
    );

    let thread = reports_ch
        .create_thread(
            &ctx.http,
            serenity::CreateThread::new(thread_name)
                .kind(serenity::ChannelType::PrivateThread)
                .auto_archive_duration(serenity::AutoArchiveDuration::OneWeek)
                .invitable(false),
        )
        .await?;

    data.db
        .set_report_thread(report_id, &thread.id.to_string())
        .await?;

    ctx.http
        .add_thread_channel_member(thread.id, ci.user.id)
        .await
        .ok();

    super::super::view::post_investigation_cards(
        ctx,
        data,
        serenity::ChannelId::new(thread.id.get()),
        &report,
    )
    .await;

    ci.create_followup(
        &ctx.http,
        serenity::CreateInteractionResponseFollowup::new()
            .ephemeral(true)
            .content(format!("Investigation thread created: <#{}>", thread.id)),
    )
    .await?;
    Ok(())
}
