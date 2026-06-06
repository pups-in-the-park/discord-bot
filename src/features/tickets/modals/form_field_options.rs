use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::parse_form_field_options_modal;
use crate::util::modal_field;

use super::refresh_category_form;

/// `m:ff:opts:{field_id}` — step 2 of the add-question wizard: store the choices for a
/// dropdown/checkbox question, then re-render the category form.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let field_id = parse_form_field_options_modal(&mi.data.custom_id)
        .ok_or_else(|| anyhow::anyhow!("Invalid field ID"))?;
    let field = data
        .db
        .get_form_field(field_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Field not found"))?;

    let options: Vec<String> = modal_field(&mi.data.components, "ff_options")
        .unwrap_or("")
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    if options.is_empty() {
        mi.create_response(
            &ctx.http,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content("Add at least one choice (one per line). The question was left without choices — open it again to add some."),
            ),
        )
        .await?;
        return Ok(());
    }

    let placeholder = modal_field(&mi.data.components, "ff_placeholder").filter(|s| !s.is_empty());
    let json = serde_json::to_string(&options).ok();
    data.db.set_form_field_options(field_id, json.as_deref(), placeholder).await?;

    refresh_category_form(ctx, data, mi, field.ticket_type_id).await?;
    Ok(())
}
