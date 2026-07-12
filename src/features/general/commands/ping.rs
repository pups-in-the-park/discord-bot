use crate::context::{Context, Error};

/// Check if the bot is online.
#[poise::command(slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    // Read-only, no controls → plain markdown (no embed/container). See docs/ui-conventions.md.
    ctx.send(
        poise::CreateReply::default()
            .ephemeral(true)
            .content("🏓 **Pong!** The bot is online and responding."),
    )
    .await?;
    Ok(())
}
