use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};
use crate::ids::cid_form_field_add_modal;
use crate::ui::slash_respond;

use super::super::view::build_category_config_form;

/// Manage ticket categories.
#[poise::command(
    slash_command,
    guild_only,
    subcommands("create", "configure", "delete", "list", "form_add", "form_remove", "form_list")
)]
pub async fn category(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Create a new ticket category.
#[poise::command(slash_command, guild_only, rename = "create", default_member_permissions = "MANAGE_GUILD")]
pub async fn create(ctx: Context<'_>) -> Result<(), Error> {
    crate::util::modal_response(
        ctx,
        serenity::CreateModal::new("m:cat:create", "Create Ticket Category").components(vec![
            serenity::CreateActionRow::InputText(
                serenity::CreateInputText::new(serenity::InputTextStyle::Short, "Internal Name", "cat_name")
                    .required(true)
                    .placeholder("e.g. general-support"),
            ),
            serenity::CreateActionRow::InputText(
                serenity::CreateInputText::new(serenity::InputTextStyle::Short, "Button Label", "cat_label")
                    .required(true)
                    .placeholder("e.g. General Support"),
            ),
            serenity::CreateActionRow::InputText(
                serenity::CreateInputText::new(serenity::InputTextStyle::Short, "Emoji (optional)", "cat_emoji")
                    .required(false)
                    .placeholder("e.g. 🎫"),
            ),
            serenity::CreateActionRow::InputText(
                serenity::CreateInputText::new(serenity::InputTextStyle::Short, "Colour (hex, optional)", "cat_color")
                    .required(false)
                    .placeholder("e.g. 5865F2"),
            ),
            serenity::CreateActionRow::InputText(
                serenity::CreateInputText::new(serenity::InputTextStyle::Short, "Description (shown in dropdowns)", "cat_description")
                    .required(false)
                    .placeholder("Brief description of what this category is for"),
            ),
        ]),
    )
    .await?;
    Ok(())
}

/// Configure an existing ticket category.
#[poise::command(slash_command, guild_only, rename = "configure", default_member_permissions = "MANAGE_GUILD")]
pub async fn configure(
    ctx: Context<'_>,
    #[description = "Category name"]
    #[autocomplete = "autocomplete_category"]
    name: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let cat = ctx
        .data()
        .db
        .get_ticket_type_by_name(&guild_id.to_string(), &name)
        .await?
        .ok_or_else(|| Error::user(format!("Category '{}' not found.", name)))?;

    let ping_role_ids = ctx.data().db.get_type_roles(cat.id).await?;
    slash_respond(ctx, &build_category_config_form(&cat, &ping_role_ids)).await?;
    Ok(())
}

/// Delete a ticket category.
#[poise::command(slash_command, guild_only, rename = "delete", default_member_permissions = "MANAGE_GUILD")]
pub async fn delete(
    ctx: Context<'_>,
    #[description = "Category name"]
    #[autocomplete = "autocomplete_category"]
    name: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let cat = ctx
        .data()
        .db
        .get_ticket_type_by_name(&guild_id.to_string(), &name)
        .await?
        .ok_or_else(|| Error::user(format!("Category '{}' not found.", name)))?;

    ctx.data().db.deactivate_ticket_type(cat.id).await?;

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::RED)
                .title("Category Deactivated")
                .description(format!("Category '{}' has been deactivated.", name)),
        ),
    )
    .await?;
    Ok(())
}

/// List all ticket categories.
#[poise::command(slash_command, guild_only, rename = "list", default_member_permissions = "MANAGE_GUILD")]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let types = ctx.data().db.get_ticket_types(&guild_id.to_string()).await?;
    if types.is_empty() {
        ctx.send(
            poise::CreateReply::default().ephemeral(true).embed(
                serenity::CreateEmbed::new()
                    .colour(colours::GREY)
                    .title("Ticket Categories")
                    .description("No categories configured. Use `/ticket category create` to add one."),
            ),
        )
        .await?;
        return Ok(());
    }

    let lines: Vec<String> = types
        .iter()
        .map(|t| {
            let emoji = t.emoji.as_deref().map(|e| format!("{} ", e)).unwrap_or_default();
            let form = if t.has_form { " · 📝 Form" } else { "" };
            format!("{}{} — `{}`{}", emoji, t.label, t.name, form)
        })
        .collect();

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::BLURPLE)
                .title(format!("Ticket Categories ({})", types.len()))
                .description(lines.join("\n")),
        ),
    )
    .await?;
    Ok(())
}

#[derive(Debug, poise::ChoiceParameter)]
pub enum FieldStyle {
    #[name = "Short (single line)"]
    Short,
    #[name = "Paragraph (multi-line)"]
    Paragraph,
}

/// Add a form field to a category.
#[poise::command(slash_command, guild_only, rename = "form-add", default_member_permissions = "MANAGE_GUILD")]
pub async fn form_add(
    ctx: Context<'_>,
    #[description = "Category name"]
    #[autocomplete = "autocomplete_category"]
    name: String,
    #[description = "Input style (default: Short)"] style: Option<FieldStyle>,
    #[description = "Whether the field is required (default: yes)"] required: Option<bool>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let cat = ctx
        .data()
        .db
        .get_ticket_type_by_name(&guild_id.to_string(), &name)
        .await?
        .ok_or_else(|| Error::user(format!("Category '{}' not found.", name)))?;

    let existing = ctx.data().db.get_form_fields(cat.id).await?;
    if existing.len() >= 5 {
        return Err(Error::user("A category can have at most 5 form fields (Discord modal limit)."));
    }

    let style_str = match style.unwrap_or(FieldStyle::Short) {
        FieldStyle::Short => "short",
        FieldStyle::Paragraph => "paragraph",
    };
    let required_val = required.unwrap_or(true);

    let modal_id = cid_form_field_add_modal(cat.id, style_str, required_val);

    crate::util::modal_response(
        ctx,
        serenity::CreateModal::new(modal_id, "Add Form Field").components(vec![
            serenity::CreateActionRow::InputText(
                serenity::CreateInputText::new(serenity::InputTextStyle::Short, "Field Label", "ff_label")
                    .required(true)
                    .placeholder("e.g. Describe your issue"),
            ),
            serenity::CreateActionRow::InputText(
                serenity::CreateInputText::new(serenity::InputTextStyle::Short, "Placeholder text (optional)", "ff_placeholder")
                    .required(false),
            ),
            serenity::CreateActionRow::InputText(
                serenity::CreateInputText::new(serenity::InputTextStyle::Short, "Max length (optional)", "ff_max_length")
                    .required(false),
            ),
        ]),
    )
    .await?;
    Ok(())
}

/// Remove a form field from a category.
#[poise::command(slash_command, guild_only, rename = "form-remove", default_member_permissions = "MANAGE_GUILD")]
pub async fn form_remove(
    ctx: Context<'_>,
    #[description = "Category name"]
    #[autocomplete = "autocomplete_category"]
    name: String,
    #[description = "Field label to remove"]
    #[autocomplete = "autocomplete_form_field"]
    field_label: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let cat = ctx
        .data()
        .db
        .get_ticket_type_by_name(&guild_id.to_string(), &name)
        .await?
        .ok_or_else(|| Error::user(format!("Category '{}' not found.", name)))?;

    let fields = ctx.data().db.get_form_fields(cat.id).await?;
    let was_last = fields.len() == 1;
    let field = fields
        .iter()
        .find(|f| f.label == field_label)
        .ok_or_else(|| Error::user("Field not found."))?;

    ctx.data().db.remove_form_field(cat.id, field.id).await?;
    if was_last {
        ctx.data().db.set_ticket_type_has_form(cat.id, false).await?;
    }

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::GREEN)
                .title("Form Field Removed")
                .description(format!("Field '{}' removed from category '{}'.", field_label, name)),
        ),
    )
    .await?;
    Ok(())
}

/// List form fields for a category.
#[poise::command(slash_command, guild_only, rename = "form-list", default_member_permissions = "MANAGE_GUILD")]
pub async fn form_list(
    ctx: Context<'_>,
    #[description = "Category name"]
    #[autocomplete = "autocomplete_category"]
    name: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let cat = ctx
        .data()
        .db
        .get_ticket_type_by_name(&guild_id.to_string(), &name)
        .await?
        .ok_or_else(|| Error::user(format!("Category '{}' not found.", name)))?;

    let fields = ctx.data().db.get_form_fields(cat.id).await?;

    if fields.is_empty() {
        ctx.send(
            poise::CreateReply::default().ephemeral(true).embed(
                serenity::CreateEmbed::new()
                    .colour(colours::GREY)
                    .title(format!("Form Fields — {}", name))
                    .description("No form fields. Use `/ticket category form-add` to add one."),
            ),
        )
        .await?;
        return Ok(());
    }

    let lines: Vec<String> = fields
        .iter()
        .map(|f| {
            format!(
                "{}. **{}** ({}, {})",
                f.position + 1,
                f.label,
                f.style,
                if f.required { "required" } else { "optional" }
            )
        })
        .collect();

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::BLURPLE)
                .title(format!("Form Fields — {} ({}/5)", name, fields.len()))
                .description(lines.join("\n")),
        ),
    )
    .await?;
    Ok(())
}

pub async fn autocomplete_category(ctx: Context<'_>, partial: &str) -> Vec<serenity::AutocompleteChoice> {
    let Some(guild_id) = ctx.guild_id() else {
        return vec![];
    };
    let p = partial.to_lowercase();
    ctx.data()
        .db
        .get_ticket_types(&guild_id.to_string())
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|t| p.is_empty() || t.name.to_lowercase().contains(&p) || t.label.to_lowercase().contains(&p))
        .map(|t| serenity::AutocompleteChoice::new(t.label, t.name))
        .collect()
}

async fn autocomplete_form_field(ctx: Context<'_>, partial: &str) -> Vec<serenity::AutocompleteChoice> {
    let Some(guild_id) = ctx.guild_id() else {
        return vec![];
    };
    let poise::Context::Application(app) = ctx else {
        return vec![];
    };

    // Read the already-typed "name" param from the interaction to scope the field list.
    let cat_name = app
        .interaction
        .data
        .options
        .iter()
        .find(|o| o.name == "name")
        .and_then(|o| {
            if let serenity::CommandDataOptionValue::String(s) = &o.value {
                Some(s.clone())
            } else {
                None
            }
        });
    let Some(cat_name) = cat_name else {
        return vec![];
    };

    let Ok(Some(cat)) = ctx.data().db.get_ticket_type_by_name(&guild_id.to_string(), &cat_name).await else {
        return vec![];
    };

    ctx.data()
        .db
        .get_form_fields(cat.id)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|f| partial.is_empty() || f.label.to_lowercase().contains(&partial.to_lowercase()))
        .map(|f| serenity::AutocompleteChoice::new(f.label.clone(), f.label))
        .collect()
}
