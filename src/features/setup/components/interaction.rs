use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ui::{self, Modal, TextInputStyle};
use crate::util::respond_ephemeral;

use super::super::view::{
    build_setup_appeals_form, build_setup_log_form, build_setup_mod_form, build_setup_raid_form,
    build_setup_slowmode_form, build_setup_ticket_form,
};

/// Every select/button/toggle inside a setup ephemeral form. Saves the changed
/// value immediately and updates the form in-place (or opens a numeric modal).
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
) -> Result<(), anyhow::Error> {
    let Some(guild_id) = ci.guild_id else {
        return Ok(());
    };
    let g = guild_id.to_string();
    let id = ci.data.custom_id.as_str();

    if !crate::permissions::is_mod_staff(ctx, data, guild_id, ci.user.id).await {
        respond_ephemeral(ctx, ci, "Only staff can configure the bot.").await;
        return Ok(());
    }

    // Helper: extract the first selected ChannelId as a String, or None if cleared.
    let selected_channel = |ci: &serenity::ComponentInteraction| -> Option<String> {
        if let serenity::ComponentInteractionDataKind::ChannelSelect { values } = &ci.data.kind {
            values.first().map(|id| id.to_string())
        } else {
            None
        }
    };

    match id {
        // ── Log channels ─────────────────────────────────────────────────────
        "setup:log:fallback" => {
            let ch = selected_channel(ci);
            data.db.set_log_fallback_channel(&g, ch.as_deref()).await?;
            let cfg = data.db.get_or_create_guild(&g).await?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_setup_log_form(&cfg)).await?;
        }
        "setup:log:mod" => {
            let ch = selected_channel(ci);
            data.db.set_mod_log_channel(&g, ch.as_deref()).await?;
            let cfg = data.db.get_or_create_guild(&g).await?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_setup_log_form(&cfg)).await?;
        }
        "setup:log:chat" => {
            let ch = selected_channel(ci);
            data.db.set_chat_log_channel(&g, ch.as_deref()).await?;
            let cfg = data.db.get_or_create_guild(&g).await?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_setup_log_form(&cfg)).await?;
        }
        "setup:log:ticket" => {
            let ch = selected_channel(ci);
            data.db.set_ticket_log_channel(&g, ch.as_deref()).await?;
            let cfg = data.db.get_or_create_guild(&g).await?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_setup_log_form(&cfg)).await?;
        }

        // ── Ticket channels ──────────────────────────────────────────────────
        "setup:ticket:channel" => {
            let ch = selected_channel(ci);
            data.db.set_ticket_channel(&g, ch.as_deref()).await?;
            let cfg = data.db.get_or_create_guild(&g).await?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_setup_ticket_form(&cfg)).await?;
        }
        "setup:ticket:reports" => {
            let ch = selected_channel(ci);
            data.db.set_reports_channel(&g, ch.as_deref()).await?;
            let cfg = data.db.get_or_create_guild(&g).await?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_setup_ticket_form(&cfg)).await?;
        }

        // ── Mod staff role select ────────────────────────────────────────────
        "setup:mod:staff" => {
            let role_ids: Vec<String> = if let serenity::ComponentInteractionDataKind::RoleSelect { values, .. } = &ci.data.kind {
                values.iter().map(|id| id.to_string()).collect()
            } else {
                vec![]
            };
            let json = if role_ids.is_empty() { None } else { serde_json::to_string(&role_ids).ok() };
            let cfg = data.db.get_or_create_mod_config(&g).await?;
            data.db.update_mod_config(&g, json.as_deref(), cfg.dm_on_warn, cfg.dm_on_timeout, cfg.dm_on_kick, cfg.dm_on_ban, cfg.appeal_cooldown_days).await?;
            let new_cfg = data.db.get_or_create_mod_config(&g).await?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_setup_mod_form(&new_cfg)).await?;
        }

        // ── DM notification toggles ──────────────────────────────────────────
        "setup:dm:warn" | "setup:dm:timeout" | "setup:dm:kick" | "setup:dm:ban" => {
            let cfg = data.db.get_or_create_mod_config(&g).await?;
            let (w, t, k, b) = (cfg.dm_on_warn, cfg.dm_on_timeout, cfg.dm_on_kick, cfg.dm_on_ban);
            let (w, t, k, b) = match id {
                "setup:dm:warn" => (!w, t, k, b),
                "setup:dm:timeout" => (w, !t, k, b),
                "setup:dm:kick" => (w, t, !k, b),
                _ => (w, t, k, !b),
            };
            data.db.update_mod_config(&g, cfg.staff_role_ids.as_deref(), w, t, k, b, cfg.appeal_cooldown_days).await?;
            let new_cfg = data.db.get_or_create_mod_config(&g).await?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_setup_mod_form(&new_cfg)).await?;
        }

        // ── Appeals channels ─────────────────────────────────────────────────
        "setup:appeals:ch" => {
            let ch = selected_channel(ci);
            data.db.set_appeals_channel(&g, ch.as_deref()).await?;
            let guild_cfg = data.db.get_or_create_guild(&g).await?;
            let mod_cfg = data.db.get_or_create_mod_config(&g).await?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_setup_appeals_form(&guild_cfg, mod_cfg.appeal_cooldown_days)).await?;
        }
        "setup:concerns:ch" => {
            let ch = selected_channel(ci);
            data.db.set_concerns_channel(&g, ch.as_deref()).await?;
            let guild_cfg = data.db.get_or_create_guild(&g).await?;
            let mod_cfg = data.db.get_or_create_mod_config(&g).await?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_setup_appeals_form(&guild_cfg, mod_cfg.appeal_cooldown_days)).await?;
        }

        // ── "Open text modal" buttons for numeric fields ─────────────────────
        "setup:num:appeal_cooldown" => {
            let cfg = data.db.get_or_create_mod_config(&g).await?;
            let modal = Modal::new("m:setup:appeal_cooldown", "⏳ Appeal Cooldown").text_row(
                "appeal_cooldown",
                "Days between appeals",
                TextInputStyle::Short,
                "e.g. 7",
                true,
                Some(&cfg.appeal_cooldown_days.to_string()),
            );
            ui::open_modal(&ctx.http, ci, &modal).await?;
        }
        "setup:num:raid_slowmode" => {
            let cfg = data.db.get_or_create_raid_config(&g).await?;
            let modal = Modal::new("m:setup:raid_slowmode", "⏱️ Raid Slowmode Duration").text_row(
                "raid_slowmode",
                "Slowmode duration (seconds)",
                TextInputStyle::Short,
                "e.g. 30",
                true,
                Some(&cfg.slowmode_secs.to_string()),
            );
            ui::open_modal(&ctx.http, ci, &modal).await?;
        }
        "setup:num:slowmode_config" => {
            let cfg = data.db.get_or_create_slowmode_config(&g).await?;
            let modal = Modal::new("m:setup:slowmode_config", "⏱️ Auto-Slowmode Thresholds")
                .text_row(
                    "slowmode_window",
                    "Check window (seconds)",
                    TextInputStyle::Short,
                    "Messages in this window trigger slowmode (e.g. 10)",
                    true,
                    Some(&cfg.window_secs.to_string()),
                )
                .text_row(
                    "slowmode_capacity",
                    "Message threshold",
                    TextInputStyle::Short,
                    "Number of messages before slowmode kicks in (e.g. 20)",
                    true,
                    Some(&cfg.capacity.to_string()),
                );
            ui::open_modal(&ctx.http, ci, &modal).await?;
        }

        // ── Raid sensitivity dropdown ─────────────────────────────────────────
        "setup:raid:sensitivity" => {
            let sens = if let serenity::ComponentInteractionDataKind::StringSelect { values } = &ci.data.kind {
                values.first().map(|s| s.as_str()).unwrap_or("medium").to_string()
            } else {
                "medium".to_string()
            };
            let preset = match sens.as_str() {
                "low" => &crate::features::raid::SENSITIVITY_LOW,
                "high" => &crate::features::raid::SENSITIVITY_HIGH,
                _ => &crate::features::raid::SENSITIVITY_MEDIUM,
            };
            let cfg = data.db.get_or_create_raid_config(&g).await?;
            data.db.update_raid_config(&g, preset.threshold, preset.decay_rate, cfg.slowmode_secs).await?;
            let new_cfg = data.db.get_or_create_raid_config(&g).await?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_setup_raid_form(&new_cfg)).await?;
        }

        // ── Slowmode toggle ──────────────────────────────────────────────────
        "setup:slowmode:toggle" => {
            let cfg = data.db.get_or_create_slowmode_config(&g).await?;
            data.db.update_slowmode_config(&g, !cfg.enabled, cfg.window_secs, cfg.capacity).await?;
            let new_cfg = data.db.get_or_create_slowmode_config(&g).await?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_setup_slowmode_form(&new_cfg)).await?;
        }

        _ => {
            ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge).await?;
        }
    }

    Ok(())
}
