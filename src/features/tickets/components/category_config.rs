use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ui::{self, Modal, TextInputStyle};
use crate::util::respond_ephemeral;

use super::super::view::{
    build_category_basic_modal, build_category_config_form, build_category_delete_confirm,
    build_category_hub, build_question_modal, ConfigTab,
};

/// Preset choices for "max open tickets per user" (`0` = unlimited).
const MAX_OPEN_PRESETS: &[(&str, i64)] = &[
    ("Unlimited", 0),
    ("1", 1),
    ("2", 2),
    ("3", 3),
    ("5", 5),
    ("10", 10),
];
/// Preset choices for auto-close, in hours (`0` = disabled).
const AUTO_CLOSE_PRESETS: &[(&str, i64)] = &[
    ("Disabled", 0),
    ("12 hours", 12),
    ("24 hours", 24),
    ("48 hours", 48),
    ("72 hours", 72),
    ("1 week", 168),
];

/// Render the category configure panel in place from fresh data, on the given
/// tab. Shared by every flow that lands on the panel (hub buttons, config
/// controls, config modals).
pub(in crate::features::tickets) async fn show_config_form(
    http: &serenity::Http,
    data: &Arc<BotData>,
    interaction_id: serenity::InteractionId,
    token: &str,
    type_id: i64,
    tab: ConfigTab,
) -> Result<(), anyhow::Error> {
    let cat = data
        .db
        .get_ticket_type_by_id(type_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Category not found"))?;
    let roles = data.db.get_type_roles(type_id).await?;
    let fields = data.db.get_form_fields(type_id).await?;
    ui::update(http, interaction_id, token, &build_category_config_form(&cat, &roles, &fields, tab)).await
}

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

    match field {
        // Switch to another tab of the panel: `cat:cfg:{id}:tab:{tab}`.
        "tab" => {
            let tab = ConfigTab::parse(parts.get(4).copied().unwrap_or(""));
            show_config_form(&ctx.http, data, ci.id, &ci.token, type_id, tab).await?;
        }

        // Role select — save staff roles (custom_id keeps the legacy `ping_roles` name)
        "ping_roles" => {
            let role_ids: Vec<String> = if let serenity::ComponentInteractionDataKind::RoleSelect { values, .. } = &ci.data.kind {
                values.iter().map(|r| r.to_string()).collect()
            } else {
                vec![]
            };
            data.db.replace_type_roles(type_id, &role_ids).await?;
            show_config_form(&ctx.http, data, ci.id, &ci.token, type_id, ConfigTab::Behaviour).await?;
        }

        // Channel select — where this category's tickets open (clear = use panel)
        "ticket_ch" => {
            let ch = if let serenity::ComponentInteractionDataKind::ChannelSelect { values } = &ci.data.kind {
                values.first().map(|c| c.to_string())
            } else {
                None
            };
            data.db.set_ticket_type_channel(type_id, ch.as_deref()).await?;
            show_config_form(&ctx.http, data, ci.id, &ci.token, type_id, ConfigTab::Behaviour).await?;
        }

        // Channel select — save staff alert channel
        "alert_ch" => {
            let ch = if let serenity::ComponentInteractionDataKind::ChannelSelect { values } = &ci.data.kind {
                values.first().map(|c| c.to_string())
            } else {
                None
            };
            data.db.set_staff_alert_channel(type_id, ch.as_deref()).await?;
            show_config_form(&ctx.http, data, ci.id, &ci.token, type_id, ConfigTab::Behaviour).await?;
        }

        // Staff-notification toggles — flip and re-render
        "auto_add" => {
            let cat = data.db.get_ticket_type_by_id(type_id).await?
                .ok_or_else(|| anyhow::anyhow!("Category not found"))?;
            data.db.set_ticket_type_auto_add_staff(type_id, !cat.auto_add_staff).await?;
            show_config_form(&ctx.http, data, ci.id, &ci.token, type_id, ConfigTab::Behaviour).await?;
        }
        "notify" => {
            let cat = data.db.get_ticket_type_by_id(type_id).await?
                .ok_or_else(|| anyhow::anyhow!("Category not found"))?;
            data.db.set_ticket_type_notify_staff(type_id, !cat.notify_staff).await?;
            show_config_form(&ctx.http, data, ci.id, &ci.token, type_id, ConfigTab::Behaviour).await?;
        }

        // Max-open dropdown (0 = unlimited).
        "num_max_open" => {
            let cat = data.db.get_ticket_type_by_id(type_id).await?
                .ok_or_else(|| anyhow::anyhow!("Category not found"))?;
            let modal = serenity::CreateModal::new(format!("m:cat:num:max_open:{}", type_id), "📊 Max Open Tickets Per User")
                .components(vec![crate::util::preset_select(
                    "Max open tickets per user", "max_open", MAX_OPEN_PRESETS, cat.max_open_per_user,
                )]);
            ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Modal(modal)).await?;
        }

        // Auto-close dropdown (0 = disabled).
        "num_auto_close" => {
            let cat = data.db.get_ticket_type_by_id(type_id).await?
                .ok_or_else(|| anyhow::anyhow!("Category not found"))?;
            let modal = serenity::CreateModal::new(format!("m:cat:num:auto_close:{}", type_id), "⏰ Auto-Close After Inactivity")
                .components(vec![crate::util::preset_select(
                    "Close after inactivity", "auto_close", AUTO_CLOSE_PRESETS, cat.auto_close_hours.unwrap_or(0),
                )]);
            ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Modal(modal)).await?;
        }

        // Open the shared basic-info modal (label / emoji / description / colour).
        "btn_basic" => {
            let cat = data.db.get_ticket_type_by_id(type_id).await?
                .ok_or_else(|| anyhow::anyhow!("Category not found"))?;
            ci.create_response(
                &ctx.http,
                serenity::CreateInteractionResponse::Modal(build_category_basic_modal(Some(&cat))),
            )
            .await?;
        }

        // Open modal for welcome_message
        "btn_welcome" => {
            let cat = data.db.get_ticket_type_by_id(type_id).await?
                .ok_or_else(|| anyhow::anyhow!("Category not found"))?;
            let modal = Modal::new(format!("m:cat:welcome:{}", type_id), "💬 Welcome Message")
                .text_row("welcome", "Shown when the ticket opens", TextInputStyle::Paragraph,
                    "Welcome {user}! Staff will be with you shortly. ({user}=mention, {username}=name, {type}=category)",
                    false, cat.welcome_message.as_deref());
            ui::open_modal(&ctx.http, ci, &modal).await?;
        }

        // Add a question: the type select on the Questions tab. Opens the single
        // add-question modal tailored to the chosen type.
        "ff_add" => {
            let style = if let serenity::ComponentInteractionDataKind::StringSelect { values } = &ci.data.kind {
                values.first().map(|s| s.as_str()).unwrap_or("short").to_string()
            } else {
                "short".to_string()
            };
            if data.db.count_form_fields(type_id).await? >= 5 {
                respond_ephemeral(ctx, ci, "This category already has the maximum of 5 questions (Discord's modal limit).").await;
                return Ok(());
            }
            ci.create_response(
                &ctx.http,
                serenity::CreateInteractionResponse::Modal(build_question_modal(type_id, &style, None)),
            )
            .await?;
        }

        // Edit a question: `cat:cfg:{type_id}:ff_edit:{field_id}` — same modal, prefilled.
        "ff_edit" => {
            let field_id: Option<i64> = parts.get(4).and_then(|s| s.parse().ok());
            let Some(f) = (match field_id {
                Some(fid) => data.db.get_form_field(fid).await?,
                None => None,
            }) else {
                respond_ephemeral(ctx, ci, "That question no longer exists.").await;
                return Ok(());
            };
            ci.create_response(
                &ctx.http,
                serenity::CreateInteractionResponse::Modal(build_question_modal(type_id, &f.style, Some(&f))),
            )
            .await?;
        }

        // Remove the selected intake-form field. No flag to maintain — the intake
        // form exists exactly when the category has questions.
        "ff_remove" => {
            let field_id: Option<i64> = if let serenity::ComponentInteractionDataKind::StringSelect { values } = &ci.data.kind {
                values.first().and_then(|s| s.parse().ok())
            } else {
                None
            };
            if let Some(fid) = field_id {
                data.db.remove_form_field(type_id, fid).await?;
            }
            show_config_form(&ctx.http, data, ci.id, &ci.token, type_id, ConfigTab::Questions).await?;
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

        // Delete flow: confirm → deactivate → back to the hub list.
        "delete" => {
            let cat = data.db.get_ticket_type_by_id(type_id).await?
                .ok_or_else(|| anyhow::anyhow!("Category not found"))?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_category_delete_confirm(&cat)).await?;
        }
        "delete_no" => {
            show_config_form(&ctx.http, data, ci.id, &ci.token, type_id, ConfigTab::Overview).await?;
        }
        "delete_yes" => {
            data.db.deactivate_ticket_type(type_id).await?;
            let g = guild_id.to_string();
            let cats = data.db.get_ticket_types(&g).await?;
            let counts = data.db.count_form_fields_by_type(&g).await?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_category_hub(&cats, &counts)).await?;
            // The confirm copy promises the category is removed from panels — refresh
            // the affected live panels (after responding, to stay within the 3s window).
            super::super::service::republish_panels_containing(&ctx.http, data, type_id).await;
        }

        _ => {
            ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge).await?;
        }
    }

    Ok(())
}
