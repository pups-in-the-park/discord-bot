use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::cid_open_modal;
use crate::util::respond_ephemeral;

use super::super::service::{open_ticket_no_form, ticket_open_block_reason};
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

    // Deactivated category (stale panel button), blocklist, or per-user open
    // limit. Re-checked at intake-form submit too (modals::open), since this
    // runs before the modal is shown.
    if let Some(reason) =
        ticket_open_block_reason(data, guild_id, ci.user.id, &ticket_type).await?
    {
        respond_ephemeral(ctx, ci, &reason).await;
        return Ok(());
    }

    // The intake form exists exactly when the category has renderable questions:
    // `build_open_modal` yields no modal for zero questions (or ones that are all
    // unrenderable, e.g. option-less selects) and we fall through to a direct open.
    let fields = data.db.get_form_fields(ticket_type_id).await?;
    let modal = build_open_modal(cid_open_modal(ticket_type_id), &ticket_type, &fields);
    if let Some(modal) = modal {
        ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Modal(modal))
            .await?;
    } else {
        // Acknowledge so the panel message stays as-is, then follow up ephemerally.
        ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge)
            .await?;

        let followup = |msg: String| {
            serenity::CreateInteractionResponseFollowup::new().ephemeral(true).content(msg)
        };
        // The panel message lives in this channel, so the ticket opens here.
        let parent_ch = ci.channel_id.expect_channel();

        match open_ticket_no_form(ctx, data, guild_id, &ticket_type, ci.user.id, parent_ch).await {
            Ok(opened) => {
                ci.create_followup(
                    &ctx.http,
                    followup(format!("Your ticket has been created: <#{}>", opened.thread.id)),
                )
                .await
                .ok();
            }
            Err(e) => {
                ci.create_followup(&ctx.http, followup(e.to_string())).await.ok();
            }
        }
    }
    Ok(())
}
