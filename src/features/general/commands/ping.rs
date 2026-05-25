use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};

/// Check if the bot is online.
#[poise::command(slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    let latency_ms = {
        let shard_manager = ctx.framework().shard_manager();
        let runners = shard_manager.runners.lock().await;
        runners
            .values()
            .next()
            .and_then(|r| r.latency)
            .map(|l| format!("{}ms", l.as_millis()))
            .unwrap_or_else(|| "unknown".into())
    };

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::GREEN)
                .title("🏓 Pong!")
                .description(format!("Latency: {}", latency_ms)),
        ),
    )
    .await?;
    Ok(())
}
