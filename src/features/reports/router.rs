//! Reports feature dispatch. Each `route_*` inspects an interaction's custom-id
//! and returns `Some(result)` if this feature owns it, or `None` to let the
//! global dispatcher fall through to the next feature / legacy handlers.

use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids;

use super::components::{action_chosen, action_select, dismiss, investigate};
use super::modals;

/// Component (button / select) interactions owned by reports (`rep:` prefix).
pub async fn route_component(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
) -> Option<Result<(), anyhow::Error>> {
    let id = ci.data.custom_id.as_str();

    if let Some(report_id) = ids::parse_report_investigate(id) {
        return Some(investigate::handle(ctx, data, ci, report_id).await);
    }
    if let Some(report_id) = ids::parse_report_dismiss(id) {
        return Some(dismiss::handle(ctx, data, ci, report_id, false).await);
    }
    if let Some(report_id) = ids::parse_report_thread_dismiss(id) {
        return Some(dismiss::handle(ctx, data, ci, report_id, true).await);
    }
    if let Some(report_id) = ids::parse_report_action_chosen(id) {
        return Some(action_chosen::handle(ctx, data, ci, report_id).await);
    }
    if let Some(report_id) = ids::parse_report_action_select(id) {
        return Some(action_select::handle(ctx, data, ci, report_id).await);
    }
    None
}

/// Modal submissions owned by reports (`m:rep`/`m:rusr`/`m:rep_link`/`m:ract`).
pub async fn route_modal(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Option<Result<(), anyhow::Error>> {
    let id = mi.data.custom_id.as_str();

    if id.starts_with("m:rep:") || id.starts_with("m:rusr:") {
        return Some(modals::submit::handle(ctx, data, mi).await);
    }
    if id.starts_with("m:ract:") {
        return Some(modals::action::handle(ctx, data, mi).await);
    }
    None
}
