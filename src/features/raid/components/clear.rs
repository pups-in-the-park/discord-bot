use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::util::respond_ephemeral;

/// "Clear Raid Mode" button on a raid-alert card (staff only).
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
) -> Result<(), anyhow::Error> {
    let Some(guild_id) = ci.guild_id else {
        return Ok(());
    };

    if !crate::permissions::is_mod_staff(ctx, data, guild_id, ci.user.id).await {
        respond_ephemeral(ctx, ci, "Only staff can clear raid mode.").await;
        return Ok(());
    }

    crate::events::clear_raid_mode(ctx, data, guild_id).await;

    ci.create_response(
        ctx,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .content(format!("✅ Raid mode cleared by <@{}>.", ci.user.id)),
        ),
    )
    .await?;
    Ok(())
}
