//! Tickets feature dispatch: ticket-card buttons/select (`t:*`, close button),
//! panel open flow (`p:*`), category-configure components (`cat:cfg:`), and the
//! ticket/category modals (`m:open|ctx|close|cat|ff`).

use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::{BotData, CID_CLOSE_BTN, CID_PANEL_SELECT};
use crate::ids;

use super::components::{
    category_config, category_hub, claim_button, close_button, panel_config, panel_hub,
    panel_interaction, priority_select,
};
use super::modals;

/// Component interactions owned by tickets.
pub async fn route_component(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
) -> Option<Result<(), anyhow::Error>> {
    let id = ci.data.custom_id.as_str();

    if id == CID_CLOSE_BTN {
        return Some(close_button::handle(ctx, data, ci).await);
    }
    if let Some(ticket_id) = ids::parse_claim_btn(id) {
        return Some(claim_button::handle(ctx, data, ci, ticket_id).await);
    }
    if let Some(ticket_id) = ids::parse_priority_select(id) {
        return Some(priority_select::handle(ctx, data, ci, ticket_id).await);
    }
    if ids::parse_panel_btn(id).is_some() || id == CID_PANEL_SELECT {
        return Some(panel_interaction::handle(ctx, data, ci).await);
    }
    if id.starts_with("cat:cfg:") {
        return Some(category_config::handle(ctx, data, ci).await);
    }
    if id.starts_with("cat:hub") {
        return Some(category_hub::handle(ctx, data, ci).await);
    }
    if id.starts_with("pnl:hub") {
        return Some(panel_hub::handle(ctx, data, ci).await);
    }
    if id.starts_with("pnl:cfg:") {
        return Some(panel_config::handle(ctx, data, ci).await);
    }
    None
}

/// Modal submissions owned by tickets.
pub async fn route_modal(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Option<Result<(), anyhow::Error>> {
    let id = mi.data.custom_id.as_str();

    let result = if id.starts_with("m:open:") {
        modals::open::handle(ctx, data, mi).await
    } else if id.starts_with("m:ctx:") {
        modals::ctx_open::handle(ctx, data, mi).await
    } else if id.starts_with("m:close:") {
        modals::close::handle(ctx, data, mi).await
    } else if id.starts_with("m:cat:basic:") {
        modals::category_basic::handle(ctx, data, mi).await
    } else if id.starts_with("m:cat:welcome:") {
        modals::category_welcome::handle(ctx, data, mi).await
    } else if id.starts_with("m:cat:thread:") {
        modals::category_thread::handle(ctx, data, mi).await
    } else if id.starts_with("m:cat:num:") {
        modals::category_num::handle(ctx, data, mi).await
    } else if id.starts_with("m:cat:create2:") {
        modals::category_create2::handle(ctx, data, mi).await
    } else if id.starts_with("m:cat:create") {
        modals::category_create::handle(ctx, data, mi).await
    } else if id.starts_with("m:cat:edit:") {
        modals::category_edit::handle(ctx, data, mi).await
    } else if id.starts_with("m:ff:type:") {
        modals::form_field_type::handle(ctx, data, mi).await
    } else if id.starts_with("m:ff:opts:") {
        modals::form_field_options::handle(ctx, data, mi).await
    } else if id == "m:pnl:create" {
        modals::panel_create::handle(ctx, data, mi).await
    } else if id.starts_with("m:pnl:basics:") {
        modals::panel_basics::handle(ctx, data, mi).await
    } else {
        return None;
    };

    Some(result)
}
