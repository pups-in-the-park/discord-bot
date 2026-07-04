use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ui::{self, read_multi_select};
use crate::util::{modal_field, parse_hex_color, respond_ephemeral_modal, sanitise_name};

use super::super::view::{build_category_config_form, ConfigTab};

/// `m:cat:create` — create a new ticket category from the single create modal,
/// then land straight on its configure panel. The internal name is auto-derived
/// from the label.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let Some(guild_id) = mi.guild_id else {
        return Ok(());
    };
    let g = guild_id.to_string();

    let label = modal_field(&mi.data.components, "cat_label").unwrap_or("").trim().to_string();
    let emoji = modal_field(&mi.data.components, "cat_emoji").filter(|s| !s.is_empty()).map(str::to_string);
    let description = modal_field(&mi.data.components, "cat_description").filter(|s| !s.is_empty()).map(str::to_string);
    let color = read_multi_select(&mi.data.components, "cat_color")
        .first()
        .and_then(|h| parse_hex_color(h))
        .unwrap_or_else(|| "5865F2".to_string());

    if label.is_empty() {
        respond_ephemeral_modal(ctx, mi, "A name is required to create a category.").await;
        return Ok(());
    }

    // Auto-derive a unique internal name from the label. Deactivated categories
    // keep their rows (and the UNIQUE(guild_id,name) constraint), so dedup
    // against every name, not just active ones.
    let taken: std::collections::HashSet<String> =
        data.db.get_all_ticket_type_names(&g).await?.into_iter().collect();
    let base = {
        let s = sanitise_name(&label);
        if s.is_empty() { "category".to_string() } else { s }
    };
    let mut name = base.clone();
    let mut n = 2;
    while taken.contains(&name) {
        name = format!("{}-{}", base, n);
        n += 1;
    }

    let cat = data
        .db
        .create_ticket_type(&g, &name, &label, emoji.as_deref(), description.as_deref(), &color, None)
        .await?;

    // Land straight on the configure panel: update the hub in place when the
    // modal came from a hub button; a fresh ephemeral when it came from the
    // `/ticket category create` slash command (no message to update).
    let form = build_category_config_form(&cat, &[], &[], ConfigTab::Overview);
    if mi.message.is_some() {
        ui::update(&ctx.http, mi.id, &mi.token, &form).await?;
    } else {
        ui::respond_ephemeral_to(&ctx.http, mi.id, &mi.token, &form).await?;
    }
    Ok(())
}
