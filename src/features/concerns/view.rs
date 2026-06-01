//! Concern cards: the `#concerns` (admin-only) card and its "reviewed" edit.

use poise::serenity_prelude as serenity;

use crate::context::colours;
use crate::db::Concern;
use crate::ids::cid_concern_reviewed;
use crate::ui::{self, Button, ButtonStyle, Container, Spacing};

/// Post a concern to the admin `#concerns` channel with a "Mark as Reviewed" button.
pub async fn post_concern_card(
    ctx: &serenity::Context,
    concerns_ch: serenity::ChannelId,
    concern: &Concern,
    appellant: serenity::UserId,
) {
    let reviewed_btn = Button::new(
        cid_concern_reviewed(concern.id),
        "Mark as Reviewed",
        ButtonStyle::Secondary,
    )
    .emoji("✅");
    let card = Container::new(vec![
        ui::text(format!(
            "**🚩 Concern Raised — #{}**\nUser: <@{}>\nKind: {}\nReason: {}",
            concern.id, appellant, concern.kind, concern.reason,
        )),
        ui::separator(false, Spacing::Small),
        ui::action_row(vec![reviewed_btn.into()]),
    ])
    .accent(colours::ORANGE.0);
    ui::send(&ctx.http, concerns_ch, &[card.into()]).await.ok();
}

/// Replace a concern card with a "reviewed" summary.
pub async fn mark_concern_reviewed(
    ctx: &serenity::Context,
    channel_id: serenity::ChannelId,
    message_id: serenity::MessageId,
    reviewer: serenity::UserId,
) {
    let updated = Container::new(vec![ui::text(format!(
        "**✅ Concern Reviewed**\nReviewed by <@{}>",
        reviewer,
    ))])
    .accent(colours::GREEN.0);
    ui::edit(&ctx.http, channel_id, message_id, &[updated.into()]).await.ok();
}
