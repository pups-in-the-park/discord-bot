use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::parse_mod_timeout_modal;
use crate::util::modal_field;

/// "Timeout User" context-menu modal submitted (`m:timeout:{target_id}`).
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let target_id = parse_mod_timeout_modal(&mi.data.custom_id).unwrap_or(0);
    let Some(guild_id) = mi.guild_id else {
        return Ok(());
    };
    let reason = modal_field(&mi.data.components, "reason")
        .unwrap_or("No reason given")
        .to_string();
    let secs: i64 = crate::util::modal_secs(&mi.data.components, "duration").unwrap_or(3600);

    let until = serenity::Timestamp::from_unix_timestamp(
        serenity::Timestamp::now().unix_timestamp() + secs,
    )
    .map_err(|_| anyhow::anyhow!("Invalid timestamp"))?;

    let until_str = until.to_rfc3339();

    guild_id
        .edit_member(
            &ctx.http,
            serenity::UserId::new(target_id),
            serenity::EditMember::new().disable_communication_until(until),
        )
        .await
        .ok();

    data.db
        .create_infraction(
            &guild_id.to_string(),
            &target_id.to_string(),
            &mi.user.id.to_string(),
            "timeout",
            &reason,
            Some(secs),
            true,
            Some(until_str.as_str()),
        )
        .await?;

    mi.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content(format!("⏱️ <@{}> timed out.", target_id)),
        ),
    )
    .await?;
    Ok(())
}
