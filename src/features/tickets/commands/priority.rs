use crate::context::{Context, Error, Priority};
use crate::permissions::require_mod_staff;

#[derive(Debug, poise::ChoiceParameter)]
pub enum PriorityChoice {
    #[name = "Low"]
    Low,
    #[name = "Normal"]
    Normal,
    #[name = "High"]
    High,
    #[name = "Urgent"]
    Urgent,
}

/// Set the priority of this ticket.
#[poise::command(slash_command, guild_only)]
pub async fn priority(
    ctx: Context<'_>,
    #[description = "Priority level"] level: PriorityChoice,
) -> Result<(), Error> {
    require_mod_staff(&ctx).await?;
    let channel_id = ctx.channel_id();

    let ticket = ctx
        .data()
        .db
        .get_ticket_by_thread(&channel_id.to_string())
        .await?
        .ok_or_else(|| Error::user("This command can only be used inside a ticket thread."))?;

    let p = match level {
        PriorityChoice::Low => Priority::Low,
        PriorityChoice::Normal => Priority::Normal,
        PriorityChoice::High => Priority::High,
        PriorityChoice::Urgent => Priority::Urgent,
    };

    ctx.data().db.set_priority(ticket.id, p.as_str()).await?;

    ctx.send(
        poise::CreateReply::default()
            .content(format!("{} Priority set to **{}**.", p.emoji(), p.label())),
    )
    .await?;

    // Reflect the new priority on the in-thread ticket card.
    if let Ok(Some(fresh)) = ctx.data().db.get_ticket_by_id(ticket.id).await {
        super::super::service::refresh_ticket_card(&ctx.serenity_context().http, &ctx.data(), &fresh)
            .await
            .ok();
    }
    Ok(())
}
