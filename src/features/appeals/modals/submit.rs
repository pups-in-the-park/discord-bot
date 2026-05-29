use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::{parse_appeal_modal, APPEAL_REASON_FIELD};
use crate::util::modal_field;

/// The appeal modal was submitted (`m:ap:{infraction_id}:{guild_id}`). Creates the
/// appeal, opens a private thread, and posts the staff card.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let (infraction_id, guild_id) = parse_appeal_modal(&mi.data.custom_id)
        .ok_or_else(|| anyhow::anyhow!("Invalid appeal modal id"))?;
    let guild_id = serenity::GuildId::new(guild_id);

    let reason = modal_field(&mi.data.components, APPEAL_REASON_FIELD)
        .unwrap_or("")
        .to_string();
    if reason.is_empty() {
        mi.create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content("Please provide a reason for your appeal."),
            ),
        )
        .await?;
        return Ok(());
    }

    let appeal = data
        .db
        .create_appeal(&guild_id.to_string(), infraction_id, &mi.user.id.to_string(), &reason)
        .await?;

    let guild_cfg = data.db.get_or_create_guild(&guild_id.to_string()).await?;
    if let Some(appeals_ch) = guild_cfg.appeals_channel_id.and_then(|s| s.parse::<u64>().ok()) {
        let appeals_ch = serenity::ChannelId::new(appeals_ch);

        let infraction = data
            .db
            .get_infraction_by_id(infraction_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Infraction not found"))?;

        let thread_name = format!("appeal-{:04}-user", appeal.id);
        let thread = appeals_ch
            .create_thread(
                ctx,
                serenity::CreateThread::new(&thread_name)
                    .kind(serenity::ChannelType::PrivateThread)
                    .auto_archive_duration(serenity::AutoArchiveDuration::OneWeek)
                    .invitable(false),
            )
            .await?;

        let card_msg = super::super::view::post_appeal_card(
            ctx,
            appeals_ch,
            &appeal,
            &infraction,
            mi.user.id,
            &reason,
        )
        .await?;

        data.db
            .set_appeal_thread(appeal.id, &thread.id.to_string(), &card_msg.id.to_string())
            .await?;

        super::super::view::post_appeal_thread_intro(
            ctx,
            thread.id,
            &appeal,
            &infraction,
            mi.user.id,
            &reason,
        )
        .await;
    }

    mi.create_response(
        ctx,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new().ephemeral(true).content(format!(
                "Your appeal has been submitted and will be reviewed. Reference: `#appeal-{}`.",
                appeal.id
            )),
        ),
    )
    .await?;
    Ok(())
}
