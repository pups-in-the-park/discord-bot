use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::{
    BotData, CID_PANEL_HUB_BACK, CID_PANEL_HUB_CREATE, CID_PANEL_HUB_OPEN, CID_PANEL_HUB_SELECT,
};
use crate::ui;
use crate::util::respond_ephemeral;

use super::super::view::{build_panel_basics_modal, build_panel_hub};

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
            if data.db.get_panel(panel_id).await?.is_none() {
                respond_ephemeral(ctx, ci, "That panel no longer exists.").await;
                return Ok(());
            }
            super::panel_config::show_config_form(&ctx.http, data, ci.id, &ci.token, panel_id, &g).await?;
        }
        CID_PANEL_HUB_CREATE => {
            let modal = build_panel_basics_modal(
                crate::ids::CID_PANEL_CREATE_MODAL.to_string(),
                "➕ Create Ticket Panel",
                None,
            );
            ui::open_modal(&ctx.http, ci, &modal).await?;
        }
        _ => {
            ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge).await?;
        }
    }
    Ok(())
}
