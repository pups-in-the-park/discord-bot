use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ui::{self, Modal, TextInputStyle};
use crate::util::respond_ephemeral;

use super::super::view::build_category_config_form;

/// Every select/button inside the ephemeral category-configure form
/// (`cat:cfg:{type_id}:{field}`). Saves immediately and rebuilds the form in place.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
) -> Result<(), anyhow::Error> {
    let id = ci.data.custom_id.as_str();
    let parts: Vec<&str> = id.split(':').collect();
    let type_id: i64 = parts
        .get(2)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("Invalid category config ID"))?;
    let field = parts.get(3).copied().unwrap_or("");

    let Some(guild_id) = ci.guild_id else {
        return Ok(());
    };
    if !crate::permissions::is_mod_staff(ctx, data, guild_id, ci.user.id).await {
        respond_ephemeral(ctx, ci, "Only staff can configure ticket categories.").await;
        return Ok(());
    }

    async fn refresh(
        ctx: &serenity::Context,
        data: &Arc<BotData>,
        ci: &serenity::ComponentInteraction,
        type_id: i64,
    ) -> Result<(), anyhow::Error> {
        let cat = data
            .db
            .get_ticket_type_by_id(type_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Category not found"))?;
        let roles = data.db.get_type_roles(type_id).await?;
        ui::update(&ctx.http, ci.id, &ci.token, &build_category_config_form(&cat, &roles)).await
    }

    match field {
        // Role select — save ping roles
        "ping_roles" => {
            let role_ids: Vec<String> = if let serenity::ComponentInteractionDataKind::RoleSelect { values, .. } = &ci.data.kind {
                values.iter().map(|r| r.to_string()).collect()
            } else {
                vec![]
            };
            data.db.replace_type_roles(type_id, &role_ids).await?;
            refresh(ctx, data, ci, type_id).await?;
        }

        // Channel select — save staff alert channel
        "alert_ch" => {
            let ch = if let serenity::ComponentInteractionDataKind::ChannelSelect { values } = &ci.data.kind {
                values.first().map(|c| c.to_string())
            } else {
                None
            };
            data.db.set_staff_alert_channel(type_id, ch.as_deref()).await?;
            refresh(ctx, data, ci, type_id).await?;
        }

        // Toggle auto_add_staff
        "auto_staff" => {
            let cat = data.db.get_ticket_type_by_id(type_id).await?
                .ok_or_else(|| anyhow::anyhow!("Category not found"))?;
            data.db.set_ticket_type_auto_add_staff(type_id, !cat.auto_add_staff).await?;
            refresh(ctx, data, ci, type_id).await?;
        }

        // Toggle has_form
        "has_form" => {
            let cat = data.db.get_ticket_type_by_id(type_id).await?
                .ok_or_else(|| anyhow::anyhow!("Category not found"))?;
            data.db.set_ticket_type_has_form(type_id, !cat.has_form).await?;
            refresh(ctx, data, ci, type_id).await?;
        }

        // Open modal for max_open_per_user
        "num_max_open" => {
            let cat = data.db.get_ticket_type_by_id(type_id).await?
                .ok_or_else(|| anyhow::anyhow!("Category not found"))?;
            let modal = Modal::new(format!("m:cat:num:max_open:{}", type_id), "📊 Max Open Tickets Per User")
                .text_row("max_open", "Max tickets per user (minimum 1)", TextInputStyle::Short,
                    "e.g. 1 — 0 or blank = no limit", true, Some(&cat.max_open_per_user.to_string()));
            ui::open_modal(&ctx.http, ci, &modal).await?;
        }

        // Open modal for auto_close_hours
        "num_auto_close" => {
            let cat = data.db.get_ticket_type_by_id(type_id).await?
                .ok_or_else(|| anyhow::anyhow!("Category not found"))?;
            let modal = Modal::new(format!("m:cat:num:auto_close:{}", type_id), "⏰ Auto-Close After Inactivity")
                .text_row("auto_close", "Hours of inactivity before auto-close", TextInputStyle::Short,
                    "e.g. 48 — leave blank or 0 to disable", false,
                    cat.auto_close_hours.map(|h| h.to_string()).as_deref());
            ui::open_modal(&ctx.http, ci, &modal).await?;
        }

        // Open modal for label / emoji / color / description
        "btn_basic" => {
            let cat = data.db.get_ticket_type_by_id(type_id).await?
                .ok_or_else(|| anyhow::anyhow!("Category not found"))?;
            let modal = Modal::new(format!("m:cat:basic:{}", type_id), "🏷️ Edit Basic Info")
                .text_row("cat_label", "Button Label", TextInputStyle::Short, "e.g. General Support", true, Some(&cat.label))
                .text_row("cat_emoji", "Emoji (optional)", TextInputStyle::Short, "e.g. 🎫", false, cat.emoji.as_deref())
                .text_row("cat_color", "Accent Color (hex)", TextInputStyle::Short, "e.g. 5865F2", false, Some(&cat.color))
                .text_row("cat_desc", "Description (shown in select menus)", TextInputStyle::Short,
                    "Brief description of this ticket type", false, cat.description.as_deref());
            ui::open_modal(&ctx.http, ci, &modal).await?;
        }

        // Open modal for welcome_message
        "btn_welcome" => {
            let cat = data.db.get_ticket_type_by_id(type_id).await?
                .ok_or_else(|| anyhow::anyhow!("Category not found"))?;
            let modal = Modal::new(format!("m:cat:welcome:{}", type_id), "💬 Welcome Message")
                .text_row("welcome", "Welcome message", TextInputStyle::Paragraph,
                    "Welcome, {user}! A member of staff will be with you shortly.\nVariables: {user}, {username}, {type}",
                    false, cat.welcome_message.as_deref());
            ui::open_modal(&ctx.http, ci, &modal).await?;
        }

        // Open modal for thread_name_pattern
        "btn_thread" => {
            let cat = data.db.get_ticket_type_by_id(type_id).await?
                .ok_or_else(|| anyhow::anyhow!("Category not found"))?;
            let modal = Modal::new(format!("m:cat:thread:{}", type_id), "📝 Thread Name Pattern")
                .text_row("pattern", "Thread name pattern", TextInputStyle::Short,
                    "ticket-{number}-{username} — Variables: {number}, {username}, {type}",
                    true, Some(&cat.thread_name_pattern));
            ui::open_modal(&ctx.http, ci, &modal).await?;
        }

        _ => {
            ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge).await?;
        }
    }

    Ok(())
}
