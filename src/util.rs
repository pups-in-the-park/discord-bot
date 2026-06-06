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

/// Preset timeout durations offered in moderation/report modals, as `(label, seconds)`.
/// A select of these replaces free-text duration entry (a *choice* is a select, per
/// the UI conventions — and it avoids the seconds-vs-`"5min"` parse mismatch).
const DURATION_PRESETS: &[(&str, i64)] = &[
    ("60 seconds", 60),
    ("5 minutes", 300),
    ("10 minutes", 600),
    ("1 hour", 3600),
    ("1 day", 86_400),
    ("1 week", 604_800),
];

/// Preset "delete recent messages" windows for a ban modal, as `(label, seconds)`.
const DELETE_WINDOW_PRESETS: &[(&str, i64)] = &[
    ("Don't delete any", 0),
    ("Last 1 hour", 3600),
    ("Last 6 hours", 21_600),
    ("Last 24 hours", 86_400),
    ("Last 3 days", 259_200),
    ("Last 7 days", 604_800),
];

/// Build a `Label`-wrapped single-select from `(label, seconds)` presets, pre-selecting
/// the option whose value equals `default_secs`. Option values are the second-counts;
/// read the choice back with [`modal_secs`].
fn secs_select(
    label: &str,
    custom_id: &str,
    presets: &'static [(&'static str, i64)],
    default_secs: i64,
) -> serenity::CreateModalComponent<'static> {
    let options: Vec<serenity::CreateSelectMenuOption> = presets
        .iter()
        .map(|(lbl, secs)| {
            serenity::CreateSelectMenuOption::new(*lbl, secs.to_string())
                .default_selection(*secs == default_secs)
        })
        .collect();
    let menu = serenity::CreateSelectMenu::new(
        custom_id.to_string(),
        serenity::CreateSelectMenuKind::String { options: options.into() },
    )
    .min_values(1)
    .max_values(1)
    .required(true);
    serenity::CreateModalComponent::Label(serenity::CreateLabel::select_menu(label.to_string(), menu))
}

/// A `Label`-wrapped single-select of preset timeout durations for a modal.
/// `default_secs` pre-selects its matching option; read back with [`modal_secs`].
pub fn duration_select(
    label: &str,
    custom_id: &str,
    default_secs: i64,
) -> serenity::CreateModalComponent<'static> {
    secs_select(label, custom_id, DURATION_PRESETS, default_secs)
}

/// A `Label`-wrapped single-select of preset message-deletion windows for a ban modal.
/// `default_secs` pre-selects its matching option; read back with [`modal_secs`].
pub fn delete_window_select(
    label: &str,
    custom_id: &str,
    default_secs: i64,
) -> serenity::CreateModalComponent<'static> {
    secs_select(label, custom_id, DELETE_WINDOW_PRESETS, default_secs)
}

/// Ban-length presets for the temporary-ban dropdown. `0` = permanent.
const BAN_DURATION_PRESETS: &[(&str, i64)] = &[
    ("Permanent", 0),
    ("1 day", 86_400),
    ("3 days", 259_200),
    ("7 days", 604_800),
    ("14 days", 1_209_600),
    ("30 days", 2_592_000),
];

/// A `Label`-wrapped single-select of ban-length presets (0 = permanent).
/// Read back with [`modal_secs`] (0 means a permanent ban).
pub fn ban_duration_select(
    label: &str,
    custom_id: &str,
    default_secs: i64,
) -> serenity::CreateModalComponent<'static> {
    secs_select(label, custom_id, BAN_DURATION_PRESETS, default_secs)
}

/// A `Label`-wrapped single-select built from arbitrary `(label, value)` integer
/// presets, pre-selecting `default`. Read back with [`modal_secs`]. Use this to turn
/// a numeric config field (a time period, a count) into a dropdown of sensible choices.
pub fn preset_select(
    label: &str,
    custom_id: &str,
    presets: &'static [(&'static str, i64)],
    default: i64,
) -> serenity::CreateModalComponent<'static> {
    secs_select(label, custom_id, presets, default)
}

/// A single optional checkbox — one box, ticked by default if `default`. Unlike
/// [`yes_no_checkbox`] (two mutually-exclusive options) the user simply ticks or
/// unticks it. Read back with [`modal_checked`].
pub fn single_checkbox(
    label: &str,
    custom_id: &str,
    option_label: &str,
    default: bool,
) -> serenity::CreateModalComponent<'static> {
    let group = serenity::CreateCheckboxGroup::new(
        custom_id.to_string(),
        vec![serenity::CreateCheckboxGroupOption::new(option_label.to_string(), "on")
            .default_selection(default)],
    )
    .min_values(0)
    .max_values(1);
    serenity::CreateModalComponent::Label(serenity::CreateLabel::checkbox_group(
        label.to_string(),
        group,
    ))
}

/// Read a [`single_checkbox`] back as a bool (`true` = ticked).
pub fn modal_checked(components: &[serenity::ModalComponent], id: &str) -> bool {
    !crate::ui::read_checkbox_group(components, id).is_empty()
}

/// Read the seconds value back from a [`secs_select`]-built field, or `None` if absent.
pub fn modal_secs(components: &[serenity::ModalComponent], id: &str) -> Option<i64> {
    crate::ui::read_multi_select(components, id)
        .first()
        .and_then(|v| v.parse().ok())
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

/// Validate a 6-digit hex colour like `5865F2` or `#5865F2`. Returns the canonical
/// uppercase 6-char form (no `#`) when valid, or `None`. Used to reject typos before
/// they're silently stored and rendered as black.
pub fn parse_hex_color(s: &str) -> Option<String> {
    let h = s.trim().trim_start_matches('#');
    if h.len() == 6 && h.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(h.to_uppercase())
    } else {
        None
    }
}

/// Render a thread-name pattern with sample values to check it stays within Discord's
/// 100-char channel-name limit and isn't empty. Returns `Err(message)` when the
/// pattern has no variables or would produce an over-long name.
pub fn validate_thread_pattern(pattern: &str) -> Result<(), String> {
    let p = pattern.trim();
    if p.is_empty() {
        return Err("Thread pattern can't be empty.".to_string());
    }
    if !p.contains("{number}") && !p.contains("{username}") {
        return Err(
            "Add at least one of `{number}` or `{username}` so each thread gets a unique name."
                .to_string(),
        );
    }
    // Render with worst-case-ish sample values to estimate the final length.
    let sample = p
        .replace("{number}", "0000")
        .replace("{username}", "an-example-username")
        .replace("{type}", "an-example-category");
    if sample.chars().count() > 100 {
        return Err(
            "That pattern can produce thread names longer than Discord's 100-character limit. Shorten it.".to_string(),
        );
    }
    Ok(())
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
