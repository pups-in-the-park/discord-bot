use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::{
    BotData, CID_CAT_HUB_BACK, CID_CAT_HUB_CREATE, CID_CAT_HUB_OPEN, CID_CAT_HUB_SELECT,
};
use crate::ui;
use crate::util::respond_ephemeral;

use super::super::view::{build_category_config_form, build_category_create_modal, build_category_hub};

/// The category management hub (`cat:hub:*`): open it, go back to the list, pick a
/// category to configure, or create a new one.
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

    match ci.data.custom_id.as_str() {
        // Opened from the setup hub: post the category hub as a fresh ephemeral.
        CID_CAT_HUB_OPEN => {
            let cats = data.db.get_ticket_types(&g).await?;
            ui::respond_ephemeral(&ctx.http, ci, &build_category_hub(&cats)).await?;
        }
        // Back to the list from a config form: update in place.
        CID_CAT_HUB_BACK => {
            let cats = data.db.get_ticket_types(&g).await?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_category_hub(&cats)).await?;
        }
        // A category was picked: show its configure form in place.
        CID_CAT_HUB_SELECT => {
            let type_id: Option<i64> = if let serenity::ComponentInteractionDataKind::StringSelect { values } = &ci.data.kind {
                values.first().and_then(|s| s.parse().ok())
            } else {
                None
            };
            let Some(type_id) = type_id else {
                return Ok(());
            };
            let Some(cat) = data.db.get_ticket_type_by_id(type_id).await? else {
                respond_ephemeral(ctx, ci, "That category no longer exists.").await;
                return Ok(());
            };
            let roles = data.db.get_type_roles(type_id).await?;
            let fields = data.db.get_form_fields(type_id).await?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_category_config_form(&cat, &roles, &fields)).await?;
        }
        // Create a new category: open the shared create modal (step 1 of the wizard).
        CID_CAT_HUB_CREATE => {
            ci.create_response(
                &ctx.http,
                serenity::CreateInteractionResponse::Modal(build_category_create_modal()),
            )
            .await?;
        }
        _ => {
            ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge).await?;
        }
    }
    Ok(())
}
