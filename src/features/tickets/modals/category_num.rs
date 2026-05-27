use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::util::modal_field;

use super::refresh_category_form;

/// `m:cat:num:{action}:{type_id}` — numeric fields: `max_open` or `auto_close`.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let parts: Vec<&str> = mi.data.custom_id.split(':').collect();
    let action = parts.get(3).copied().unwrap_or("");
    let type_id: i64 = parts
        .get(4)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("Invalid category ID"))?;

    match action {
        "max_open" => {
            let max: i64 = modal_field(&mi.data.components, "max_open")
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(1)
                .max(1);
            data.db.set_ticket_type_max_open(type_id, max).await?;
        }
        "auto_close" => {
            // 0 or blank = disabled (None)
            let hours: Option<i64> = modal_field(&mi.data.components, "auto_close")
                .and_then(|s| s.trim().parse::<i64>().ok())
                .filter(|&h| h > 0);
            data.db.set_ticket_type_auto_close(type_id, hours).await?;
        }
        _ => {}
    }

    refresh_category_form(ctx, data, mi, type_id).await?;
    Ok(())
}
