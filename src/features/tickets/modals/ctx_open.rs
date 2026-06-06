use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::parse_ctx_open_modal;

use super::super::service::{collect_form_responses, open_thread, OpenThreadOptions};

/// Staff-on-behalf intake modal (`m:ctx:{type_id}:{target_user_id}`): open a
/// ticket for another user.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let (ticket_type_id, target_user_id) =
        parse_ctx_open_modal(&mi.data.custom_id).unwrap_or((0, 0));

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
    let Some(parent_ch) = guild_cfg
        .ticket_channel_id
        .and_then(|s| s.parse::<u64>().ok())
        .map(serenity::ChannelId::new)
    else {
        mi.create_response(
            &ctx.http,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content("No ticket channel is set yet. Configure one in `/setup overview` → Ticket channel before opening tickets."),
            ),
        )
        .await?;
        return Ok(());
    };

    // Thread creation + card + staff auto-add can exceed the 3-second window.
    mi.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Defer(
            serenity::CreateInteractionResponseMessage::new().ephemeral(true),
        ),
    )
    .await?;

    let ticket_number = data.db.next_ticket_number(&guild_id.to_string()).await?;
    let owner_id = serenity::UserId::new(target_user_id);
    open_thread(
        ctx,
        data,
        guild_id,
        OpenThreadOptions {
            ticket_type: &ticket_type,
            ticket_number,
            owner_id,
            parent_channel_id: parent_ch,
            form_responses: if responses.is_empty() { None } else { Some(responses) },
            reported_message_id: None,
            reported_message_url: None,
            reported_message_content: None,
            reported_author_id: None,
        },
    )
    .await?;

    mi.create_followup(
        &ctx.http,
        serenity::CreateInteractionResponseFollowup::new()
            .ephemeral(true)
            .content(format!("Ticket opened for <@{}>.", target_user_id)),
    )
    .await?;
    Ok(())
}
