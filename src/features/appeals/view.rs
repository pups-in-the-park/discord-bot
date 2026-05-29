//! Appeal cards (the `#appeals` channel card + investigation-thread intro), the
//! resolved-card edit, and the appellant DMs.

use poise::serenity_prelude as serenity;

use crate::context::colours;
use crate::db::{Appeal, Infraction};
use crate::ids::cid_concern_btn;
use crate::ui::{self, Button, ButtonStyle, Container};

/// Post the staff-facing appeal card to `#appeals`; returns its message id.
pub async fn post_appeal_card(
    ctx: &serenity::Context,
    appeals_ch: serenity::ChannelId,
    appeal: &Appeal,
    infraction: &Infraction,
    appellant: serenity::UserId,
    reason: &str,
) -> anyhow::Result<serenity::Message> {
    let card = Container::new(vec![ui::text(format!(
        "**📝 New Appeal — #{}**\nUser: <@{}>\nInfraction: {} — {}\nAppeal reason: {}\nSubmitted: <t:{}:R>",
        appeal.id,
        appellant,
        infraction.kind,
        infraction.reason,
        reason,
        chrono::Utc::now().timestamp(),
    ))])
    .accent(colours::ORANGE.0);
    ui::send(&ctx.http, appeals_ch, &[card.into()]).await
}

/// Post the appeal detail card inside the private appeal thread.
pub async fn post_appeal_thread_intro(
    ctx: &serenity::Context,
    thread_id: serenity::ChannelId,
    appeal: &Appeal,
    infraction: &Infraction,
    appellant: serenity::UserId,
    reason: &str,
) {
    let card = Container::new(vec![ui::text(format!(
        "**Appeal #{} — {}**\nUser: <@{}>\nOriginal infraction: {} — {}\n\n**Appeal reason:**\n{}",
        appeal.id, infraction.kind, appellant, infraction.kind, infraction.reason, reason,
    ))])
    .accent(colours::BLURPLE.0);
    ui::send(&ctx.http, thread_id, &[card.into()]).await.ok();
}

/// Replace the `#appeals` card with an "accepted"/"denied" summary.
pub async fn mark_appeal_resolved(
    ctx: &serenity::Context,
    appeals_ch: serenity::ChannelId,
    card_message_id: serenity::MessageId,
    status: &str,
    appellant_id: &str,
    response: &str,
    moderator: serenity::UserId,
) {
    let accepted = status == "accepted";
    let updated_card = Container::new(vec![ui::text(format!(
        "**{} Appeal {}**\nUser: <@{}>\nResponse: {}\nResolved by: <@{}>",
        if accepted { "✅" } else { "❌" },
        if accepted { "Accepted" } else { "Denied" },
        appellant_id,
        response,
        moderator,
    ))])
    .accent(if accepted { colours::GREEN.0 } else { colours::RED.0 });
    ui::edit(&ctx.http, appeals_ch, card_message_id, &[updated_card.into()])
        .await
        .ok();
}

/// DM the appellant that their appeal was accepted/denied (with an optional rejoin invite).
pub async fn notify_appeal_resolved(
    http: &serenity::Http,
    user: &serenity::User,
    status: &str,
    response: &str,
    invite_url: Option<&str>,
) {
    let Ok(dm) = user.create_dm_channel(http).await else {
        return;
    };
    let mut text = format!(
        "**Your appeal has been {}**\n\nResponse: {}",
        if status == "accepted" { "accepted" } else { "denied" },
        response,
    );
    if let Some(url) = invite_url {
        text.push_str(&format!("\n\n[Click here to rejoin the server]({})", url));
    }
    let card = Container::new(vec![ui::text(text)])
        .accent(if status == "accepted" { colours::GREEN.0 } else { colours::RED.0 });
    ui::send(http, dm.id, &[card.into()]).await.ok();
}

/// DM the appellant that their appeal was denied, with a "Report a Concern" button.
pub async fn notify_appeal_denied(
    http: &serenity::Http,
    user: &serenity::User,
    guild_id: serenity::GuildId,
    response: &str,
    appeal_id: i64,
) {
    let Ok(dm) = user.create_dm_channel(http).await else {
        return;
    };
    let concern_btn = Button::new(
        cid_concern_btn("appeal", appeal_id, guild_id.get()),
        "Report a Concern",
        ButtonStyle::Secondary,
    )
    .emoji("🚩");
    let card = Container::new(vec![
        ui::text(format!(
            "**Your appeal has been denied**\n\nResponse: {}\n\nIf you believe this decision was unfair, you may report a concern.",
            response,
        )),
        ui::action_row(vec![concern_btn.into()]),
    ])
    .accent(colours::RED.0);
    ui::send(http, dm.id, &[card.into()]).await.ok();
}
