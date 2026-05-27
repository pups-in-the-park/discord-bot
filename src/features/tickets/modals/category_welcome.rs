use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::util::modal_field;

use super::refresh_category_form;

/// `m:cat:welcome:{type_id}` — edit the welcome message.
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

    let welcome = modal_field(&mi.data.components, "welcome").filter(|s| !s.is_empty()).map(str::to_string);

    // Preserve all other fields
    let current = data
        .db
        .get_ticket_type_by_id(type_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Category not found"))?;

    data.db
        .update_ticket_type(type_id, &current.label, current.emoji.as_deref(), current.description.as_deref(), &current.color, welcome.as_deref())
        .await?;

    refresh_category_form(ctx, data, mi, type_id).await?;
    Ok(())
}
