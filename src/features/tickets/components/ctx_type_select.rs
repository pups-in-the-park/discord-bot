use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::{cid_ctx_open_modal, parse_ctx_type_select};
use crate::util::respond_ephemeral;

use super::super::service::{open_thread, resolve_parent_for_type, OpenThreadOptions};
use super::super::view::build_open_modal;

/// The category select from the "Open ticket for user" context menu
/// (`ctx:sel:{target_user_id}`). Continues the staff-on-behalf flow: opens the
/// intake modal for the chosen category, or the ticket directly when the
/// category has no renderable intake questions.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
) -> Result<(), anyhow::Error> {
    let Some(guild_id) = ci.guild_id else {
        return Ok(());
    };
    let Some(target_user_id) = parse_ctx_type_select(&ci.data.custom_id) else {
        return Ok(());
    };

    let serenity::ComponentInteractionDataKind::StringSelect { values } = &ci.data.kind else {
        return Ok(());
    };
    let ticket_type_id: i64 = values.first().and_then(|s| s.parse().ok()).unwrap_or(0);

    let ticket_type = data
        .db
        .get_ticket_type_by_id(ticket_type_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Ticket type not found"))?;

    // The select message can outlive a category; refuse deactivated ones.
    if !ticket_type.active {
        respond_ephemeral(ctx, ci, "This ticket category is no longer available.").await;
        return Ok(());
    }

    let modal_id = cid_ctx_open_modal(ticket_type_id, target_user_id);
    let fields = data.db.get_form_fields(ticket_type_id).await?;
    if let Some(modal) = build_open_modal(modal_id, &ticket_type, &fields) {
        ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Modal(modal))
            .await?;
        return Ok(());
    }

    // No intake modal: open the ticket directly, as the single-category
    // context-menu path does. No panel interaction to inherit a channel from,
    // so the ticket opens in the category's panel's channel.
    let parent_ch = match resolve_parent_for_type(data, &ticket_type).await {
        Ok(c) => c,
        Err(e) => {
            respond_ephemeral(ctx, ci, &e.to_string()).await;
            return Ok(());
        }
    };

    // Thread creation + card can exceed the 3-second window.
    ci.create_response(
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
            owner_id: serenity::UserId::new(target_user_id),
            parent_channel_id: parent_ch,
            form_responses: None,
            reported_message_id: None,
            reported_message_url: None,
            reported_message_content: None,
            reported_author_id: None,
        },
    )
    .await;

    // Surface open_thread's actionable message instead of a generic dispatch
    // error; we've deferred, so respond via follow-up.
    let content = match opened {
        Ok(opened) => {
            format!("Ticket opened for <@{}>: <#{}>", target_user_id, opened.thread.id)
        }
        Err(e) => e.to_string(),
    };
    ci.create_followup(
        &ctx.http,
        serenity::CreateInteractionResponseFollowup::new().ephemeral(true).content(content),
    )
    .await?;
    Ok(())
}
