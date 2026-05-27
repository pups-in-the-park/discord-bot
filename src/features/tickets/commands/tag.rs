use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};
use crate::permissions::require_mod_staff;

/// Manage ticket tags.
#[poise::command(slash_command, guild_only, subcommands("tag_add", "tag_remove", "tag_list"))]
pub async fn tag(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add a tag to this ticket.
#[poise::command(slash_command, guild_only, rename = "add")]
pub async fn tag_add(
    ctx: Context<'_>,
    #[description = "Tag to add"]
    #[autocomplete = "autocomplete_tag"]
    tag: String,
) -> Result<(), Error> {
    require_mod_staff(&ctx).await?;
    let channel_id = ctx.channel_id();

    let ticket = ctx
        .data()
        .db
        .get_ticket_by_thread(&channel_id.to_string())
        .await?
        .ok_or_else(|| Error::user("This command can only be used inside a ticket thread."))?;

    ctx.data().db.add_tag(ticket.id, &tag, &ctx.author().id.to_string()).await?;

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::GREEN)
                .description(format!("Tag `{}` added.", tag)),
        ),
    )
    .await?;
    Ok(())
}

/// Remove a tag from this ticket.
#[poise::command(slash_command, guild_only, rename = "remove")]
pub async fn tag_remove(
    ctx: Context<'_>,
    #[description = "Tag to remove"]
    #[autocomplete = "autocomplete_ticket_tag"]
    tag: String,
) -> Result<(), Error> {
    require_mod_staff(&ctx).await?;
    let channel_id = ctx.channel_id();

    let ticket = ctx
        .data()
        .db
        .get_ticket_by_thread(&channel_id.to_string())
        .await?
        .ok_or_else(|| Error::user("This command can only be used inside a ticket thread."))?;

    ctx.data().db.remove_tag(ticket.id, &tag).await?;

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::GREEN)
                .description(format!("Tag `{}` removed.", tag)),
        ),
    )
    .await?;
    Ok(())
}

/// Manage guild-wide available tags.
#[poise::command(slash_command, guild_only, rename = "list", default_member_permissions = "MANAGE_GUILD")]
pub async fn tag_list(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let definitions = ctx.data().db.get_tag_definitions(&guild_id.to_string()).await?;

    if definitions.is_empty() {
        ctx.send(
            poise::CreateReply::default().ephemeral(true).embed(
                serenity::CreateEmbed::new()
                    .colour(colours::GREY)
                    .title("Tag Definitions")
                    .description("No tags defined. Use the buttons below to add one."),
            ),
        )
        .await?;
        return Ok(());
    }

    let lines: Vec<String> = definitions
        .iter()
        .map(|t| {
            let color = t.color.as_deref().map(|c| format!(" (#{c})")).unwrap_or_default();
            format!("`{}`{}", t.name, color)
        })
        .collect();

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::BLURPLE)
                .title(format!("Tag Definitions ({})", definitions.len()))
                .description(lines.join(", ")),
        ),
    )
    .await?;
    Ok(())
}

async fn autocomplete_tag(ctx: Context<'_>, partial: &str) -> Vec<serenity::AutocompleteChoice> {
    let Some(guild_id) = ctx.guild_id() else {
        return vec![];
    };
    ctx.data()
        .db
        .get_tag_definitions(&guild_id.to_string())
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|t| t.name.to_lowercase().contains(&partial.to_lowercase()) || partial.is_empty())
        .map(|t| serenity::AutocompleteChoice::new(t.name.clone(), t.name))
        .collect()
}

async fn autocomplete_ticket_tag(ctx: Context<'_>, partial: &str) -> Vec<serenity::AutocompleteChoice> {
    let ticket = ctx
        .data()
        .db
        .get_ticket_by_thread(&ctx.channel_id().to_string())
        .await
        .ok()
        .flatten();
    let tags = if let Some(t) = ticket {
        ctx.data().db.get_tags(t.id).await.unwrap_or_default()
    } else {
        vec![]
    };
    tags.into_iter()
        .filter(|t| partial.is_empty() || t.to_lowercase().contains(&partial.to_lowercase()))
        .map(|t| serenity::AutocompleteChoice::new(t.clone(), t))
        .collect()
}
