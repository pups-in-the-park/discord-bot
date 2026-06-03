use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::{parse_concern_modal, CONCERN_REASON_FIELD};
use crate::util::modal_field;

/// The concern modal was submitted (`m:con:{kind}:{source_id}:{guild_id}`).
/// Creates the concern and posts the admin card.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let (kind, source_id, guild_id) = parse_concern_modal(&mi.data.custom_id)
        .ok_or_else(|| anyhow::anyhow!("Invalid concern modal id"))?;
    let guild_id = serenity::GuildId::new(guild_id);

    let reason = modal_field(&mi.data.components, CONCERN_REASON_FIELD)
        .unwrap_or("")
        .to_string();

    let concern = data
        .db
        .create_concern(&guild_id.to_string(), &mi.user.id.to_string(), &kind, source_id, &reason)
        .await?;

    let guild_cfg = data.db.get_or_create_guild(&guild_id.to_string()).await?;
    if let Some(concerns_ch) = guild_cfg.concerns_channel_id.and_then(|s| s.parse::<u64>().ok()) {
        super::super::view::post_concern_card(
            ctx,
            serenity::ChannelId::new(concerns_ch),
            &concern,
            mi.user.id,
        )
        .await;
    }

    mi.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content("Your concern has been submitted to the administration team."),
        ),
    )
    .await?;
    Ok(())
}
