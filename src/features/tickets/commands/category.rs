use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};
use crate::ui::slash_respond;

use super::super::view::{build_category_config_form, build_category_create_modal, build_category_hub};

/// Manage ticket categories.
#[poise::command(slash_command, guild_only, subcommands("manage", "create", "edit", "delete"))]
pub async fn category(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Create and configure ticket categories from one place.
#[poise::command(slash_command, guild_only, rename = "manage", default_member_permissions = "MANAGE_GUILD")]
pub async fn manage(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let cats = ctx.data().db.get_ticket_types(&guild_id.to_string()).await?;
    slash_respond(ctx, &build_category_hub(&cats)).await?;
    Ok(())
}

/// Quickly create a new ticket category.
#[poise::command(slash_command, guild_only, rename = "create", default_member_permissions = "MANAGE_GUILD")]
pub async fn create(ctx: Context<'_>) -> Result<(), Error> {
    crate::util::modal_response(ctx, build_category_create_modal()).await?;
    Ok(())
}

/// Jump straight to a category's configuration form.
#[poise::command(slash_command, guild_only, rename = "edit", default_member_permissions = "MANAGE_GUILD")]
pub async fn edit(
    ctx: Context<'_>,
    #[description = "Category to edit"]
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

    let roles = ctx.data().db.get_type_roles(cat.id).await?;
    let fields = ctx.data().db.get_form_fields(cat.id).await?;
    slash_respond(ctx, &build_category_config_form(&cat, &roles, &fields)).await?;
    Ok(())
}

/// Delete (deactivate) a ticket category.
#[poise::command(slash_command, guild_only, rename = "delete", default_member_permissions = "MANAGE_GUILD")]
pub async fn delete(
    ctx: Context<'_>,
    #[description = "Category to delete"]
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
                .title("Category Removed")
                .description(format!(
                    "**{}** is no longer available for new tickets. Existing tickets are unaffected, and it's removed from any panels.",
                    cat.label
                )),
        ),
    )
    .await?;
    Ok(())
}

/// Autocomplete over category names (used by `/ticket category edit`, `delete`, and `/ticket transfer`).
pub async fn autocomplete_category<'a>(ctx: Context<'a>, partial: &'a str) -> serenity::CreateAutocompleteResponse<'a> {
    let Some(guild_id) = ctx.guild_id() else {
        return serenity::CreateAutocompleteResponse::new();
    };
    let p = partial.to_lowercase();
    let choices: Vec<_> = ctx
        .data()
        .db
        .get_ticket_types(&guild_id.to_string())
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|t| p.is_empty() || t.name.to_lowercase().contains(&p) || t.label.to_lowercase().contains(&p))
        .map(|t| serenity::AutocompleteChoice::new(t.label, t.name))
        .collect();
    serenity::CreateAutocompleteResponse::new().set_choices(choices)
}
