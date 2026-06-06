use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ui;
use crate::util::modal_secs;

use super::super::view::{build_setup_appeals_form, build_setup_raid_form, build_setup_slowmode_form};

/// Numeric-field setup modals (`m:setup:*`). The main forms use selects/toggles;
/// only free-text numbers open a small modal, which re-renders the form on submit.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let Some(guild_id) = mi.guild_id else {
        return Ok(());
    };
    let g = guild_id.to_string();

    match mi.data.custom_id.as_str() {
        "m:setup:appeal_cooldown" => {
            let days: i64 = modal_secs(&mi.data.components, "appeal_cooldown").unwrap_or(7).max(0);
            let cfg = data.db.get_or_create_mod_config(&g).await?;
            data.db
                .update_mod_config(&g, cfg.staff_role_ids.as_deref(), cfg.dm_on_warn, cfg.dm_on_timeout, cfg.dm_on_kick, cfg.dm_on_ban, days)
                .await?;
            let guild_cfg = data.db.get_or_create_guild(&g).await?;
            let new_mod = data.db.get_or_create_mod_config(&g).await?;
            ui::update(&ctx.http, mi.id, &mi.token, &build_setup_appeals_form(&guild_cfg, new_mod.appeal_cooldown_days)).await?;
            return Ok(());
        }
        "m:setup:raid_slowmode" => {
            let secs: i64 = modal_secs(&mi.data.components, "raid_slowmode").unwrap_or(30).max(1);
            let cfg = data.db.get_or_create_raid_config(&g).await?;
            data.db.update_raid_config(&g, cfg.join_threshold, cfg.decay_rate, secs).await?;
            let new_cfg = data.db.get_or_create_raid_config(&g).await?;
            ui::update(&ctx.http, mi.id, &mi.token, &build_setup_raid_form(&new_cfg)).await?;
            return Ok(());
        }
        "m:setup:slowmode_config" => {
            let window: i64 = modal_secs(&mi.data.components, "slowmode_window").unwrap_or(10).max(1);
            let capacity: i64 = modal_secs(&mi.data.components, "slowmode_capacity").unwrap_or(20).max(1);
            let cfg = data.db.get_or_create_slowmode_config(&g).await?;
            data.db.update_slowmode_config(&g, cfg.enabled, window, capacity).await?;
            let new_cfg = data.db.get_or_create_slowmode_config(&g).await?;
            ui::update(&ctx.http, mi.id, &mi.token, &build_setup_slowmode_form(&new_cfg)).await?;
            return Ok(());
        }
        _ => {}
    }

    // Fallback acknowledgement for any unmatched setup modal (shouldn't be reached).
    mi.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge).await?;
    Ok(())
}
