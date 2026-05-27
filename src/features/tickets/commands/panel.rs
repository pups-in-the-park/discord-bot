use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};
use crate::ui;

use super::super::view::build_panel_cv2;

#[derive(Debug, poise::ChoiceParameter)]
pub enum PanelLayout {
    #[name = "Buttons (up to 5)"]
    Buttons,
    #[name = "Select Menu"]
    Select,
}

impl PanelLayout {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Buttons => "buttons",
            Self::Select => "select",
        }
    }
}

/// Manage ticket panels.
#[poise::command(slash_command, guild_only, subcommands("create", "configure", "delete", "list", "publish"))]
pub async fn panel(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Create a new ticket panel.
#[poise::command(slash_command, guild_only, rename = "create", default_member_permissions = "MANAGE_GUILD")]
pub async fn create(
    ctx: Context<'_>,
    #[description = "Panel title"] title: String,
    #[description = "Description shown below the title (optional)"] description: Option<String>,
    #[description = "Accent colour hex, e.g. 5865F2 (optional)"] color: Option<String>,
    #[description = "How ticket categories are displayed (default: Buttons)"] layout: Option<PanelLayout>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let layout_str = layout.unwrap_or(PanelLayout::Buttons).as_str();
    let color_str = color.as_deref().unwrap_or("5865F2");

    let panel = ctx
        .data()
        .db
        .create_panel(&guild_id.to_string(), &title, description.as_deref(), color_str, layout_str)
        .await?;

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::BLURPLE)
                .title("Panel Created")
                .description(format!(
                    "Panel **{}** created (ID: **{}**). Use `/ticket panel publish` to post it to a channel.",
                    title, panel.id,
                )),
        ),
    )
    .await?;
    Ok(())
}

/// Edit an existing panel's settings.
#[poise::command(slash_command, guild_only, rename = "configure", default_member_permissions = "MANAGE_GUILD")]
pub async fn configure(
    ctx: Context<'_>,
    #[description = "Panel"]
    #[autocomplete = "autocomplete_panel"]
    panel_id: i64,
    #[description = "New title (leave blank to keep current)"] title: Option<String>,
    #[description = "New description (leave blank to keep current)"] description: Option<String>,
    #[description = "New accent colour hex (leave blank to keep current)"] color: Option<String>,
    #[description = "Layout style (leave blank to keep current)"] layout: Option<PanelLayout>,
) -> Result<(), Error> {
    let panel = ctx
        .data()
        .db
        .get_panel(panel_id)
        .await?
        .ok_or_else(|| Error::user("Panel not found."))?;

    let new_title = title.as_deref().unwrap_or(&panel.title);
    let new_desc = description.as_deref().or(panel.description.as_deref());
    let new_color = color.as_deref().unwrap_or(&panel.color);
    let new_layout = layout.map(|l| l.as_str()).unwrap_or(panel.layout.as_str());

    ctx.data().db.update_panel(panel_id, new_title, new_desc, new_color, new_layout).await?;

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::GREEN)
                .title("Panel Updated")
                .description(format!("Panel **{}** updated.", new_title)),
        ),
    )
    .await?;
    Ok(())
}

/// Delete a panel.
#[poise::command(slash_command, guild_only, rename = "delete", default_member_permissions = "MANAGE_GUILD")]
pub async fn delete(
    ctx: Context<'_>,
    #[description = "Panel"]
    #[autocomplete = "autocomplete_panel"]
    panel_id: i64,
) -> Result<(), Error> {
    ctx.data().db.delete_panel(panel_id).await?;

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new().colour(colours::RED).title("Panel Deleted"),
        ),
    )
    .await?;
    Ok(())
}

/// List all panels for this server.
#[poise::command(slash_command, guild_only, rename = "list", default_member_permissions = "MANAGE_GUILD")]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let panels = ctx.data().db.get_panels(&guild_id.to_string()).await?;

    if panels.is_empty() {
        ctx.send(
            poise::CreateReply::default().ephemeral(true).embed(
                serenity::CreateEmbed::new()
                    .colour(colours::GREY)
                    .title("Panels")
                    .description("No panels yet. Use `/ticket panel create` to add one."),
            ),
        )
        .await?;
        return Ok(());
    }

    let lines: Vec<String> = panels
        .iter()
        .map(|p| {
            let layout_label = if p.layout == "select" { "Select Menu" } else { "Buttons" };
            let published = p.message_id.as_ref().map(|_| " · ✅ Published").unwrap_or(" · ❌ Not published");
            format!("**[{}]** {} ({}{})", p.id, p.title, layout_label, published)
        })
        .collect();

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::BLURPLE)
                .title(format!("Panels ({})", panels.len()))
                .description(lines.join("\n")),
        ),
    )
    .await?;
    Ok(())
}

/// Publish a panel to a channel.
#[poise::command(slash_command, guild_only, rename = "publish", default_member_permissions = "MANAGE_GUILD")]
pub async fn publish(
    ctx: Context<'_>,
    #[description = "Panel"]
    #[autocomplete = "autocomplete_panel"]
    panel_id: i64,
    #[description = "Channel to post the panel in"] channel: serenity::GuildChannel,
    #[description = "Category names to include, comma-separated (blank = all)"] categories: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let panel = ctx
        .data()
        .db
        .get_panel(panel_id)
        .await?
        .ok_or_else(|| Error::user("Panel not found."))?;

    let all_types = ctx.data().db.get_ticket_types(&guild_id.to_string()).await?;
    let types = if let Some(ref cats_str) = categories {
        let names: Vec<&str> = cats_str.split(',').map(str::trim).collect();
        let matched: Vec<_> = all_types.into_iter().filter(|t| names.contains(&t.name.as_str())).collect();
        if matched.is_empty() {
            return Err(Error::user("No matching categories found. Check the names and try again."));
        }
        matched
    } else {
        all_types
    };

    if types.is_empty() {
        return Err(Error::user("No ticket categories found. Create one with `/ticket category create` first."));
    }

    // Link types to the panel
    for t in &types {
        ctx.data().db.add_panel_type(panel_id, t.id).await.ok();
    }

    let cv2_tree = build_panel_cv2(&panel, &types);
    let msg = ui::send(&ctx.serenity_context().http, channel.id, &cv2_tree).await?;
    ctx.data().db.update_panel_message(panel_id, &channel.id.to_string(), &msg.id.to_string()).await?;

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::GREEN)
                .title("Panel Published")
                .description(format!("Panel **{}** posted to <#{}>.", panel.title, channel.id)),
        ),
    )
    .await?;
    Ok(())
}

pub async fn autocomplete_panel(ctx: Context<'_>, partial: &str) -> Vec<serenity::AutocompleteChoice> {
    let Some(guild_id) = ctx.guild_id() else {
        return vec![];
    };
    let p = partial.to_lowercase();
    ctx.data()
        .db
        .get_panels(&guild_id.to_string())
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|panel| p.is_empty() || panel.title.to_lowercase().contains(&p))
        .map(|panel| serenity::AutocompleteChoice::new(format!("[{}] {}", panel.id, panel.title), panel.id))
        .collect()
}
