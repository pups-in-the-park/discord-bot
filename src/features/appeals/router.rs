//! Appeals feature dispatch (`ap:btn:` button, `m:ap:` modal).

use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids;

/// Component interactions owned by appeals.
pub async fn route_component(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
) -> Option<Result<(), anyhow::Error>> {
    if let Some((infraction_id, guild_id)) = ids::parse_appeal_btn(&ci.data.custom_id) {
        return Some(
            super::components::appeal_button::handle(ctx, data, ci, infraction_id, guild_id).await,
        );
    }
    if let Some((appeal_id, accept)) = ids::parse_appeal_resolve(&ci.data.custom_id) {
        return Some(super::components::resolve::handle(ctx, data, ci, appeal_id, accept).await);
    }
    None
}

/// Modal submissions owned by appeals.
pub async fn route_modal(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Option<Result<(), anyhow::Error>> {
    if mi.data.custom_id.starts_with("m:apr:") {
        return Some(super::modals::resolve::handle(ctx, data, mi).await);
    }
    if mi.data.custom_id.starts_with("m:ap:") {
        return Some(super::modals::submit::handle(ctx, data, mi).await);
    }
    None
}
