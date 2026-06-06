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
            &ctx.http,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content("Please provide a reason for your appeal."),
            ),
        )
        .await?;
        return Ok(());
    }

    // Creating the appeal, opening the review thread, and posting two cards far
    // exceeds Discord's 3-second window — acknowledge first, confirm via follow-up.
    mi.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Defer(
            serenity::CreateInteractionResponseMessage::new().ephemeral(true),
        ),
    )
    .await?;

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
                &ctx.http,
                serenity::CreateThread::new(thread_name)
                    .kind(serenity::ChannelType::PrivateThread)
                    .auto_archive_duration(serenity::AutoArchiveDuration::OneWeek)
                    .invitable(false),
            )
            .await?;

        let thread_ch = serenity::ChannelId::new(thread.id.get());

        // Post the staff card (which links the thread + carries Accept/Deny buttons).
        // If anything here fails, delete the freshly-made thread so we don't strand an
        // empty private thread that no card points to.
        let card_msg = match super::super::view::post_appeal_card(
            ctx,
            appeals_ch,
            &appeal,
            &infraction,
            mi.user.id,
            &reason,
            thread_ch,
        )
        .await
        {
            Ok(msg) => msg,
            Err(e) => {
                thread.id.widen().delete(&ctx.http, None).await.ok();
                return Err(e);
            }
        };

        if let Err(e) = data
            .db
            .set_appeal_thread(appeal.id, &thread.id.to_string(), &card_msg.id.to_string())
            .await
        {
            thread.id.widen().delete(&ctx.http, None).await.ok();
            return Err(e.into());
        }

        super::super::view::post_appeal_thread_intro(
            ctx,
            thread_ch,
            &appeal,
            &infraction,
            mi.user.id,
            &reason,
        )
        .await;
    }

    mi.create_followup(
        &ctx.http,
        serenity::CreateInteractionResponseFollowup::new().ephemeral(true).content(format!(
            "Your appeal has been submitted and will be reviewed. Reference: `#appeal-{}`.",
            appeal.id
        )),
    )
    .await?;
    Ok(())
}
