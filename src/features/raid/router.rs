//! Raid feature dispatch (`raid:clear` button).

use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::CID_RAID_CLEAR;

/// Component interactions owned by raid.
pub async fn route_component(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
) -> Option<Result<(), anyhow::Error>> {
    if ci.data.custom_id == CID_RAID_CLEAR {
        return Some(super::components::clear::handle(ctx, data, ci).await);
    }
    None
}
