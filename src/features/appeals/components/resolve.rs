use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::{cid_appeal_resolve_modal, APPEAL_RESPONSE_FIELD};
use crate::util::respond_ephemeral;

/// "Accept"/"Deny" button on a staff appeal card or thread intro: gate on staff +
/// open status, then open the response modal that actually resolves the appeal.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
    appeal_id: i64,
    accept: bool,
) -> Result<(), anyhow::Error> {
    let Some(guild_id) = ci.guild_id else {
        return Ok(());
    };
    // Buttons ignore command permissions, so self-gate to mod staff.
    if !crate::permissions::is_mod_staff(ctx, data, guild_id, ci.user.id).await {
        respond_ephemeral(ctx, ci, "Only staff can resolve appeals.").await;
        return Ok(());
    }

    let appeal = data
        .db
        .get_appeal_by_id(appeal_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Appeal not found"))?;

    if appeal.status != "pending" {
        respond_ephemeral(ctx, ci, "This appeal has already been resolved.").await;
        return Ok(());
    }

    let title = if accept { "✅ Accept Appeal" } else { "❌ Deny Appeal" };
    let placeholder = if accept {
        "Let them know their appeal was accepted…"
    } else {
        "Explain why the appeal was denied…"
    };
    ci.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Modal(
            serenity::CreateModal::new(cid_appeal_resolve_modal(appeal_id, accept), title)
                .components(vec![crate::util::modal_input(
                    "Response to send to the user",
                    APPEAL_RESPONSE_FIELD,
                    true,
                    true,
                    Some(placeholder),
                    None,
                )]),
        ),
    )
    .await?;
    Ok(())
}
