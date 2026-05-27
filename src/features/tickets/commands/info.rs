use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error, Priority};
use crate::permissions::require_mod_staff;

/// View info about this ticket.
#[poise::command(slash_command, guild_only)]
pub async fn info(ctx: Context<'_>) -> Result<(), Error> {
    require_mod_staff(&ctx).await?;
    let channel_id = ctx.channel_id();

    let ticket = ctx
        .data()
        .db
        .get_ticket_by_thread(&channel_id.to_string())
        .await?
        .ok_or_else(|| Error::user("This command can only be used inside a ticket thread."))?;

    let type_label = if let Some(tid) = ticket.ticket_type_id {
        ctx.data()
            .db
            .get_ticket_type_by_id(tid)
            .await
            .ok()
            .flatten()
            .map(|t| t.label)
            .unwrap_or_else(|| "Unknown".into())
    } else {
        "Unknown".into()
    };

    let tags = ctx.data().db.get_tags(ticket.id).await.unwrap_or_default();
    let priority = Priority::from_str(&ticket.priority);

    let mut embed = serenity::CreateEmbed::new()
        .colour(colours::BLURPLE)
        .title(format!("Ticket #{:04}", ticket.ticket_number))
        .field("Category", &type_label, true)
        .field("Owner", format!("<@{}>", ticket.owner_id), true)
        .field("Priority", format!("{} {}", priority.emoji(), priority.label()), true)
        .field("Status", ticket.status.to_string(), true)
        .field("Opened", &ticket.created_at, true);

    if let Some(ref claimed) = ticket.claimed_by {
        embed = embed.field("Claimed by", format!("<@{}>", claimed), true);
    }
    if !tags.is_empty() {
        embed = embed.field("Tags", tags.join(", "), false);
    }

    ctx.send(poise::CreateReply::default().ephemeral(true).embed(embed)).await?;
    Ok(())
}
