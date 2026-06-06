use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ui;
use crate::util::{modal_field, sanitise_name};

use super::super::view::build_category_step2_card;

/// `m:cat:create` — create a new ticket category from the create modal, then show
/// its configure form in place. The internal name is auto-derived from the label.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let Some(guild_id) = mi.guild_id else {
        return Ok(());
    };
    let g = guild_id.to_string();

    let label = modal_field(&mi.data.components, "cat_label").unwrap_or("").trim().to_string();
    let emoji = modal_field(&mi.data.components, "cat_emoji").filter(|s| !s.is_empty()).map(str::to_string);
    let description = modal_field(&mi.data.components, "cat_description").filter(|s| !s.is_empty()).map(str::to_string);
    // Accent colour, welcome, pattern, and behaviour are gathered in step 2.
    let color = "5865F2".to_string();

    if label.is_empty() {
        mi.create_response(
            &ctx.http,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content("A name is required to create a category."),
            ),
        )
        .await?;
        return Ok(());
    }

    // Auto-derive a unique internal name from the label.
    let existing = data.db.get_ticket_types(&g).await?;
    let taken: std::collections::HashSet<String> = existing.iter().map(|t| t.name.clone()).collect();
    let base = {
        let s = sanitise_name(&label);
        if s.is_empty() { "category".to_string() } else { s }
    };
    let mut name = base.clone();
    let mut n = 2;
    while taken.contains(&name) {
        name = format!("{}-{}", base, n);
        n += 1;
    }

    data.db
        .create_ticket_type(&g, &name, &label, emoji.as_deref(), description.as_deref(), &color, None)
        .await?;

    // Continue to step 2 of the wizard (appearance & behaviour).
    let Some(cat) = data.db.get_ticket_type_by_name(&g, &name).await? else {
        mi.create_response(
            &ctx.http,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content(format!("Category **{}** created.", label)),
            ),
        )
        .await?;
        return Ok(());
    };
    ui::respond_ephemeral_to(&ctx.http, mi.id, &mi.token, &build_category_step2_card(&cat)).await?;
    Ok(())
}
