use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::parse_mod_kick_modal;
use crate::util::modal_field;

/// "Kick User" context-menu modal submitted (`m:kick:{target_id}`).
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let target_id = parse_mod_kick_modal(&mi.data.custom_id).unwrap_or(0);
    let Some(guild_id) = mi.guild_id else {
        return Ok(());
    };
    let reason = modal_field(&mi.data.components, "reason")
        .unwrap_or("No reason given")
        .to_string();

    guild_id
        .kick_with_reason(ctx, serenity::UserId::new(target_id), &reason)
        .await
        .ok();
    data.db
        .create_infraction(
            &guild_id.to_string(),
            &target_id.to_string(),
            &mi.user.id.to_string(),
            "kick",
            &reason,
            None,
            false,
            None,
        )
        .await?;

    mi.create_response(
        ctx,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content(format!("👢 <@{}> kicked.", target_id)),
        ),
    )
    .await?;
    Ok(())
}
