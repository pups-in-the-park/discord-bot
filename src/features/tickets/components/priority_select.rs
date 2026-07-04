use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::util::respond_ephemeral;

/// Priority string-select on a ticket card (staff only). Saves and re-renders
/// the card in place so the status line and default selection stay current.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
    ticket_id: i64,
) -> Result<(), anyhow::Error> {
    let serenity::ComponentInteractionDataKind::StringSelect { values } = &ci.data.kind else {
        return Ok(());
    };
    let Some(guild_id) = ci.guild_id else {
        return Ok(());
    };
    if !crate::permissions::is_mod_staff(ctx, data, guild_id, ci.user.id).await {
        respond_ephemeral(ctx, ci, "Only staff can set a ticket's priority.").await;
        return Ok(());
    }

    let priority = values.first().map(|s| s.as_str()).unwrap_or("normal");
    data.db.set_priority(ticket_id, priority).await?;

    let refreshed = match data.db.get_ticket_by_id(ticket_id).await? {
        Some(ticket) => super::claim_button::rebuild_card(ctx, data, &ticket).await,
        None => None,
    };
    match refreshed {
        Some(card) => crate::ui::update(&ctx.http, ci.id, &ci.token, &card).await?,
        None => {
            ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge).await?
        }
    }
    Ok(())
}
