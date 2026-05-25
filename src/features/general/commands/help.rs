use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};

/// Show information about this bot.
#[poise::command(slash_command)]
pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::BLURPLE)
                .title("pip — Help")
                .description(
                    "**Tickets**\n\
                    `/ticket` — Manage tickets (open, close, claim, etc.)\n\n\
                    **Moderation**\n\
                    `/warn`, `/timeout`, `/kick`, `/ban`, `/unban` — Moderation actions\n\
                    `/history` — View a user's infraction history\n\n\
                    **Reports**\n\
                    `/report user` or `/report message` — Submit a report\n\
                    Right-click any message → Apps → Report Message\n\n\
                    **Appeals**\n\
                    Check your DMs for action notifications — they include appeal buttons.\n\n\
                    **Admin**\n\
                    `/setup` — Configure bot settings\n\
                    `/blocklist` — Manage the ticket blocklist",
                ),
        ),
    )
    .await?;
    Ok(())
}
