use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ui;
use crate::util::respond_ephemeral;

/// "Claim" button on a ticket card (staff only). Records the claim, re-renders
/// the card in place (claimer shown, button disabled until `/ticket unclaim`),
/// and announces the claim in the thread.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
    ticket_id: i64,
) -> Result<(), anyhow::Error> {
    let Some(guild_id) = ci.guild_id else {
        return Ok(());
    };
    if !crate::permissions::is_mod_staff(ctx, data, guild_id, ci.user.id).await {
        respond_ephemeral(ctx, ci, "Only staff can claim tickets.").await;
        return Ok(());
    }

    let Some(ticket) = data.db.get_ticket_by_id(ticket_id).await? else {
        respond_ephemeral(ctx, ci, "That ticket no longer exists.").await;
        return Ok(());
    };
    if let Some(ref claimer) = ticket.claimed_by {
        // A stale card (e.g. pre-refresh) can still have an active button.
        respond_ephemeral(ctx, ci, &format!("Already claimed by <@{}>.", claimer)).await;
        return Ok(());
    }

    if !data.db.claim_ticket(ticket_id, &ci.user.id.to_string()).await? {
        // Lost the race — someone else claimed it between our read and write.
        respond_ephemeral(ctx, ci, "This ticket was just claimed by someone else.").await;
        return Ok(());
    }

    // The clicked message *is* the ticket card — rebuild it in place, then
    // announce the claim as a follow-up so the thread has a visible record.
    let ticket = data
        .db
        .get_ticket_by_id(ticket_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Ticket vanished mid-claim"))?;
    match rebuild_card(ctx, data, &ticket).await {
        Some(card) => ui::update(&ctx.http, ci.id, &ci.token, &card).await?,
        None => {
            ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge).await?
        }
    }
    ci.create_followup(
        &ctx.http,
        serenity::CreateInteractionResponseFollowup::new()
            .content(format!("✋ <@{}> has claimed this ticket.", ci.user.id)),
    )
    .await
    .ok();
    Ok(())
}

/// Build the refreshed card for an interaction-response update, or `None` when
/// the category is gone (deactivated mid-flight). Shared with the priority select.
pub(super) async fn rebuild_card(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ticket: &crate::db::Ticket,
) -> Option<Vec<ui::Component>> {
    let cat = data.db.get_ticket_type_by_id(ticket.ticket_type_id?).await.ok()??;
    let owner_id: u64 = ticket.owner_id.parse().ok()?;
    let username = serenity::UserId::new(owner_id)
        .to_user(ctx)
        .await
        .map(|u| u.name.to_string())
        .unwrap_or_else(|_| "user".to_string());
    Some(super::super::view::build_ticket_card(&cat, ticket, &username))
}
