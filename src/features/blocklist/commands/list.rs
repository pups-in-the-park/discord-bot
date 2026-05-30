use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};

/// View the current blocklist.
#[poise::command(slash_command, guild_only, default_member_permissions = "MANAGE_GUILD")]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let entries = ctx.data().db.get_blocklist(&guild_id.to_string()).await?;

    if entries.is_empty() {
        ctx.send(
            poise::CreateReply::default().ephemeral(true).embed(
                serenity::CreateEmbed::new()
                    .colour(colours::GREY)
                    .title("Blocklist")
                    .description("No users are currently on the blocklist."),
            ),
        )
        .await?;
        return Ok(());
    }

    let lines: Vec<String> = entries
        .iter()
        .take(20)
        .map(|e| format!("<@{}> · scope: `{}` · {}", e.user_id, e.scope, e.reason))
        .collect();

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::ORANGE)
                .title(format!("Blocklist ({} entries)", entries.len()))
                .description(lines.join("\n")),
        ),
    )
    .await?;
    Ok(())
}
