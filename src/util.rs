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

/// Read a submitted text-input value out of a modal submission by `custom_id`.
/// Thin wrapper over [`crate::ui::read_text`] for the serenity-`next` modal model
/// (`FixedArray<ModalComponent>` of `Label`-wrapped inputs/selects).
pub fn modal_field<'a>(components: &'a [serenity::ModalComponent], id: &str) -> Option<&'a str> {
    crate::ui::read_text(components, id)
}

/// Build a `Label`-wrapped text input for a serenity-`next` modal. serenity `next`
/// only deserializes modal submissions whose fields are `Label`-wrapped, so every
/// modal we *send* must use this shape (not the legacy action-row text input).
pub fn modal_input(
    label: &str,
    custom_id: &str,
    paragraph: bool,
    required: bool,
    placeholder: Option<&str>,
    value: Option<&str>,
) -> serenity::CreateModalComponent<'static> {
    let style = if paragraph {
        serenity::InputTextStyle::Paragraph
    } else {
        serenity::InputTextStyle::Short
    };
    let mut input = serenity::CreateInputText::new(style, custom_id.to_string()).required(required);
    if let Some(p) = placeholder {
        input = input.placeholder(p.to_string());
    }
    if let Some(v) = value {
        input = input.value(v.to_string());
    }
    serenity::CreateModalComponent::Label(serenity::CreateLabel::input_text(label.to_string(), input))
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

/// Parse a human duration like `30s`, `15m`, `2h`, `7d`, `1w` (bare number = seconds)
/// into seconds. Returns `None` for empty/invalid input.
pub fn parse_duration_secs(s: &str) -> Option<i64> {
    let s = s.trim().to_lowercase();
    if s.is_empty() {
        return None;
    }
    let split = s.find(|c: char| c.is_alphabetic()).unwrap_or(s.len());
    let (num, unit) = s.split_at(split);
    let n: i64 = num.trim().parse().ok()?;
    if n < 0 {
        return None;
    }
    let mult = match unit.trim() {
        "" | "s" | "sec" | "secs" | "second" | "seconds" => 1,
        "m" | "min" | "mins" | "minute" | "minutes" => 60,
        "h" | "hr" | "hrs" | "hour" | "hours" => 3600,
        "d" | "day" | "days" => 86400,
        "w" | "wk" | "week" | "weeks" => 604800,
        _ => return None,
    };
    n.checked_mul(mult)
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
    modal: serenity::CreateModal<'_>,
) -> Result<(), crate::context::BotError> {
    use poise::Context as Ctx;
    match ctx {
        Ctx::Application(app) => app
            .interaction
            .create_response(
                &ctx.serenity_context().http,
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
        &ctx.http,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content(msg),
        ),
    )
    .await
    .ok();
}
