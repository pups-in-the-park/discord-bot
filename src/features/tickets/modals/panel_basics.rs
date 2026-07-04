use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::parse_panel_basics_modal;
use crate::util::{modal_field, parse_hex_color, respond_ephemeral_modal};

use super::super::components::panel_config::show_config_form;
use super::super::service::republish_panel;

/// `m:pnl:basics:{panel_id}` — edit a panel's title / description / colour, then
/// re-render the configure form (and the live panel message, if published).
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let panel_id = parse_panel_basics_modal(&mi.data.custom_id)
        .ok_or_else(|| anyhow::anyhow!("Invalid panel ID"))?;
    let Some(guild_id) = mi.guild_id else {
        return Ok(());
    };
    let g = guild_id.to_string();

    let panel = data.db.get_panel(panel_id).await?
        .ok_or_else(|| anyhow::anyhow!("Panel not found"))?;

    let title = modal_field(&mi.data.components, "pnl_title").unwrap_or("").trim().to_string();
    let title = if title.is_empty() { panel.title.clone() } else { title };
    let description = modal_field(&mi.data.components, "pnl_desc").filter(|s| !s.is_empty()).map(str::to_string);
    // Empty = keep the current colour; a non-empty but unparseable value is a
    // mistake worth surfacing rather than silently ignoring.
    let color_raw = modal_field(&mi.data.components, "pnl_color").unwrap_or("").trim().to_string();
    let color = if color_raw.is_empty() {
        panel.color.clone()
    } else {
        match parse_hex_color(&color_raw) {
            Some(c) => c,
            None => {
                respond_ephemeral_modal(
                    ctx,
                    mi,
                    "That colour isn't a valid hex code — use something like `5865F2`.",
                )
                .await;
                return Ok(());
            }
        }
    };

    data.db
        .update_panel(panel_id, &title, description.as_deref(), &color, &panel.layout)
        .await?;

    // Re-render the live message if this panel is already published. The updated
    // fields are in hand, so no re-fetch is needed.
    let panel = crate::db::Panel { title, description, color, ..panel };
    republish_panel(&ctx.http, data, &panel).await;

    show_config_form(&ctx.http, data, mi.id, &mi.token, panel_id, &g).await?;
    Ok(())
}
