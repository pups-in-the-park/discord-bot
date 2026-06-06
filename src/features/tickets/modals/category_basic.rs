use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::util::{modal_field, parse_hex_color};

use super::refresh_category_form;

/// `m:cat:basic:{type_id}` — edit label, emoji, color, description.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let type_id: i64 = mi
        .data
        .custom_id
        .split(':')
        .nth(3)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("Invalid category ID"))?;

    let label = modal_field(&mi.data.components, "cat_label").unwrap_or("").trim().to_string();
    let emoji = modal_field(&mi.data.components, "cat_emoji").filter(|s| !s.is_empty()).map(str::to_string);
    let color_input = modal_field(&mi.data.components, "cat_color").filter(|s| !s.is_empty());
    let desc = modal_field(&mi.data.components, "cat_desc").filter(|s| !s.is_empty()).map(str::to_string);

    // Read current values so we don't clobber the welcome message or a good colour.
    let current = data
        .db
        .get_ticket_type_by_id(type_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Category not found"))?;

    // Validate the colour; on a typo, keep the existing colour and tell the user.
    let color = match color_input {
        Some(raw) => match parse_hex_color(raw) {
            Some(valid) => valid,
            None => {
                mi.create_response(
                    &ctx.http,
                    serenity::CreateInteractionResponse::Message(
                        serenity::CreateInteractionResponseMessage::new()
                            .ephemeral(true)
                            .content("Couldn't read that colour — use a 6-digit hex like `5865F2`. Nothing was changed."),
                    ),
                )
                .await?;
                return Ok(());
            }
        },
        None => current.color.clone(),
    };

    data.db
        .update_ticket_type(type_id, &label, emoji.as_deref(), desc.as_deref(), &color, current.welcome_message.as_deref())
        .await?;

    refresh_category_form(ctx, data, mi, type_id).await?;
    Ok(())
}
