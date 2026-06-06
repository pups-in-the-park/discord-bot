use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::{
    BotData, CID_PANEL_HUB_BACK, CID_PANEL_HUB_CREATE, CID_PANEL_HUB_OPEN, CID_PANEL_HUB_SELECT,
};
use crate::ui::{self, Modal, TextInputStyle};
use crate::util::respond_ephemeral;

use super::super::view::{build_panel_config_form, build_panel_hub};

/// The panel management hub (`pnl:hub:*`): open it, go back to the list, pick a
/// panel to configure, or create a new one.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
) -> Result<(), anyhow::Error> {
    let Some(guild_id) = ci.guild_id else {
        return Ok(());
    };
    if !crate::permissions::is_mod_staff(ctx, data, guild_id, ci.user.id).await {
        respond_ephemeral(ctx, ci, "Only staff can configure ticket panels.").await;
        return Ok(());
    }
    let g = guild_id.to_string();

    match ci.data.custom_id.as_str() {
        CID_PANEL_HUB_OPEN => {
            let panels = data.db.get_panels(&g).await?;
            ui::respond_ephemeral(&ctx.http, ci, &build_panel_hub(&panels)).await?;
        }
        CID_PANEL_HUB_BACK => {
            let panels = data.db.get_panels(&g).await?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_panel_hub(&panels)).await?;
        }
        CID_PANEL_HUB_SELECT => {
            let panel_id: Option<i64> = if let serenity::ComponentInteractionDataKind::StringSelect { values } = &ci.data.kind {
                values.first().and_then(|s| s.parse().ok())
            } else {
                None
            };
            let Some(panel_id) = panel_id else {
                return Ok(());
            };
            let Some(panel) = data.db.get_panel(panel_id).await? else {
                respond_ephemeral(ctx, ci, "That panel no longer exists.").await;
                return Ok(());
            };
            let all_cats = data.db.get_ticket_types(&g).await?;
            let linked: Vec<i64> = data.db.get_panel_types(panel_id).await?.iter().map(|t| t.id).collect();
            ui::update(&ctx.http, ci.id, &ci.token, &build_panel_config_form(&panel, &all_cats, &linked)).await?;
        }
        CID_PANEL_HUB_CREATE => {
            let modal = Modal::new("m:pnl:create", "➕ Create Ticket Panel")
                .text_row("pnl_title", "Panel title", TextInputStyle::Short, "e.g. Support", true, None)
                .text_row("pnl_desc", "Description", TextInputStyle::Paragraph, "Shown under the title on the panel", false, None)
                .text_row("pnl_color", "Accent colour (hex)", TextInputStyle::Short, "e.g. 5865F2", false, None);
            ui::open_modal(&ctx.http, ci, &modal).await?;
        }
        _ => {
            ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge).await?;
        }
    }
    Ok(())
}
