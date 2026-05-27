use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};
use crate::util::format_infraction_history;

/// View moderation history for this user (staff only).
#[poise::command(context_menu_command = "View History", guild_only, default_member_permissions = "MODERATE_MEMBERS")]
pub async fn view_history(ctx: Context<'_>, target: serenity::User) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let infractions = ctx
        .data()
        .db
        .get_infractions(&guild_id.to_string(), &target.id.to_string())
        .await?;

    if infractions.is_empty() {
        ctx.send(
            poise::CreateReply::default().ephemeral(true).embed(
                serenity::CreateEmbed::new()
                    .colour(colours::GREY)
                    .title(format!("History — {}", target.name))
                    .description("No infractions on record."),
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
                .title(format!("History — {} ({} total)", target.name, infractions.len()))
                .description(lines.join("\n\n")),
        ),
    )
    .await?;
    Ok(())
}
