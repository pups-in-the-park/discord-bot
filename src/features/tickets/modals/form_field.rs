use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::{parse_form_field_edit_modal, parse_form_field_new_modal};
use crate::util::{modal_checked, modal_field, respond_ephemeral_modal};

use super::super::view::ConfigTab;
use super::refresh_category_form;

/// The values shared by the add and edit question modals.
struct Submission {
    label: String,
    required: bool,
    placeholder: Option<String>,
    /// `Some(json)` when the modal carried a choices field with at least one line.
    options_json: Option<String>,
    /// Whether the modal carried a choices field at all (dropdown/checkbox).
    has_choices_field: bool,
}

fn read_submission(mi: &serenity::ModalInteraction) -> Submission {
    let label = modal_field(&mi.data.components, "ff_label").unwrap_or("").trim().to_string();
    let required = modal_checked(&mi.data.components, "ff_required");
    let placeholder =
        modal_field(&mi.data.components, "ff_placeholder").filter(|s| !s.is_empty()).map(str::to_string);

    let raw_options = modal_field(&mi.data.components, "ff_options");
    let has_choices_field = raw_options.is_some();
    let options: Vec<String> = raw_options
        .unwrap_or("")
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    let options_json = if options.is_empty() { None } else { serde_json::to_string(&options).ok() };

    Submission { label, required, placeholder, options_json, has_choices_field }
}

/// Discord rejects selects/checkboxes with >25 options or >100-char labels —
/// at ticket-open time, taking the whole category down. Catch it at authoring.
fn choices_error(sub: &Submission) -> Option<String> {
    let options: Vec<String> = sub
        .options_json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();
    if options.len() > 25 {
        return Some(format!(
            "Discord allows at most 25 choices per question — you listed {}.",
            options.len()
        ));
    }
    if let Some(long) = options.iter().find(|o| o.chars().count() > 100) {
        return Some(format!(
            "Each choice must be 100 characters or fewer — \"{}…\" is too long.",
            long.chars().take(30).collect::<String>()
        ));
    }
    // Discord also rejects duplicate values. Compare lowercased, first 100
    // chars only — the render-time cap can collapse two long choices.
    let mut seen = std::collections::HashSet::new();
    for o in &options {
        let key: String = o.trim().to_lowercase().chars().take(100).collect();
        if !seen.insert(key) {
            let snippet: String = o.chars().take(30).collect();
            return Some(format!(
                "Choices must be unique — \"{}{}\" appears more than once.",
                snippet,
                if o.chars().count() > 30 { "…" } else { "" }
            ));
        }
    }
    None
}

/// `m:ff:new:{type_id}:{style}` — create an intake question in one step: the type
/// was picked from the Questions-tab select, so the modal collects everything
/// (label, required, placeholder, and choices for dropdown/checkbox questions).
pub async fn handle_new(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let (type_id, style) = parse_form_field_new_modal(&mi.data.custom_id)
        .ok_or_else(|| anyhow::anyhow!("Invalid question modal ID"))?;
    if !matches!(style.as_str(), "short" | "paragraph" | "select" | "checkbox") {
        return Err(anyhow::anyhow!("Unknown question style: {style}"));
    }

    let sub = read_submission(mi);
    if sub.label.is_empty() {
        mi.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge).await?;
        return Ok(());
    }
    if sub.has_choices_field && sub.options_json.is_none() {
        respond_ephemeral_modal(ctx, mi, "Add at least one choice (one per line) — the question wasn't created.").await;
        return Ok(());
    }
    if let Some(err) = choices_error(&sub) {
        respond_ephemeral_modal(ctx, mi, &format!("{err} The question wasn't created.")).await;
        return Ok(());
    }

    let existing = data.db.get_form_fields(type_id).await?;
    // Guard the 5-question Discord modal limit.
    if existing.len() >= 5 {
        respond_ephemeral_modal(ctx, mi, "This category already has the maximum of 5 questions.").await;
        return Ok(());
    }
    // Responses are keyed by label — a duplicate would swallow an answer.
    if existing.iter().any(|f| f.label.eq_ignore_ascii_case(&sub.label)) {
        respond_ephemeral_modal(ctx, mi, "This category already has a question with that label — pick a different one.").await;
        return Ok(());
    }

    data.db
        .add_form_field(
            type_id,
            &sub.label,
            sub.placeholder.as_deref(),
            &style,
            sub.required,
            None,
            None,
            sub.options_json.as_deref(),
        )
        .await?;

    refresh_category_form(ctx, data, mi, type_id, ConfigTab::Questions).await?;
    Ok(())
}

/// `m:ff:edit:{field_id}` — update an existing question from the same modal,
/// prefilled. The style is fixed at creation; label, required, placeholder, and
/// choices (for dropdown/checkbox) are editable.
pub async fn handle_edit(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let field_id = parse_form_field_edit_modal(&mi.data.custom_id)
        .ok_or_else(|| anyhow::anyhow!("Invalid question modal ID"))?;
    let field = data
        .db
        .get_form_field(field_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Question not found"))?;

    let sub = read_submission(mi);
    if sub.label.is_empty() {
        mi.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge).await?;
        return Ok(());
    }
    if sub.has_choices_field && sub.options_json.is_none() {
        respond_ephemeral_modal(ctx, mi, "Add at least one choice (one per line) — nothing was changed.").await;
        return Ok(());
    }
    if let Some(err) = choices_error(&sub) {
        respond_ephemeral_modal(ctx, mi, &format!("{err} Nothing was changed.")).await;
        return Ok(());
    }
    // Responses are keyed by label — renaming onto another question's label
    // would swallow an answer.
    let siblings = data.db.get_form_fields(field.ticket_type_id).await?;
    if siblings
        .iter()
        .any(|f| f.id != field_id && f.label.eq_ignore_ascii_case(&sub.label))
    {
        respond_ephemeral_modal(ctx, mi, "Another question in this category already has that label — nothing was changed.").await;
        return Ok(());
    }

    // A text question's modal has no choices field; keep its stored options as-is.
    let options_json =
        if sub.has_choices_field { sub.options_json } else { field.options.clone() };
    data.db
        .update_form_field(field_id, &sub.label, sub.required, options_json.as_deref(), sub.placeholder.as_deref())
        .await?;

    refresh_category_form(ctx, data, mi, field.ticket_type_id, ConfigTab::Questions).await?;
    Ok(())
}
