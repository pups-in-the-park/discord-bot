use crate::context::{Context, Error};
use crate::permissions::require_mod_staff;

/// Transfer this ticket to a different category.
#[poise::command(slash_command, guild_only)]
pub async fn transfer(
    ctx: Context<'_>,
    #[description = "New category"]
    #[autocomplete = "super::category::autocomplete_category"]
    category: String,
) -> Result<(), Error> {
    require_mod_staff(&ctx).await?;
    let guild_id = ctx.guild_id().unwrap();
    let channel_id = ctx.channel_id();

    let ticket = ctx
        .data()
        .db
        .get_ticket_by_thread(&channel_id.to_string())
        .await?
        .ok_or_else(|| Error::user("This command can only be used inside a ticket thread."))?;

    let new_type = ctx
        .data()
        .db
        .get_ticket_type_by_name(&guild_id.to_string(), &category)
        .await?
        .ok_or_else(|| Error::user(format!("Category '{}' not found.", category)))?;

    ctx.data().db.transfer_ticket(ticket.id, new_type.id).await?;

    ctx.send(
        poise::CreateReply::default()
            .content(format!("🔀 Ticket transferred to **{}**.", new_type.label)),
    )
    .await?;

    // The card shows the category's label/colour — re-render after responding
    // (3s window), like claim/priority do.
    if let Ok(Some(fresh)) = ctx.data().db.get_ticket_by_id(ticket.id).await {
        super::super::service::refresh_ticket_card(&ctx.serenity_context().http, &ctx.data(), &fresh)
            .await
            .ok();
    }
    Ok(())
}
