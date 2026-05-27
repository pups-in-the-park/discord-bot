use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error, Priority};
use crate::permissions::require_mod_staff;

/// List open tickets in this server.
#[poise::command(slash_command, guild_only)]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    require_mod_staff(&ctx).await?;
    let guild_id = ctx.guild_id().unwrap();

    let tickets = ctx.data().db.get_open_tickets(&guild_id.to_string()).await?;

    if tickets.is_empty() {
        ctx.send(
            poise::CreateReply::default().ephemeral(true).embed(
                serenity::CreateEmbed::new()
                    .colour(colours::GREEN)
                    .title("Open Tickets")
                    .description("No open tickets."),
            ),
        )
        .await?;
        return Ok(());
    }

    let lines: Vec<String> = tickets
        .iter()
        .take(20)
        .map(|t| {
            let p = Priority::from_str(&t.priority);
            let claimed = t.claimed_by.as_ref().map(|c| format!(" · claimed by <@{}>", c)).unwrap_or_default();
            format!(
                "{} **#{:04}** — <@{}>{} · <#{}>",
                p.emoji(),
                t.ticket_number,
                t.owner_id,
                claimed,
                t.thread_id,
            )
        })
        .collect();

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::BLURPLE)
                .title(format!("Open Tickets ({}/{})", tickets.len().min(20), tickets.len()))
                .description(lines.join("\n")),
        ),
    )
    .await?;
    Ok(())
}
