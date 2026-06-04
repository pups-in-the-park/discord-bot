//! Top-level interaction dispatch. Every feature owns a `router` that claims its
//! custom-id prefixes; we try them in turn. An interaction matching no router is a
//! stale/unknown component and is silently ignored.

use std::sync::Arc;

use poise::serenity_prelude as serenity;
use tracing::warn;

use crate::context::BotData;
use crate::features;

/// Try each feature router in turn; the first to claim the id wins.
macro_rules! try_route {
    ($($router:expr),+ $(,)?) => {{
        let mut handled = None;
        $(
            if handled.is_none() {
                handled = $router;
            }
        )+
        handled
    }};
}

/// Route a component (button / select) interaction.
pub async fn component(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
) {
    let handled = try_route!(
        features::reports::router::route_component(ctx, data, ci).await,
        features::appeals::router::route_component(ctx, data, ci).await,
        features::concerns::router::route_component(ctx, data, ci).await,
        features::raid::router::route_component(ctx, data, ci).await,
        features::setup::router::route_component(ctx, data, ci).await,
        features::tickets::router::route_component(ctx, data, ci).await,
    );

    if let Some(Err(e)) = handled {
        warn!("Component handler error for '{}': {:?}", ci.data.custom_id, e);
        // The handler may already have deferred/acknowledged (long-running flows do),
        // in which case a fresh response is rejected — fall back to a follow-up so the
        // user never sees a silent failure or a stuck "thinking…" state.
        let failed = ci
            .create_response(
                &ctx.http,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .ephemeral(true)
                        .content("An error occurred. Please try again."),
                ),
            )
            .await
            .is_err();
        if failed {
            ci.create_followup(
                &ctx.http,
                serenity::CreateInteractionResponseFollowup::new()
                    .ephemeral(true)
                    .content("An error occurred. Please try again."),
            )
            .await
            .ok();
        }
    }
}

/// Route a modal-submit interaction.
pub async fn modal(ctx: &serenity::Context, data: &Arc<BotData>, mi: &serenity::ModalInteraction) {
    let handled = try_route!(
        features::reports::router::route_modal(ctx, data, mi).await,
        features::appeals::router::route_modal(ctx, data, mi).await,
        features::concerns::router::route_modal(ctx, data, mi).await,
        features::moderation::router::route_modal(ctx, data, mi).await,
        features::setup::router::route_modal(ctx, data, mi).await,
        features::tickets::router::route_modal(ctx, data, mi).await,
    );

    if let Some(Err(e)) = handled {
        warn!("Modal handler error for '{}': {:?}", mi.data.custom_id, e);
        // As with components, the handler may already have deferred — fall back to a
        // follow-up if a fresh response is rejected.
        let failed = mi
            .create_response(
                &ctx.http,
                serenity::CreateInteractionResponse::Message(
                    serenity::CreateInteractionResponseMessage::new()
                        .ephemeral(true)
                        .content("An error occurred processing your submission."),
                ),
            )
            .await
            .is_err();
        if failed {
            mi.create_followup(
                &ctx.http,
                serenity::CreateInteractionResponseFollowup::new()
                    .ephemeral(true)
                    .content("An error occurred processing your submission."),
            )
            .await
            .ok();
        }
    }
}
