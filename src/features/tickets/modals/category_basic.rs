use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ui::read_multi_select;
use crate::util::{modal_field, parse_hex_color, respond_ephemeral_modal};

use super::super::view::ConfigTab;
use super::refresh_category_form;

/// `m:cat:basic:{type_id}` — the shared basic-info modal: label, emoji,
/// description, and the accent-colour dropdown.
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
    // Discord blocks an empty required field but not a whitespace-only one; guard
    // it here too so we never persist a blank button/menu label (as create does).
    if label.is_empty() {
        respond_ephemeral_modal(ctx, mi, "A name is required.").await;
        return Ok(());
    }
    let emoji = modal_field(&mi.data.components, "cat_emoji").filter(|s| !s.is_empty()).map(str::to_string);
    let desc = modal_field(&mi.data.components, "cat_description").filter(|s| !s.is_empty()).map(str::to_string);

    // Read current values so we don't clobber the welcome message or a good colour.
    let current = data
        .db
        .get_ticket_type_by_id(type_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Category not found"))?;

    let color = read_multi_select(&mi.data.components, "cat_color")
        .first()
        .and_then(|h| parse_hex_color(h))
        .unwrap_or_else(|| current.color.clone());

    data.db
        .update_ticket_type(type_id, &label, emoji.as_deref(), desc.as_deref(), &color, current.welcome_message.as_deref())
        .await?;

    refresh_category_form(ctx, data, mi, type_id, ConfigTab::Overview).await?;
    Ok(())
}
