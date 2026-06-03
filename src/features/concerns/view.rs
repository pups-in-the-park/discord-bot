//! Concern cards: the `#concerns` (admin-only) card and its "reviewed" edit.

use poise::serenity_prelude as serenity;

use crate::context::colours;
use crate::db::Concern;
use crate::ids::cid_concern_reviewed;
use crate::ui::{self, Button, ButtonStyle, Container, Spacing};

/// Readable label for a concern's source kind.
fn kind_label(kind: &str) -> &str {
    match kind {
        "appeal" => "Denied appeal",
        "report" => "Dismissed report (no action)",
        other => other,
    }
}

/// The concern's body text (shared by the open card and the reviewed summary so
/// review never destroys the original content).
fn concern_body(concern: &Concern) -> String {
    let mut s = format!(
        "**🚩 Concern — #{}**\nFrom: <@{}>\nAbout: {}",
        concern.id, concern.user_id, kind_label(&concern.kind),
    );
    if let Some(ref m) = concern.target_moderator_id {
        s.push_str(&format!("\nModerator: <@{m}>"));
    }
    s.push_str(&format!("\n\n**Concern:**\n{}", concern.reason));
    s
}

/// Post a concern to the admin `#concerns` channel with a "Mark as Reviewed" button.
pub async fn post_concern_card(
    ctx: &serenity::Context,
    concerns_ch: serenity::ChannelId,
    concern: &Concern,
) {
    let reviewed_btn = Button::new(
        cid_concern_reviewed(concern.id),
        "Mark as Reviewed",
        ButtonStyle::Secondary,
    )
    .emoji("✅");
    let card = Container::new(vec![
        ui::text(concern_body(concern)),
        ui::separator(false, Spacing::Small),
        ui::action_row(vec![reviewed_btn.into()]),
    ])
    .accent(colours::ORANGE.0);
    ui::send(&ctx.http, concerns_ch, &[card.into()]).await.ok();
}

/// Re-render a concern card as reviewed — retains the original body and appends a
/// reviewed footer (who + when) instead of discarding the concern.
pub async fn mark_concern_reviewed(
    ctx: &serenity::Context,
    channel_id: serenity::ChannelId,
    message_id: serenity::MessageId,
    concern: &Concern,
) {
    let mut body = concern_body(concern);
    body.push_str("\n");
    let footer = match (&concern.reviewed_by, &concern.reviewed_at) {
        (Some(by), Some(at)) => {
            // SQLite `datetime('now')` -> "YYYY-MM-DD HH:MM:SS" (UTC, no offset).
            let ts = chrono::NaiveDateTime::parse_from_str(at, "%Y-%m-%d %H:%M:%S")
                .map(|d| d.and_utc().timestamp())
                .unwrap_or(0);
            format!("\n✅ **Reviewed** by <@{by}> · <t:{ts}:R>")
        }
        (Some(by), None) => format!("\n✅ **Reviewed** by <@{by}>"),
        _ => "\n✅ **Reviewed**".to_string(),
    };
    body.push_str(&footer);

    let updated = Container::new(vec![ui::text(body)]).accent(colours::GREEN.0);
    ui::edit(&ctx.http, channel_id, message_id, &[updated.into()]).await.ok();
}
