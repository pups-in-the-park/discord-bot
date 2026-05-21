//! Shared, non-UI utilities: infraction/duration formatting, name sanitisation,
//! and modal helpers. CV2 component construction lives in the typed [`crate::ui`]
//! kit; staff checks live in [`crate::permissions`].

use poise::serenity_prelude as serenity;

/// Formats up to 15 infractions into display lines for a history embed.
pub fn format_infraction_history(infractions: &[crate::db::Infraction]) -> Vec<String> {
    infractions
        .iter()
        .take(15)
        .map(|i| {
            let kind_emoji = match i.kind.as_str() {
                "warn" => "⚠️",
                "timeout" => "⏱️",
                "kick" => "👢",
                "ban" => "🔨",
                "unban" | "untimeout" => "✅",
                "blocklist" => "🚫",
                _ => "•",
            };
            let duration = i
                .duration_secs
                .map(|d| format!(" ({})", format_duration(d)))
                .unwrap_or_default();
            format!(
                "{} **{}{}** — {}\n<@{}> · <t:{}:R>",
                kind_emoji,
                i.kind,
                duration,
                i.reason,
                i.moderator_id,
                chrono::DateTime::parse_from_rfc3339(&i.created_at)
                    .map(|dt| dt.timestamp())
                    .unwrap_or(0),
            )
        })
        .collect()
}

/// Read a submitted text-input value out of a modal interaction by `custom_id`.
pub fn modal_field<'a>(components: &'a [serenity::ActionRow], id: &str) -> Option<&'a str> {
    components.iter().flat_map(|row| &row.components).find_map(|c| {
        if let serenity::ActionRowComponent::InputText(t) = c {
            if t.custom_id == id {
                return t.value.as_deref();
            }
        }
        None
    })
}

/// Lowercase, replace non-alphanumerics with `-`, trim, and cap at 24 chars.
pub fn sanitise_name(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .take(24)
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Format a duration in seconds into a human-readable string.
pub fn format_duration(secs: i64) -> String {
    if secs < 60 {
        format!("{} seconds", secs)
    } else if secs < 3600 {
        format!("{} minutes", secs / 60)
    } else if secs < 86400 {
        format!("{} hours", secs / 3600)
    } else {
        format!("{} days", secs / 86400)
    }
}

/// Respond to an application command (slash or context-menu) with a modal.
/// Poise's `Context` enum has no built-in `send_modal`; we extract the raw
/// `CommandInteraction` from the `Application` variant and call Serenity directly.
pub async fn modal_response(
    ctx: crate::context::Context<'_>,
    modal: serenity::CreateModal,
) -> Result<(), crate::context::BotError> {
    use poise::Context as Ctx;
    match ctx {
        Ctx::Application(app) => app
            .interaction
            .create_response(
                ctx.serenity_context(),
                serenity::CreateInteractionResponse::Modal(modal),
            )
            .await
            .map_err(crate::context::BotError::Serenity),
        _ => Err(crate::context::BotError::user(
            "This command must be used as a slash command.",
        )),
    }
}

/// Reply to a component interaction with a short ephemeral text message.
pub async fn respond_ephemeral(
    ctx: &serenity::Context,
    ci: &serenity::ComponentInteraction,
    msg: &str,
) {
    ci.create_response(
        ctx,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content(msg),
        ),
    )
    .await
    .ok();
}
