use crate::context::{Context, Error};

/// Show information about this bot.
#[poise::command(slash_command)]
pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
    // Read-only info, no controls → plain markdown (no embed). See docs/ui-conventions.md.
    ctx.send(
        poise::CreateReply::default().ephemeral(true).content(
            "## pip — Help\n\
            **Tickets**\n\
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
            `/setup overview` — Setup dashboard: what's configured & what's left\n\
            `/ticket category manage` — Create & configure ticket categories\n\
            `/ticket panel manage` — Build & publish ticket panels\n\
            `/blocklist` — Manage the ticket blocklist",
        ),
    )
    .await?;
    Ok(())
}
