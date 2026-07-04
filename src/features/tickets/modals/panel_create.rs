use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::util::{modal_field, parse_hex_color, respond_ephemeral_modal};

use super::super::components::panel_config::show_config_form;

/// `m:pnl:create` — create a new panel, then open its configure form in place.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let Some(guild_id) = mi.guild_id else {
        return Ok(());
    };
    let g = guild_id.to_string();

    let title = modal_field(&mi.data.components, "pnl_title").unwrap_or("").trim().to_string();
    let description = modal_field(&mi.data.components, "pnl_desc").filter(|s| !s.is_empty()).map(str::to_string);
    let color = modal_field(&mi.data.components, "pnl_color")
        .and_then(parse_hex_color)
        .unwrap_or_else(|| "5865F2".to_string());

    if title.is_empty() {
        respond_ephemeral_modal(ctx, mi, "A title is required to create a panel.").await;
        return Ok(());
    }

    let panel = data
        .db
        .create_panel(&g, &title, description.as_deref(), &color, "buttons")
        .await?;

    show_config_form(&ctx.http, data, mi.id, &mi.token, panel.id, &g).await?;
    Ok(())
}
