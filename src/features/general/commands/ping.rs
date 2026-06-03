use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};

/// Check if the bot is online.
#[poise::command(slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::GREEN)
                .title("🏓 Pong!")
                .description("The bot is online and responding."),
        ),
    )
    .await?;
    Ok(())
}
