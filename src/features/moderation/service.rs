//! Non-UI moderation helpers shared across entry points. `send_action_dm` is the
//! one cross-feature export — slash commands, context menus, and the reports
//! "take action" flow all notify the target through it.

use poise::serenity_prelude as serenity;

use crate::context::colours;
use crate::ids::cid_appeal_btn;
use crate::ui::{self, Button, ButtonStyle, Container};

/// The moderation action a DM describes. Carries just enough to write copy that
/// leads with what the member needs to know (and, for timeouts, when it ends).
pub enum ModActionDm<'a> {
    Warn { reason: &'a str },
    Timeout { reason: &'a str, until: serenity::Timestamp },
    Kick { reason: &'a str },
    Ban { reason: &'a str },
    Untimeout,
    Unban,
}

impl ModActionDm<'_> {
    /// The accent colour for the DM card.
    fn accent(&self) -> u32 {
        match self {
            ModActionDm::Warn { .. } | ModActionDm::Timeout { .. } => colours::YELLOW.0,
            ModActionDm::Kick { .. } => colours::ORANGE.0,
            ModActionDm::Ban { .. } => colours::RED.0,
            ModActionDm::Untimeout | ModActionDm::Unban => colours::GREEN.0,
        }
    }

    /// Natural-language body, leading with the outcome and what to do next.
    fn body(&self, guild: &str) -> String {
        // Trim placeholder reasons so we don't print "Reason: No reason given".
        let reason_line = |reason: &str| {
            let r = reason.trim();
            if r.is_empty() || r.eq_ignore_ascii_case("no reason given") {
                String::new()
            } else {
                format!("\n**Why:** {r}")
            }
        };
        match self {
            ModActionDm::Warn { reason } => format!(
                "⚠️ **You've received a warning in {guild}.**{}\n\nPlease take a moment to review the \
                 server rules — further issues may lead to a timeout or removal.",
                reason_line(reason)
            ),
            ModActionDm::Timeout { reason, until } => format!(
                "⏱️ **You've been timed out in {guild}.**\nYou won't be able to send messages, react, \
                 or speak in voice until <t:{ts}:F> (<t:{ts}:R>).{}",
                reason_line(reason),
                ts = until.unix_timestamp(),
            ),
            ModActionDm::Kick { reason } => format!(
                "👢 **You've been removed from {guild}.**{}\n\nYou're welcome to rejoin with a valid \
                 invite, but please follow the rules.",
                reason_line(reason)
            ),
            ModActionDm::Ban { reason } => format!(
                "🔨 **You've been banned from {guild}.**{}",
                reason_line(reason)
            ),
            ModActionDm::Untimeout => format!(
                "✅ **Your timeout in {guild} has been lifted.** You can take part again — thanks for \
                 your patience."
            ),
            ModActionDm::Unban => format!(
                "✅ **Your ban from {guild} has been lifted.** You're welcome to rejoin with a valid invite."
            ),
        }
    }
}

/// DM a user about a moderation action in natural language, with an optional
/// "Appeal this action" button when the infraction is appealable. Best-effort:
/// silently does nothing if the user's DMs are closed.
pub async fn send_action_dm(
    http: &serenity::Http,
    user: &serenity::User,
    guild_id: serenity::GuildId,
    action: ModActionDm<'_>,
    appeal_info: Option<(i64, serenity::GuildId)>,
) {
    let dm_channel = match user.create_dm_channel(http).await {
        Ok(c) => c,
        Err(_) => return,
    };

    // Prefer the guild's real name over a bare id.
    let guild = guild_id
        .to_partial_guild(http)
        .await
        .map(|g| g.name.to_string())
        .unwrap_or_else(|_| "the server".to_string());

    let mut body = action.body(&guild);

    let mut components: Vec<ui::Component> = Vec::new();
    if let Some((infraction_id, gid)) = appeal_info {
        body.push_str("\n\nIf you believe this was a mistake, you can appeal below.");
        components.push(ui::text(body));
        components.push(ui::action_row(vec![Button::new(
            cid_appeal_btn(infraction_id, gid.get()),
            "Appeal this action",
            ButtonStyle::Secondary,
        )
        .emoji("📝")
        .into()]));
    } else {
        components.push(ui::text(body));
    }

    let card = Container::new(components).accent(action.accent());
    ui::send(http, dm_channel.id, &[card.into()]).await.ok();
}
