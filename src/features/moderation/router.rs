//! Moderation feature dispatch. Reason modals fired by the context menus
//! (`m:warn`/`m:timeout`/`m:kick`/`m:ban`/`m:dw`). There are no component
//! interactions — the actions run from slash commands and context menus directly.

use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;

use super::modals;

/// Modal submissions owned by moderation.
pub async fn route_modal(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Option<Result<(), anyhow::Error>> {
    let id = mi.data.custom_id.as_str();

    if id.starts_with("m:warn:") {
        return Some(modals::warn::handle(ctx, data, mi).await);
    }
    if id.starts_with("m:timeout:") {
        return Some(modals::timeout::handle(ctx, data, mi).await);
    }
    if id.starts_with("m:kick:") {
        return Some(modals::kick::handle(ctx, data, mi).await);
    }
    if id.starts_with("m:ban:") {
        return Some(modals::ban::handle(ctx, data, mi).await);
    }
    if id.starts_with("m:dw:") {
        return Some(modals::delete_and_warn::handle(ctx, data, mi).await);
    }
    None
}
