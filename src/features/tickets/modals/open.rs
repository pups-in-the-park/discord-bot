use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::parse_open_modal;
use crate::util::respond_ephemeral_modal;

use super::super::service::{
    collect_form_responses, open_thread, ticket_open_block_reason, OpenThreadOptions,
};

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

    // The panel button check ran before this modal opened, so re-check here:
    // otherwise rapidly re-clicking the button (or a block added meanwhile) could
    // submit several forms and bypass the per-user limit / blocklist.
    if let Some(reason) = ticket_open_block_reason(data, guild_id, mi.user.id, &ticket_type).await? {
        respond_ephemeral_modal(ctx, mi, &reason).await;
        return Ok(());
    }

    let fields = data.db.get_form_fields(ticket_type_id).await?;
    let responses = collect_form_responses(&mi.data.components, &fields);

    // The modal was submitted from the panel message, so its channel is the
    // panel's channel — the ticket opens there.
    let parent_ch = mi.channel_id.expect_channel();

    // Creating the thread + card can exceed the 3-second window — acknowledge first.
    mi.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Defer(
            serenity::CreateInteractionResponseMessage::new().ephemeral(true),
        ),
    )
    .await?;

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
    .await;

    // Surface open_thread's actionable message (e.g. a missing permission) rather
    // than letting `?` replace it with a generic dispatch error. We've deferred,
    // so respond via follow-up.
    let content = match opened {
        Ok(opened) => format!("Your ticket has been created: <#{}>", opened.thread.id),
        Err(e) => e.to_string(),
    };
    mi.create_followup(
        &ctx.http,
        serenity::CreateInteractionResponseFollowup::new().ephemeral(true).content(content),
    )
    .await?;
    Ok(())
}
