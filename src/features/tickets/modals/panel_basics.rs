use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::parse_panel_basics_modal;
use crate::ui;
use crate::util::{modal_field, parse_hex_color};

use super::super::view::{build_panel_config_form, build_panel_cv2};

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
    let color = modal_field(&mi.data.components, "pnl_color")
        .and_then(parse_hex_color)
        .unwrap_or_else(|| panel.color.clone());

    data.db
        .update_panel(panel_id, &title, description.as_deref(), &color, &panel.layout)
        .await?;

    // Re-render the live message if this panel is already published.
    let updated = data.db.get_panel(panel_id).await?;
    if let Some(p) = &updated {
        if let (Some(mid), Some(cid)) = (p.message_id.as_ref(), p.channel_id.as_ref()) {
            let types = data.db.get_panel_types(panel_id).await?;
            if !types.is_empty() {
                if let (Ok(c), Ok(m)) = (cid.parse::<u64>(), mid.parse::<u64>()) {
                    let tree = build_panel_cv2(p, &types);
                    ui::edit(&ctx.http, serenity::ChannelId::new(c), serenity::MessageId::new(m), &tree).await.ok();
                }
            }
        }
    }

    let all_cats = data.db.get_ticket_types(&g).await?;
    let linked: Vec<i64> = data.db.get_panel_types(panel_id).await?.iter().map(|t| t.id).collect();
    let panel = updated.unwrap_or(panel);
    ui::update(&ctx.http, mi.id, &mi.token, &build_panel_config_form(&panel, &all_cats, &linked)).await?;
    Ok(())
}
