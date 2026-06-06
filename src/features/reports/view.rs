//! Report message cards: the `#reports` channel card, the investigation-thread
//! detail/action cards, and the reporter DM follow-ups.

use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::{colours, BotData};
use crate::db::Report;
use crate::ids;
use crate::ui::{self, Button, ButtonStyle, Container, Spacing};

/// The profile/user-report modal: a checkbox group of profile parts plus an
/// optional free-text reason — one surface, mirroring Discord's own report flow.
pub(crate) fn build_report_modal(modal_id: String, title: String) -> serenity::CreateModal<'static> {
    let parts = serenity::CreateCheckboxGroup::new(
        crate::ids::REPORT_PARTS_FIELD,
        vec![
            serenity::CreateCheckboxGroupOption::new("Username", "username"),
            serenity::CreateCheckboxGroupOption::new("Display name", "display"),
            serenity::CreateCheckboxGroupOption::new("Avatar", "avatar"),
            serenity::CreateCheckboxGroupOption::new("Banner", "banner"),
            serenity::CreateCheckboxGroupOption::new("Bio / About me", "bio"),
            serenity::CreateCheckboxGroupOption::new("Status / activity", "status"),
            serenity::CreateCheckboxGroupOption::new("Overall behaviour", "behaviour"),
        ],
    )
    .min_values(1);

    serenity::CreateModal::new(modal_id, title).components(vec![
        serenity::CreateModalComponent::Label(serenity::CreateLabel::checkbox_group(
            "What are you reporting?",
            parts,
        )),
        crate::util::modal_input(
            "Why are you reporting this?",
            crate::ids::REPORT_REASON_FIELD,
            true,
            false,
            Some("Describe specific behaviours or violations…"),
            None,
        ),
    ])
}

/// Extract the channel id from a `…/channels/{guild}/{channel}/{message}` jump URL.
fn parse_channel_from_url(url: &str) -> Option<u64> {
    let segs: Vec<&str> = url.trim_end_matches('/').split('/').collect();
    let n = segs.len();
    (n >= 2).then(|| segs[n - 2].parse().ok()).flatten()
}

/// Human-readable labels for the stored profile-part codes (see the report modal).
pub(crate) fn profile_parts_label(codes: &str) -> String {
    codes
        .split(',')
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(|c| match c {
            "username" => "Username",
            "display" => "Display name",
            "avatar" => "Avatar",
            "banner" => "Banner",
            "bio" => "Bio/About me",
            "status" => "Status/activity",
            "behaviour" => "Overall behaviour",
            other => other,
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// A compact context block about the reported user — account age, server join,
/// role count, prior-infraction count, and a staff-member warning. Best-effort:
/// missing pieces are simply omitted.
async fn target_context(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    guild_id: serenity::GuildId,
    target_id: serenity::UserId,
) -> String {
    let mut lines = vec![format!("**Account created:** <t:{}:R>", target_id.created_at().unix_timestamp())];

    if let Ok(member) = guild_id.member(&ctx.http, target_id).await {
        if let Some(joined) = member.joined_at {
            lines.push(format!("**Joined server:** <t:{}:R>", joined.unix_timestamp()));
        }
        lines.push(format!("**Roles:** {}", member.roles.len()));
    }

    if let Ok(infractions) = data.db.get_infractions(&guild_id.to_string(), &target_id.to_string()).await {
        let active = infractions.iter().filter(|i| i.active).count();
        lines.push(format!(
            "**History:** {} prior infraction(s), {active} active",
            infractions.len()
        ));
    }

    if crate::permissions::is_mod_staff(ctx, data, guild_id, target_id).await {
        lines.push("⚠️ **Target is a staff member** — hand off to another moderator.".to_string());
    }

    lines.join("\n")
}

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
    if let Some(ref parts) = report.profile_parts {
        if !parts.is_empty() {
            description.push_str(&format!("\n**Reporting:** {}", profile_parts_label(parts)));
        }
    }
    if let Some(ref reason) = report.reason {
        description.push_str(&format!("\n\n**Reason:** {}", reason));
    }
    if let Ok(gid) = report.guild_id.parse::<u64>() {
        let ctx_block =
            target_context(ctx, data, serenity::GuildId::new(gid), target.id).await;
        if !ctx_block.is_empty() {
            description.push_str(&format!("\n\n{ctx_block}"));
        }
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
    data: &Arc<BotData>,
    thread_id: serenity::ChannelId,
    report: &Report,
) {
    let mut detail = format!(
        "**📋 Report Investigation**\nReporter: <@{}>\nTarget: <@{}>",
        report.reporter_id, report.target_user_id,
    );
    if let Some(ref parts) = report.profile_parts {
        if !parts.is_empty() {
            detail.push_str(&format!("\n**Reporting:** {}", profile_parts_label(parts)));
        }
    }
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

    if let (Ok(gid), Ok(uid)) =
        (report.guild_id.parse::<u64>(), report.target_user_id.parse::<u64>())
    {
        let ctx_block =
            target_context(ctx, data, serenity::GuildId::new(gid), serenity::UserId::new(uid)).await;
        if !ctx_block.is_empty() {
            detail.push_str(&format!("\n\n{ctx_block}"));
        }
        // Fuller infraction history lives in the thread (not the compact card).
        if let Ok(infractions) = data
            .db
            .get_infractions(&report.guild_id, &report.target_user_id)
            .await
        {
            let lines = crate::util::format_infraction_history(&infractions);
            if !lines.is_empty() {
                detail.push_str("\n\n**Recent history:**\n");
                detail.push_str(&lines.join("\n"));
            }
        }
    }

    let detail_card = Container::new(vec![ui::text(detail)]).accent(colours::ORANGE.0);
    ui::send(&ctx.http, thread_id, &[detail_card.into()]).await.ok();

    // Forward the reported message into the thread so mods see it natively, with
    // attachments and embeds intact. The quoted snapshot above is the fallback if
    // this fails (e.g. the message was deleted).
    if let (Some(url), Some(mid)) = (&report.message_url, &report.message_id) {
        if let (Some(chan), Ok(mid), Ok(gid)) = (
            parse_channel_from_url(url),
            mid.parse::<u64>(),
            report.guild_id.parse::<u64>(),
        ) {
            ui::forward_message(
                &ctx.http,
                thread_id,
                serenity::ChannelId::new(chan),
                serenity::MessageId::new(mid),
                serenity::GuildId::new(gid),
            )
            .await
            .ok();
        }
    }

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
