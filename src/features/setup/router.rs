//! Setup feature dispatch: the top-level section buttons, the in-form
//! selects/toggles (`setup:*`), and the numeric-field modals (`m:setup:*`).

use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::{
    BotData, CID_SETUP_APPEALS, CID_SETUP_LOG_CHANNELS, CID_SETUP_MOD_STAFF, CID_SETUP_RAID,
    CID_SETUP_SLOWMODE, CID_SETUP_TICKET,
};

use super::components::{interaction, section_button};

/// Component interactions owned by setup.
pub async fn route_component(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
) -> Option<Result<(), anyhow::Error>> {
    let id = ci.data.custom_id.as_str();

    if matches!(
        id,
        CID_SETUP_LOG_CHANNELS
            | CID_SETUP_TICKET
            | CID_SETUP_MOD_STAFF
            | CID_SETUP_APPEALS
            | CID_SETUP_RAID
            | CID_SETUP_SLOWMODE
    ) {
        return Some(section_button::handle(ctx, data, ci, id).await);
    }

    if id.starts_with("setup:") {
        return Some(interaction::handle(ctx, data, ci).await);
    }

    None
}

/// Modal submissions owned by setup.
pub async fn route_modal(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Option<Result<(), anyhow::Error>> {
    if mi.data.custom_id.starts_with("m:setup:") {
        return Some(super::modals::numeric::handle(ctx, data, mi).await);
    }
    None
}
