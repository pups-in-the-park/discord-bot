use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};
use crate::util::format_infraction_history;

/// View a user's infraction history.
#[poise::command(slash_command, guild_only, default_member_permissions = "MODERATE_MEMBERS")]
pub async fn history(
    ctx: Context<'_>,
    #[description = "User"] user: serenity::User,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let infractions = ctx
        .data()
        .db
        .get_infractions(&guild_id.to_string(), &user.id.to_string())
        .await?;

    if infractions.is_empty() {
        ctx.send(
            poise::CreateReply::default().ephemeral(true).embed(
                serenity::CreateEmbed::new()
                    .colour(colours::GREY)
                    .title(format!("History — {}", user.name))
                    .description("No infractions found."),
            ),
        )
        .await?;
        return Ok(());
    }

    let lines = format_infraction_history(&infractions);

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::ORANGE)
                .title(format!("History — {} ({} total)", user.name, infractions.len()))
                .description(lines.join("\n\n")),
        ),
    )
    .await?;
    Ok(())
}
