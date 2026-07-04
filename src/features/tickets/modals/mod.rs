//! Modal-submit handlers for tickets: opening, closing, category create/edit, the
//! in-form category config modals, and intake-form-field creation.

pub mod category_basic;
pub mod category_create;
pub mod category_num;
pub mod category_thread;
pub mod category_welcome;
pub mod close;
pub mod ctx_open;
pub mod form_field;
pub mod open;
pub mod panel_basics;
pub mod panel_create;

use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;

use super::view::ConfigTab;

/// Re-read the category and its roles, then update the configure panel in-place
/// on the tab the submitting modal belongs to. Shared by the category-config
/// modals (basic/welcome/thread/num/question); thin wrapper over the
/// components-side renderer.
pub(crate) async fn refresh_category_form(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
    type_id: i64,
    tab: ConfigTab,
) -> Result<(), anyhow::Error> {
    super::components::category_config::show_config_form(&ctx.http, data, mi.id, &mi.token, type_id, tab)
        .await
}
