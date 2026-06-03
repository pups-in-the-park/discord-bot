use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;

/// "Mark as Reviewed" button on a concern card.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
    concern_id: i64,
) -> Result<(), anyhow::Error> {
    data.db
        .mark_concern_reviewed(concern_id, &ci.user.id.to_string())
        .await?;

    // Acknowledge first, then edit the original message via the HTTP API.
    ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge)
        .await?;
    // Reload so the re-render shows reviewer + reviewed_at, keeping the original body.
    if let Ok(Some(concern)) = data.db.get_concern_by_id(concern_id).await {
        super::super::view::mark_concern_reviewed(
            ctx,
            serenity::ChannelId::new(ci.channel_id.get()),
            ci.message.id,
            &concern,
        )
        .await;
    }
    Ok(())
}
