//! Report message cards: the `#reports` channel card, the investigation-thread
//! detail/action cards, and the reporter DM follow-ups.

use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::{colours, BotData};
use crate::db::Report;
use crate::ids;
use crate::ui::{self, Button, ButtonStyle, Container, Spacing};

/// Post the initial report card to `#reports` and remember its message id.
pub async fn post_report_card(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    channel_id: serenity::ChannelId,
    report: &Report,
    target: &serenity::User,
) {
    let mut description = format!(
        "**📊 New Report**\nReporter: <@{}>\nTarget: <@{}> ({})",
        report.reporter_id, report.target_user_id, target.name,
    );
    if let Some(ref content) = report.message_content {
        description.push_str(&format!("\n\n**Message:**\n> {}", content));
    }
    if let Some(ref url) = report.message_url {
        description.push_str(&format!("\n[Jump]({url})"));
    }
    if let Some(ref reason) = report.reason {
        description.push_str(&format!("\n\n**Reason:** {}", reason));
    }

    let investigate_btn = Button::new(
        ids::cid_report_investigate(report.id),
        "Investigate",
        ButtonStyle::Primary,
    )
    .emoji("🔍");
    let dismiss_btn = Button::new(ids::cid_report_dismiss(report.id), "Dismiss", ButtonStyle::Danger)
        .emoji("❌");

    let card = Container::new(vec![
        ui::text(description),
        ui::separator(false, Spacing::Small),
        ui::action_row(vec![investigate_btn.into(), dismiss_btn.into()]),
    ])
    .accent(colours::ORANGE.0);

    if let Ok(msg) = ui::send(&ctx.http, channel_id, &[card.into()]).await {
        data.db
            .set_report_card(report.id, &msg.id.to_string())
            .await
            .ok();
    }
}

/// Detail + action cards posted into a freshly-created investigation thread.
pub async fn post_investigation_cards(
    ctx: &serenity::Context,
    thread_id: serenity::ChannelId,
    report: &Report,
) {
    let mut detail = format!(
        "**📋 Report Investigation**\nReporter: <@{}>\nTarget: <@{}>",
        report.reporter_id, report.target_user_id,
    );
    if let Some(ref content) = report.message_content {
        detail.push_str(&format!("\n\n**Reported message:**\n> {}", content));
    }
    if let Some(ref url) = report.message_url {
        detail.push_str(&format!("\n[Jump to message]({})", url));
    }
    if let Some(ref reason) = report.reason {
        detail.push_str(&format!("\n\nReason: {}", reason));
    }
    detail.push_str(&format!("\nSubmitted: {}", report.created_at));

    let detail_card = Container::new(vec![ui::text(detail)]).accent(colours::ORANGE.0);
    ui::send(&ctx.http, thread_id, &[detail_card.into()]).await.ok();

    let take_action_btn = Button::new(
        ids::cid_report_action_select(report.id),
        "Take Action",
        ButtonStyle::Primary,
    )
    .emoji("⚡");
    let dismiss_btn = Button::new(
        ids::cid_report_thread_dismiss(report.id),
        "Dismiss",
        ButtonStyle::Danger,
    )
    .emoji("❌");
    let action_card = Container::new(vec![
        ui::text("**Select an action for this report:**"),
        ui::separator(false, Spacing::Small),
        ui::action_row(vec![take_action_btn.into(), dismiss_btn.into()]),
    ])
    .accent(colours::BLURPLE.0);
    ui::send(&ctx.http, thread_id, &[action_card.into()]).await.ok();
}

/// Replace the `#reports` card with a "dismissed" summary.
pub async fn mark_card_dismissed(
    ctx: &serenity::Context,
    channel_id: serenity::ChannelId,
    message_id: serenity::MessageId,
    moderator: serenity::UserId,
) {
    let card = Container::new(vec![ui::text(format!(
        "**📊 Report — Dismissed**\nDismissed by <@{}> · <t:{}:R>",
        moderator,
        chrono::Utc::now().timestamp(),
    ))])
    .accent(colours::GREY.0);
    ui::edit(&ctx.http, channel_id, message_id, &[card.into()]).await.ok();
}

/// DM the reporter that no action was taken, with a "Report a Concern" button.
pub async fn notify_reporter_no_action(ctx: &serenity::Context, report: &Report) {
    let Ok(uid) = report.reporter_id.parse::<u64>() else {
        return;
    };
    let uid = serenity::UserId::new(uid);
    let Ok(user) = uid.to_user(ctx).await else {
        return;
    };
    let Ok(dm) = user.create_dm_channel(ctx).await else {
        return;
    };

    let concern_btn = Button::new(
        ids::cid_concern_btn("report", report.id, report.guild_id.parse::<u64>().unwrap_or(0)),
        "Report a Concern",
        ButtonStyle::Secondary,
    )
    .emoji("🚩");

    let card = Container::new(vec![
        ui::text("**Report Update**\nThank you for your report. After review, no action was taken at this time."),
        ui::action_row(vec![concern_btn.into()]),
    ])
    .accent(colours::GREY.0);

    ui::send(&ctx.http, dm.id, &[card.into()]).await.ok();
}

/// DM the reporter that action was taken.
pub async fn notify_reporter_action_taken(ctx: &serenity::Context, reporter_id: &str) {
    let Ok(uid) = reporter_id.parse::<u64>() else {
        return;
    };
    let uid = serenity::UserId::new(uid);
    let Ok(user) = uid.to_user(ctx).await else {
        return;
    };
    let Ok(dm) = user.create_dm_channel(ctx).await else {
        return;
    };

    let card = Container::new(vec![ui::text(
        "**Report Update**\nThank you for your report. Action has been taken.",
    )])
    .accent(colours::GREEN.0);
    ui::send(&ctx.http, dm.id, &[card.into()]).await.ok();
}
