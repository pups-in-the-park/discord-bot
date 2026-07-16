//! Helpers that put CV2 component trees on the wire: plain messages, interaction
//! responses, and modal opens.

use anyhow::Result;
use poise::serenity_prelude as serenity;

use super::{Component, Modal, CV2_FLAG};

fn components_json(components: &[Component]) -> Result<serde_json::Value> {
    // Propagate rather than swallowing: a silently-empty component tree would wipe
    // the message (or be rejected by Discord) while the caller still sees `Ok`.
    Ok(serde_json::to_value(components)?)
}

/// Send a CV2 message to a channel. Returns the created message.
pub async fn send(
    http: &serenity::Http,
    channel_id: serenity::ChannelId,
    components: &[Component],
) -> Result<serenity::Message> {
    let body = serde_json::json!({ "flags": CV2_FLAG, "components": components_json(components)? });
    Ok(http.send_message(channel_id.widen(), vec![], &body).await?)
}

/// Forward an existing message into `channel_id` (Discord message forwarding —
/// `message_reference` type 1). Preserves the original's content, attachments, and
/// embeds as a snapshot, giving moderators full context. Errors (e.g. the message
/// was deleted) are the caller's to fall back from.
pub async fn forward_message(
    http: &serenity::Http,
    channel_id: serenity::ChannelId,
    source_channel: serenity::ChannelId,
    message_id: serenity::MessageId,
    guild_id: serenity::GuildId,
) -> Result<serenity::Message> {
    let body = serde_json::json!({
        "message_reference": {
            "type": 1,
            "message_id": message_id.to_string(),
            "channel_id": source_channel.to_string(),
            "guild_id": guild_id.to_string(),
        }
    });
    Ok(http.send_message(channel_id.widen(), vec![], &body).await?)
}

/// Edit an existing message to a new CV2 component tree.
pub async fn edit(
    http: &serenity::Http,
    channel_id: serenity::ChannelId,
    message_id: serenity::MessageId,
    components: &[Component],
) -> Result<serenity::Message> {
    let body = serde_json::json!({ "flags": CV2_FLAG, "components": components_json(components)? });
    Ok(http.edit_message(channel_id.widen(), message_id, &body, vec![]).await?)
}

/// Respond to a component interaction with a new ephemeral CV2 message (type 4).
pub async fn respond_ephemeral(
    http: &serenity::Http,
    ci: &serenity::ComponentInteraction,
    components: &[Component],
) -> Result<()> {
    let body = serde_json::json!({
        "type": 4,
        "data": { "flags": 64u64 | CV2_FLAG, "components": components_json(components)? },
    });
    http.create_interaction_response(ci.id, &ci.token, &body, vec![]).await?;
    Ok(())
}

/// Respond to any interaction (by id+token) with a new ephemeral CV2 message
/// (type 4). Use for modal submissions that have no source message to update — e.g.
/// a modal opened from a slash command.
pub async fn respond_ephemeral_to(
    http: &serenity::Http,
    interaction_id: serenity::InteractionId,
    token: &str,
    components: &[Component],
) -> Result<()> {
    let body = serde_json::json!({
        "type": 4,
        "data": { "flags": 64u64 | CV2_FLAG, "components": components_json(components)? },
    });
    http.create_interaction_response(interaction_id, token, &body, vec![]).await?;
    Ok(())
}

/// Update the message an interaction lives on in-place (type 7 UPDATE_MESSAGE).
/// Works for both component and modal interactions — pass `id`/`token` from either.
pub async fn update(
    http: &serenity::Http,
    interaction_id: serenity::InteractionId,
    token: &str,
    components: &[Component],
) -> Result<()> {
    let body = serde_json::json!({
        "type": 7,
        "data": { "flags": CV2_FLAG, "components": components_json(components)? },
    });
    http.create_interaction_response(interaction_id, token, &body, vec![]).await?;
    Ok(())
}

/// Acknowledge a component interaction as a deferred *update* (type 6). Buys the
/// 15-minute follow-up window to finish slow work before editing the source
/// message with [`update_original`]. Use when the work before the first response
/// might otherwise blow the 3-second ack deadline.
pub async fn defer_update(
    http: &serenity::Http,
    interaction_id: serenity::InteractionId,
    token: &str,
) -> Result<()> {
    let body = serde_json::json!({ "type": 6 });
    http.create_interaction_response(interaction_id, token, &body, vec![]).await?;
    Ok(())
}

/// Edit the original response of an already-acknowledged interaction (PATCH
/// `@original`) to a new CV2 component tree. Pairs with [`defer_update`], and
/// works for the ephemeral config-form messages (which a channel edit can't touch).
pub async fn update_original(
    http: &serenity::Http,
    token: &str,
    components: &[Component],
) -> Result<()> {
    let body = serde_json::json!({ "flags": CV2_FLAG, "components": components_json(components)? });
    http.edit_original_interaction_response(token, &body, vec![]).await?;
    Ok(())
}

/// Open a modal in response to a component interaction (type 9).
pub async fn open_modal(
    http: &serenity::Http,
    ci: &serenity::ComponentInteraction,
    modal: &Modal,
) -> Result<()> {
    let body = serde_json::json!({ "type": 9, "data": modal });
    http.create_interaction_response(ci.id, &ci.token, &body, vec![]).await?;
    Ok(())
}

/// Respond to a poise slash/context-menu command with an ephemeral CV2 message.
pub async fn slash_respond(
    ctx: crate::context::Context<'_>,
    components: &[Component],
) -> Result<()> {
    if let poise::Context::Application(app) = ctx {
        let body = serde_json::json!({
            "type": 4,
            "data": { "flags": 64u64 | CV2_FLAG, "components": components_json(components)? },
        });
        ctx.serenity_context()
            .http
            .create_interaction_response(app.interaction.id, &app.interaction.token, &body, vec![])
            .await?;
    }
    Ok(())
}
