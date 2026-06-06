use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::util::modal_field;

/// `m:cat:edit:{cat_id}` — update a category's basic fields.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let cat_id: i64 = mi
        .data
        .custom_id
        .split(':')
        .nth(3)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("Invalid category ID"))?;

    let label = modal_field(&mi.data.components, "cat_label").unwrap_or("").trim().to_string();
    let emoji = modal_field(&mi.data.components, "cat_emoji").filter(|s| !s.is_empty()).map(str::to_string);
    let color = modal_field(&mi.data.components, "cat_color").filter(|s| !s.is_empty()).unwrap_or("5865F2").to_string();
    let welcome = modal_field(&mi.data.components, "cat_welcome").filter(|s| !s.is_empty()).map(str::to_string);

    data.db
        .update_ticket_type(cat_id, &label, emoji.as_deref(), None, &color, welcome.as_deref())
        .await?;

    mi.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content("Category updated."),
        ),
    )
    .await?;
    Ok(())
}
