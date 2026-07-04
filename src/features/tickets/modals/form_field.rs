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

    // Guard the 5-question Discord modal limit.
    if data.db.count_form_fields(type_id).await? >= 5 {
        respond_ephemeral_modal(ctx, mi, "This category already has the maximum of 5 questions.").await;
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

    // A text question's modal has no choices field; keep its stored options as-is.
    let options_json =
        if sub.has_choices_field { sub.options_json } else { field.options.clone() };
    data.db
        .update_form_field(field_id, &sub.label, sub.required, options_json.as_deref(), sub.placeholder.as_deref())
        .await?;

    refresh_category_form(ctx, data, mi, field.ticket_type_id, ConfigTab::Questions).await?;
    Ok(())
}
