use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::cid_open_modal;
use crate::util::respond_ephemeral;

use super::super::service::{open_thread, OpenThreadOptions};
use super::super::view::build_open_modal;

/// A panel button or select was used to open a ticket. Validates blocklist/limits,
/// then either opens the intake-form modal or creates the thread directly.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
) -> Result<(), anyhow::Error> {
    let Some(guild_id) = ci.guild_id else {
        return Ok(());
    };

    let ticket_type_id: i64 = if ci.data.custom_id == "p:sel" {
        let serenity::ComponentInteractionDataKind::StringSelect { values } = &ci.data.kind else {
            return Ok(());
        };
        values.first().and_then(|s| s.parse().ok()).unwrap_or(0)
    } else {
        ci.data.custom_id.split(':').nth(2).and_then(|s| s.parse().ok()).unwrap_or(0)
    };

    let ticket_type = data
        .db
        .get_ticket_type_by_id(ticket_type_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Ticket type not found"))?;

    // Check blocklist (category-specific, also covered by the `tickets` umbrella).
    if let Some(block) = data
        .db
        .get_active_block(
            &guild_id.to_string(),
            &ci.user.id.to_string(),
            &format!("ticket:{}", ticket_type.name),
        )
        .await?
    {
        respond_ephemeral(ctx, ci, &crate::features::blocklist::view::blocked_text(&block)).await;
        return Ok(());
    }

    // Check max open
    let open_count = data
        .db
        .count_open_tickets_for_user(&guild_id.to_string(), &ci.user.id.to_string(), ticket_type_id)
        .await?;
    // max_open_per_user of 0 means unlimited.
    if ticket_type.max_open_per_user > 0 && open_count >= ticket_type.max_open_per_user {
        respond_ephemeral(
            ctx,
            ci,
            &format!(
                "You already have {} open ticket(s) of this type. Please wait for them to be resolved.",
                open_count,
            ),
        )
        .await;
        return Ok(());
    }

    // If form, open modal; else open thread directly
    if ticket_type.has_form {
        let fields = data.db.get_form_fields(ticket_type_id).await?;
        let modal = build_open_modal(cid_open_modal(ticket_type_id), &ticket_type, &fields);
        ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Modal(modal))
            .await?;
    } else {
        // Acknowledge so the panel message stays as-is, then follow up ephemerally.
        ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge)
            .await?;

        let guild_cfg = data.db.get_or_create_guild(&guild_id.to_string()).await?;
        let Some(parent_ch) = guild_cfg
            .ticket_channel_id
            .as_ref()
            .and_then(|s| s.parse::<u64>().ok())
            .map(serenity::ChannelId::new)
        else {
            ctx.http
                .create_followup_message(
                    &ci.token,
                    &serde_json::json!({
                        "content": "This server's ticket system isn't finished being set up yet. An admin needs to pick a ticket channel in `/setup overview`.",
                        "flags": 64,
                    }),
                    vec![],
                )
                .await
                .ok();
            return Ok(());
        };

        let ticket_number = data.db.next_ticket_number(&guild_id.to_string()).await?;
        let opened = open_thread(
            ctx,
            data,
            guild_id,
            OpenThreadOptions {
                ticket_type: &ticket_type,
                ticket_number,
                owner_id: ci.user.id,
                parent_channel_id: parent_ch,
                form_responses: None,
                reported_message_id: None,
                reported_message_url: None,
                reported_message_content: None,
                reported_author_id: None,
            },
        )
        .await?;

        ctx.http
            .create_followup_message(
                &ci.token,
                &serde_json::json!({
                    "content": format!("Your ticket has been created: <#{}>", opened.thread.id),
                    "flags": 64,
                }),
                vec![],
            )
            .await
            .ok();
    }
    Ok(())
}
