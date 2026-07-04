use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::util::{modal_field, validate_thread_pattern};

use super::refresh_category_form;

/// `m:cat:thread:{type_id}` — edit the thread name pattern.
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

    let pattern = modal_field(&mi.data.components, "pattern")
        .filter(|s| !s.is_empty())
        .unwrap_or("ticket-{number}-{username}")
        .trim()
        .to_string();

    if let Err(msg) = validate_thread_pattern(&pattern) {
        crate::util::respond_ephemeral_modal(ctx, mi, &format!("{} Nothing was changed.", msg)).await;
        return Ok(());
    }

    data.db.set_ticket_type_thread_pattern(type_id, &pattern).await?;
    refresh_category_form(ctx, data, mi, type_id, super::super::view::ConfigTab::Overview).await?;
    Ok(())
}
