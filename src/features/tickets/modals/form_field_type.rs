use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::parse_form_field_type_modal;
use crate::ui::{self, read_checkbox_group, read_multi_select};
use crate::util::modal_field;

use super::super::view::build_question_options_card;
use super::refresh_category_form;

/// Reply to a modal submission with a short ephemeral message.
async fn ephemeral(ctx: &serenity::Context, mi: &serenity::ModalInteraction, msg: &str) -> Result<(), anyhow::Error> {
    mi.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new().ephemeral(true).content(msg),
        ),
    )
    .await?;
    Ok(())
}

/// `m:ff:type:{type_id}` — step 1 of the add-question wizard: create the field from
/// its label, type, and required flag. Text questions finish here; dropdown/checkbox
/// questions continue to a choices card.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let type_id = parse_form_field_type_modal(&mi.data.custom_id)
        .ok_or_else(|| anyhow::anyhow!("Invalid type ID"))?;

    let label = modal_field(&mi.data.components, "ff_label").unwrap_or("").trim().to_string();
    if label.is_empty() {
        mi.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge).await?;
        return Ok(());
    }

    let style = read_multi_select(&mi.data.components, "ff_type")
        .first()
        .map(|s| s.as_str())
        .filter(|s| matches!(*s, "short" | "paragraph" | "select" | "checkbox"))
        .unwrap_or("short")
        .to_string();
    let required = read_checkbox_group(&mi.data.components, "ff_required")
        .iter()
        .any(|v| v == "yes");

    // Guard the 5-question Discord modal limit.
    if data.db.get_form_fields(type_id).await?.len() >= 5 {
        return ephemeral(ctx, mi, "This category already has the maximum of 5 questions.").await;
    }

    let field = data
        .db
        .add_form_field(type_id, &label, None, &style, required, None, None, None)
        .await?;
    data.db.set_ticket_type_has_form(type_id, true).await?;

    if field.needs_options() {
        ui::update(&ctx.http, mi.id, &mi.token, &build_question_options_card(type_id, &field)).await?;
    } else {
        refresh_category_form(ctx, data, mi, type_id).await?;
    }
    Ok(())
}
