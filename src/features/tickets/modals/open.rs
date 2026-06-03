use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::parse_open_modal;

use super::super::service::{collect_form_responses, open_thread, OpenThreadOptions};

/// Intake-form modal submitted (`m:open:{type_id}`): create the ticket thread.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let ticket_type_id =
        parse_open_modal(&mi.data.custom_id).ok_or_else(|| anyhow::anyhow!("Invalid ticket type ID"))?;

    let Some(guild_id) = mi.guild_id else {
        return Ok(());
    };
    let ticket_type = data
        .db
        .get_ticket_type_by_id(ticket_type_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Ticket type not found"))?;

    let fields = data.db.get_form_fields(ticket_type_id).await?;
    let responses = collect_form_responses(&mi.data.components, &fields);

    let guild_cfg = data.db.get_or_create_guild(&guild_id.to_string()).await?;
    let parent_ch = guild_cfg
        .ticket_channel_id
        .as_ref()
        .and_then(|s| s.parse::<u64>().ok())
        .map(serenity::ChannelId::new)
        .ok_or_else(|| anyhow::anyhow!("Ticket channel not configured"))?;

    let ticket_number = data.db.next_ticket_number(&guild_id.to_string()).await?;
    let opened = open_thread(
        ctx,
        data,
        guild_id,
        OpenThreadOptions {
            ticket_type: &ticket_type,
            ticket_number,
            owner_id: mi.user.id,
            parent_channel_id: parent_ch,
            form_responses: if responses.is_empty() { None } else { Some(responses) },
            reported_message_id: None,
            reported_message_url: None,
            reported_message_content: None,
            reported_author_id: None,
        },
    )
    .await?;

    mi.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content(format!("Your ticket has been created: <#{}>", opened.thread.id)),
        ),
    )
    .await?;
    Ok(())
}
