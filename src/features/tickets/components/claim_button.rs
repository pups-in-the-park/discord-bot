use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::util::respond_ephemeral;

/// "Claim" button on a ticket card (staff only).
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

    data.db.claim_ticket(ticket_id, &ci.user.id.to_string()).await?;

    ci.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .content(format!("✋ <@{}> has claimed this ticket.", ci.user.id)),
        ),
    )
    .await?;
    Ok(())
}
