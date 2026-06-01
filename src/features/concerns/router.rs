//! Concerns feature dispatch (`con:btn:`, `con:rev:` buttons; `m:con:` modal).

use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids;

/// Component interactions owned by concerns.
pub async fn route_component(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
) -> Option<Result<(), anyhow::Error>> {
    let id = ci.data.custom_id.as_str();

    if let Some((kind, source_id, guild_id)) = ids::parse_concern_btn(id) {
        return Some(super::components::report::handle(ctx, ci, kind, source_id, guild_id).await);
    }
    if let Some(concern_id) = ids::parse_concern_reviewed(id) {
        return Some(super::components::reviewed::handle(ctx, data, ci, concern_id).await);
    }
    None
}

/// Modal submissions owned by concerns.
pub async fn route_modal(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Option<Result<(), anyhow::Error>> {
    if mi.data.custom_id.starts_with("m:con:") {
        return Some(super::modals::submit::handle(ctx, data, mi).await);
    }
    None
}
