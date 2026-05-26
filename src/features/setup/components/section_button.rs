use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::{
    BotData, CID_SETUP_APPEALS, CID_SETUP_LOG_CHANNELS, CID_SETUP_MOD_STAFF, CID_SETUP_RAID,
    CID_SETUP_SLOWMODE, CID_SETUP_TICKET,
};
use crate::ui;
use crate::util::respond_ephemeral;

use super::super::view::{
    build_setup_appeals_form, build_setup_log_form, build_setup_mod_form, build_setup_raid_form,
    build_setup_slowmode_form, build_setup_ticket_form,
};

/// A top-level `/setup` section button: open that section's ephemeral form.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
    section: &str,
) -> Result<(), anyhow::Error> {
    let Some(guild_id) = ci.guild_id else {
        return Ok(());
    };
    let g = guild_id.to_string();

    if !crate::permissions::is_mod_staff(ctx, data, guild_id, ci.user.id).await {
        respond_ephemeral(ctx, ci, "Only staff can configure the bot.").await;
        return Ok(());
    }

    let form = match section {
        CID_SETUP_LOG_CHANNELS => {
            let cfg = data.db.get_or_create_guild(&g).await?;
            build_setup_log_form(&cfg)
        }
        CID_SETUP_TICKET => {
            let cfg = data.db.get_or_create_guild(&g).await?;
            build_setup_ticket_form(&cfg)
        }
        CID_SETUP_MOD_STAFF => {
            let cfg = data.db.get_or_create_mod_config(&g).await?;
            build_setup_mod_form(&cfg)
        }
        CID_SETUP_APPEALS => {
            let guild_cfg = data.db.get_or_create_guild(&g).await?;
            let mod_cfg = data.db.get_or_create_mod_config(&g).await?;
            build_setup_appeals_form(&guild_cfg, mod_cfg.appeal_cooldown_days)
        }
        CID_SETUP_RAID => {
            let cfg = data.db.get_or_create_raid_config(&g).await?;
            build_setup_raid_form(&cfg)
        }
        CID_SETUP_SLOWMODE => {
            let cfg = data.db.get_or_create_slowmode_config(&g).await?;
            build_setup_slowmode_form(&cfg)
        }
        _ => return Ok(()),
    };

    ui::respond_ephemeral(&ctx.http, ci, &form).await?;
    Ok(())
}
