use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::parse_form_field_add_modal;
use crate::util::modal_field;

/// `m:ff:add:{type_id}:{style}:{required}` — add an intake-form field to a category.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let (type_id, style, required) = parse_form_field_add_modal(&mi.data.custom_id)
        .ok_or_else(|| anyhow::anyhow!("Invalid type ID"))?;
    let style = if style == "paragraph" { "paragraph" } else { "short" };

    let label = modal_field(&mi.data.components, "ff_label").unwrap_or("").trim().to_string();
    let placeholder = modal_field(&mi.data.components, "ff_placeholder").filter(|s| !s.is_empty()).map(str::to_string);
    let max_length: Option<i64> = modal_field(&mi.data.components, "ff_max_length").and_then(|s| s.trim().parse().ok());

    data.db
        .add_form_field(type_id, &label, placeholder.as_deref(), style, required, None, max_length)
        .await?;
    data.db.set_ticket_type_has_form(type_id, true).await?;

    mi.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content(format!("Form field '{}' added.", label)),
        ),
    )
    .await?;
    Ok(())
}
