use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::parse_panel_cfg;
use crate::ui::{self, Modal, TextInputStyle};
use crate::util::respond_ephemeral;

use super::super::view::{
    build_panel_config_form, build_panel_cv2, build_panel_delete_confirm, build_panel_hub,
};

/// Re-render the panel configure form in place from fresh data.
async fn refresh(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
    panel_id: i64,
    g: &str,
) -> Result<(), anyhow::Error> {
    let Some(panel) = data.db.get_panel(panel_id).await? else {
        return Ok(());
    };
    let all_cats = data.db.get_ticket_types(g).await?;
    let linked: Vec<i64> = data.db.get_panel_types(panel_id).await?.iter().map(|t| t.id).collect();
    ui::update(&ctx.http, ci.id, &ci.token, &build_panel_config_form(&panel, &all_cats, &linked)).await
}

/// If the panel is already posted, re-render that live message to match current
/// settings/categories.
async fn republish_if_live(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    panel_id: i64,
) -> Result<(), anyhow::Error> {
    let Some(panel) = data.db.get_panel(panel_id).await? else {
        return Ok(());
    };
    let (Some(mid), Some(cid)) = (panel.message_id.as_ref(), panel.channel_id.as_ref()) else {
        return Ok(());
    };
    let types = data.db.get_panel_types(panel_id).await?;
    if types.is_empty() {
        return Ok(());
    }
    if let (Ok(c), Ok(m)) = (cid.parse::<u64>(), mid.parse::<u64>()) {
        let tree = build_panel_cv2(&panel, &types);
        ui::edit(&ctx.http, serenity::ChannelId::new(c), serenity::MessageId::new(m), &tree).await.ok();
    }
    Ok(())
}

/// Controls inside the panel configure form (`pnl:cfg:{id}:{field}`).
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
) -> Result<(), anyhow::Error> {
    let (panel_id, field) =
        parse_panel_cfg(&ci.data.custom_id).ok_or_else(|| anyhow::anyhow!("Invalid panel config ID"))?;

    let Some(guild_id) = ci.guild_id else {
        return Ok(());
    };
    if !crate::permissions::is_mod_staff(ctx, data, guild_id, ci.user.id).await {
        respond_ephemeral(ctx, ci, "Only staff can configure ticket panels.").await;
        return Ok(());
    }
    let g = guild_id.to_string();

    match field.as_str() {
        // Edit title / description / colour via modal.
        "basics" => {
            let panel = data.db.get_panel(panel_id).await?
                .ok_or_else(|| anyhow::anyhow!("Panel not found"))?;
            let modal = Modal::new(crate::ids::cid_panel_basics_modal(panel_id), "✏️ Edit Panel Basics")
                .text_row("pnl_title", "Panel title", TextInputStyle::Short, "e.g. Support", true, Some(&panel.title))
                .text_row("pnl_desc", "Description", TextInputStyle::Paragraph, "Shown under the title", false, panel.description.as_deref())
                .text_row("pnl_color", "Accent colour (hex)", TextInputStyle::Short, "e.g. 5865F2", false, Some(&panel.color));
            ui::open_modal(&ctx.http, ci, &modal).await?;
        }

        // Change layout (buttons / dropdown).
        "layout" => {
            let layout = if let serenity::ComponentInteractionDataKind::StringSelect { values } = &ci.data.kind {
                if values.first().map(|s| s.as_str()) == Some("select") { "select" } else { "buttons" }
            } else {
                "buttons"
            };
            if let Some(panel) = data.db.get_panel(panel_id).await? {
                data.db.update_panel(panel_id, &panel.title, panel.description.as_deref(), &panel.color, layout).await?;
            }
            republish_if_live(ctx, data, panel_id).await?;
            refresh(ctx, data, ci, panel_id, &g).await?;
        }

        // Choose which categories appear: replace the link set (the fix for the
        // old additive-only publish, where removed categories never unlinked).
        "cats" => {
            let selected: Vec<i64> = if let serenity::ComponentInteractionDataKind::StringSelect { values } = &ci.data.kind {
                values.iter().filter_map(|s| s.parse().ok()).collect()
            } else {
                vec![]
            };
            let current: Vec<i64> = data.db.get_panel_types(panel_id).await?.iter().map(|t| t.id).collect();
            for tid in &current {
                if !selected.contains(tid) {
                    data.db.remove_panel_type(panel_id, *tid).await.ok();
                }
            }
            for tid in &selected {
                if !current.contains(tid) {
                    data.db.add_panel_type(panel_id, *tid).await.ok();
                }
            }
            republish_if_live(ctx, data, panel_id).await?;
            refresh(ctx, data, ci, panel_id, &g).await?;
        }

        // Publish (or re-publish) to the selected channel.
        "pub" => {
            let channel: Option<String> = if let serenity::ComponentInteractionDataKind::ChannelSelect { values } = &ci.data.kind {
                values.first().map(|c| c.to_string())
            } else {
                None
            };
            let Some(channel) = channel else {
                ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge).await?;
                return Ok(());
            };
            let types = data.db.get_panel_types(panel_id).await?;
            if types.is_empty() {
                respond_ephemeral(ctx, ci, "Add at least one category to this panel before publishing.").await;
                return Ok(());
            }
            let panel = data.db.get_panel(panel_id).await?
                .ok_or_else(|| anyhow::anyhow!("Panel not found"))?;
            let tree = build_panel_cv2(&panel, &types);
            let Ok(chan_id) = channel.parse::<u64>() else {
                respond_ephemeral(ctx, ci, "Couldn't read that channel.").await;
                return Ok(());
            };
            let chan = serenity::ChannelId::new(chan_id);

            // Edit the existing message if it's in the same channel; otherwise post anew.
            let same_channel = panel.channel_id.as_deref() == Some(channel.as_str());
            let msg = match (same_channel, panel.message_id.as_ref().and_then(|m| m.parse::<u64>().ok())) {
                (true, Some(mid)) => {
                    match ui::edit(&ctx.http, chan, serenity::MessageId::new(mid), &tree).await {
                        Ok(m) => m,
                        Err(_) => ui::send(&ctx.http, chan, &tree).await?,
                    }
                }
                _ => ui::send(&ctx.http, chan, &tree).await?,
            };
            data.db.update_panel_message(panel_id, &channel, &msg.id.to_string()).await?;
            refresh(ctx, data, ci, panel_id, &g).await?;
        }

        // Delete flow.
        "delete" => {
            let panel = data.db.get_panel(panel_id).await?
                .ok_or_else(|| anyhow::anyhow!("Panel not found"))?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_panel_delete_confirm(&panel)).await?;
        }
        "delete_no" => {
            refresh(ctx, data, ci, panel_id, &g).await?;
        }
        "delete_yes" => {
            data.db.delete_panel(panel_id).await?;
            let panels = data.db.get_panels(&g).await?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_panel_hub(&panels)).await?;
        }

        _ => {
            ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge).await?;
        }
    }
    Ok(())
}
