use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::util::modal_field;

/// `m:cat:create` — create a new ticket category from the create modal.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let Some(guild_id) = mi.guild_id else {
        return Ok(());
    };
    let name = modal_field(&mi.data.components, "cat_name").unwrap_or("").trim().to_string();
    let label = modal_field(&mi.data.components, "cat_label").unwrap_or("").trim().to_string();
    let emoji = modal_field(&mi.data.components, "cat_emoji").filter(|s| !s.is_empty()).map(str::to_string);
    let color = modal_field(&mi.data.components, "cat_color").filter(|s| !s.is_empty()).unwrap_or("5865F2").to_string();
    let description = modal_field(&mi.data.components, "cat_description").filter(|s| !s.is_empty()).map(str::to_string);

    if name.is_empty() || label.is_empty() {
        mi.create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .content("Name and label are required."),
            ),
        )
        .await?;
        return Ok(());
    }

    data.db
        .create_ticket_type(
            &guild_id.to_string(),
            &name,
            &label,
            emoji.as_deref(),
            description.as_deref(),
            &color,
            None,
        )
        .await?;

    mi.create_response(
        ctx,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content(format!("Category **{}** created.", label)),
        ),
    )
    .await?;
    Ok(())
}
