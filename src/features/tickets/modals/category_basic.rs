use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::util::modal_field;

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
    let color = modal_field(&mi.data.components, "cat_color").filter(|s| !s.is_empty()).unwrap_or("5865F2").to_string();
    let desc = modal_field(&mi.data.components, "cat_desc").filter(|s| !s.is_empty()).map(str::to_string);

    // Read current welcome_message so we don't clobber it
    let current = data
        .db
        .get_ticket_type_by_id(type_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Category not found"))?;

    data.db
        .update_ticket_type(type_id, &label, emoji.as_deref(), desc.as_deref(), &color, current.welcome_message.as_deref())
        .await?;

    refresh_category_form(ctx, data, mi, type_id).await?;
    Ok(())
}
