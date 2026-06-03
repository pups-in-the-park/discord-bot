use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::{cid_appeal_modal, APPEAL_REASON_FIELD};
use crate::util::respond_ephemeral;

/// "Appeal this action" button in a moderation DM: validate eligibility/cooldown,
/// then open the appeal modal.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
    infraction_id: i64,
    guild_id: u64,
) -> Result<(), anyhow::Error> {
    let guild_id = serenity::GuildId::new(guild_id);

    let infraction = data
        .db
        .get_infraction_by_id(infraction_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Infraction not found"))?;

    if !infraction.appealable {
        respond_ephemeral(ctx, ci, "This action is not eligible for appeal.").await;
        return Ok(());
    }

    if data.db.get_appeal_for_infraction(infraction_id).await?.is_some() {
        respond_ephemeral(ctx, ci, "You have already submitted an appeal for this action.").await;
        return Ok(());
    }

    // Cooldown: user must wait X days after the action before they can appeal it.
    let mod_cfg = data.db.get_or_create_mod_config(&guild_id.to_string()).await?;
    if mod_cfg.appeal_cooldown_days > 0 {
        let action_time = chrono::DateTime::parse_from_rfc3339(&infraction.created_at)
            .map(|d| d.timestamp())
            .unwrap_or(0);
        let elapsed_days = (chrono::Utc::now().timestamp() - action_time) / 86400;
        if elapsed_days < mod_cfg.appeal_cooldown_days {
            let remaining = mod_cfg.appeal_cooldown_days - elapsed_days;
            respond_ephemeral(
                ctx,
                ci,
                &format!(
                    "You must wait {} more day(s) after this action was taken before you can appeal it.",
                    remaining
                ),
            )
            .await;
            return Ok(());
        }
    }

    let modal_id = cid_appeal_modal(infraction_id, guild_id.get());
    ci.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Modal(
            serenity::CreateModal::new(modal_id, "📄 Appeal Decision").components(vec![
                crate::util::modal_input(
                    "Why should we reconsider?",
                    APPEAL_REASON_FIELD,
                    true,
                    true,
                    Some("Explain your perspective and why you believe this action was unfair…"),
                    None,
                ),
            ]),
        ),
    )
    .await?;
    Ok(())
}
