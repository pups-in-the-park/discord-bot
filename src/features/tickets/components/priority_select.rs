use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;

/// Priority string-select on a ticket card.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
    ticket_id: i64,
) -> Result<(), anyhow::Error> {
    let serenity::ComponentInteractionDataKind::StringSelect { values } = &ci.data.kind else {
        return Ok(());
    };
    let priority = values.first().map(|s| s.as_str()).unwrap_or("normal");
    data.db.set_priority(ticket_id, priority).await?;

    ci.create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
        .await?;
    Ok(())
}
