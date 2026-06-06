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
    /// `until` is the unix timestamp a temporary ban lifts, or `None` if permanent.
    Ban { reason: &'a str, until: Option<i64> },
    Untimeout,
    Unban,
    /// Blocked from a feature (tickets/reports/concerns/appeals). `expires_ts` is a
    /// unix timestamp, or `None` for a permanent block.
    Blocklist { reason: &'a str, feature: &'a str, expires_ts: Option<i64> },
}

impl ModActionDm<'_> {
    /// The accent colour for the DM card.
    fn accent(&self) -> u32 {
        match self {
            ModActionDm::Warn { .. } | ModActionDm::Timeout { .. } => colours::YELLOW.0,
            ModActionDm::Kick { .. } | ModActionDm::Blocklist { .. } => colours::ORANGE.0,
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
            ModActionDm::Ban { reason, until } => {
                let duration = match until {
                    Some(ts) => format!("\nThis ban lifts <t:{ts}:F> (<t:{ts}:R>)."),
                    None => String::new(),
                };
                format!(
                    "🔨 **You've been banned from {guild}.**{}{}",
                    reason_line(reason),
                    duration
                )
            }
            ModActionDm::Untimeout => format!(
                "✅ **Your timeout in {guild} has been lifted.** You can take part again — thanks for \
                 your patience."
            ),
            ModActionDm::Unban => format!(
                "✅ **Your ban from {guild} has been lifted.** You're welcome to rejoin with a valid invite."
            ),
            ModActionDm::Blocklist { reason, feature, expires_ts } => {
                let expiry = match expires_ts {
                    Some(ts) => format!("\nThis block lifts <t:{ts}:R>."),
                    None => "\nThis block does not expire.".to_string(),
                };
                format!(
                    "🚫 **You've been blocked from {feature} in {guild}.**{}{}",
                    reason_line(reason),
                    expiry
                )
            }
        }
    }
}

/// Compute `(duration_secs, expires_at_rfc3339, until_unix)` for a ban of `secs`
/// seconds. `secs <= 0` is a permanent ban (all `None`). The RFC3339 string is what
/// gets stored on the infraction; the unix timestamp is for DM/log copy.
pub fn ban_expiry(secs: i64) -> (Option<i64>, Option<String>, Option<i64>) {
    if secs <= 0 {
        return (None, None, None);
    }
    let until_unix = serenity::Timestamp::now().unix_timestamp() + secs;
    match serenity::Timestamp::from_unix_timestamp(until_unix) {
        Ok(ts) => (Some(secs), Some(ts.to_rfc3339()), Some(until_unix)),
        Err(_) => (Some(secs), None, None),
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

    // A DM the member must act on (Appeal button) is interactive → CV2 card. A DM
    // that's a pure notice (no appeal) is read-only → embed. One paradigm per
    // message, per the UI conventions.
    if let Some((infraction_id, gid)) = appeal_info {
        body.push_str("\n\nIf you believe this was a mistake, you can appeal below.");
        let card = Container::new(vec![
            ui::text(body),
            ui::action_row(vec![Button::new(
                cid_appeal_btn(infraction_id, gid.get()),
                "Appeal this action",
                ButtonStyle::Secondary,
            )
            .emoji("📝")
            .into()]),
        ])
        .accent(action.accent());
        ui::send(http, dm_channel.id, &[card.into()]).await.ok();
    } else {
        let embed = serenity::CreateEmbed::new()
            .colour(serenity::Colour::new(action.accent()))
            .description(body);
        dm_channel
            .id
            .widen()
            .send_message(http, serenity::CreateMessage::new().embed(embed))
            .await
            .ok();
    }
}
