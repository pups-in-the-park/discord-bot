//! Modal-submit handlers for tickets: opening, closing, category create/edit, the
//! in-form category config modals, and intake-form-field creation.

pub mod category_basic;
pub mod category_create;
pub mod category_edit;
pub mod category_num;
pub mod category_thread;
pub mod category_welcome;
pub mod close;
pub mod ctx_open;
pub mod form_field_add;
pub mod open;

use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ui;

use super::view::build_category_config_form;

/// Re-read the category and its roles, then update the configure form in-place.
/// Shared by the category-config modals (basic/welcome/thread/num).
pub(crate) async fn refresh_category_form(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
    type_id: i64,
) -> Result<(), anyhow::Error> {
    let cat = data
        .db
        .get_ticket_type_by_id(type_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Category not found"))?;
    let roles = data.db.get_type_roles(type_id).await?;
    ui::update(&ctx.http, mi.id, &mi.token, &build_category_config_form(&cat, &roles)).await
}
