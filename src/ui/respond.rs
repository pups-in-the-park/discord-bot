//! Helpers that put CV2 component trees on the wire: plain messages, interaction
//! responses, and modal opens. These replace the scattered raw-JSON bodies that
//! used to live in `util.rs` (`send_cv2`, `ci_respond_cv2`, `ci_update_cv2`,
//! `mi_update_cv2`, `slash_respond_cv2`, `ci_open_modal`).

use anyhow::Result;
use poise::serenity_prelude as serenity;

use super::{Component, Modal, CV2_FLAG};

fn components_json(components: &[Component]) -> serde_json::Value {
    serde_json::to_value(components).unwrap_or_else(|_| serde_json::json!([]))
}

/// Send a CV2 message to a channel. Returns the created message.
pub async fn send(
    http: &serenity::Http,
    channel_id: serenity::ChannelId,
    components: &[Component],
) -> Result<serenity::Message> {
    let body = serde_json::json!({ "flags": CV2_FLAG, "components": components_json(components) });
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
    let body = serde_json::json!({ "flags": CV2_FLAG, "components": components_json(components) });
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
        "data": { "flags": 64u64 | CV2_FLAG, "components": components_json(components) },
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
        "data": { "flags": 64u64 | CV2_FLAG, "components": components_json(components) },
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
        "data": { "flags": CV2_FLAG, "components": components_json(components) },
    });
    http.create_interaction_response(interaction_id, token, &body, vec![]).await?;
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
            "data": { "flags": 64u64 | CV2_FLAG, "components": components_json(components) },
        });
        ctx.serenity_context()
            .http
            .create_interaction_response(app.interaction.id, &app.interaction.token, &body, vec![])
            .await?;
    }
    Ok(())
}
