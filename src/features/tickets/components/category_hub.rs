use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::{
    BotData, CID_CAT_HUB_BACK, CID_CAT_HUB_CREATE, CID_CAT_HUB_OPEN, CID_CAT_HUB_SELECT,
};
use crate::ui;
use crate::util::respond_ephemeral;

use super::super::view::{build_category_basic_modal, build_category_hub, ConfigTab};

/// The category management hub (`cat:hub:*`): open it, go back to the list, pick a
/// category to configure (inline button or select fallback), or create a new one.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
) -> Result<(), anyhow::Error> {
    let Some(guild_id) = ci.guild_id else {
        return Ok(());
    };
    if !crate::permissions::is_mod_staff(ctx, data, guild_id, ci.user.id).await {
        respond_ephemeral(ctx, ci, "Only staff can configure ticket categories.").await;
        return Ok(());
    }
    let g = guild_id.to_string();

    // Per-category "Configure" button (`cat:hub:sel:{id}`): show its config panel.
    // Checked before the exact matches — `CID_CAT_HUB_SELECT` is a prefix of it.
    if let Some(type_id) = crate::ids::parse_cat_hub_configure(&ci.data.custom_id) {
        if data.db.get_ticket_type_by_id(type_id).await?.is_none() {
            respond_ephemeral(ctx, ci, "That category no longer exists.").await;
            return Ok(());
        }
        super::category_config::show_config_form(&ctx.http, data, ci.id, &ci.token, type_id, ConfigTab::Overview)
            .await?;
        return Ok(());
    }

    match ci.data.custom_id.as_str() {
        // Opened from the setup hub: post the category hub as a fresh ephemeral.
        CID_CAT_HUB_OPEN => {
            let cats = data.db.get_ticket_types(&g).await?;
            let counts = data.db.count_form_fields_by_type(&g).await?;
            ui::respond_ephemeral(&ctx.http, ci, &build_category_hub(&cats, &counts)).await?;
        }
        // Back to the list from a config form: update in place.
        CID_CAT_HUB_BACK => {
            let cats = data.db.get_ticket_types(&g).await?;
            let counts = data.db.count_form_fields_by_type(&g).await?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_category_hub(&cats, &counts)).await?;
        }
        // A category was picked from the big-list select fallback: show its panel.
        CID_CAT_HUB_SELECT => {
            let type_id: Option<i64> = if let serenity::ComponentInteractionDataKind::StringSelect { values } = &ci.data.kind {
                values.first().and_then(|s| s.parse().ok())
            } else {
                None
            };
            let Some(type_id) = type_id else {
                return Ok(());
            };
            if data.db.get_ticket_type_by_id(type_id).await?.is_none() {
                respond_ephemeral(ctx, ci, "That category no longer exists.").await;
                return Ok(());
            }
            super::category_config::show_config_form(&ctx.http, data, ci.id, &ci.token, type_id, ConfigTab::Overview)
                .await?;
        }
        // Create a new category: open the single create modal.
        CID_CAT_HUB_CREATE => {
            ci.create_response(
                &ctx.http,
                serenity::CreateInteractionResponse::Modal(build_category_basic_modal(None)),
            )
            .await?;
        }
        _ => {
            ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge).await?;
        }
    }
    Ok(())
}
