use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ui;
use crate::util::{modal_field, parse_hex_color};

use super::super::view::build_panel_config_form;

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
        mi.create_response(
            &ctx.http,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content("A title is required to create a panel."),
            ),
        )
        .await?;
        return Ok(());
    }

    let panel = data
        .db
        .create_panel(&g, &title, description.as_deref(), &color, "buttons")
        .await?;

    let all_cats = data.db.get_ticket_types(&g).await?;
    ui::update(&ctx.http, mi.id, &mi.token, &build_panel_config_form(&panel, &all_cats, &[])).await?;
    Ok(())
}
