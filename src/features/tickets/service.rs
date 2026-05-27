//! Non-UI ticket logic: opening a ticket thread, closing a ticket, and reading
//! intake-form responses out of a submitted modal.

use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::warn;

use crate::context::{cid_claim_btn, colours, BotData, CID_CLOSE_BTN};
use crate::db::{Ticket, TicketType};
use crate::ui::{self, Button, ButtonStyle, Container, Spacing};
use crate::util::{modal_field, sanitise_name};

pub struct OpenThreadOptions<'a> {
    pub ticket_type: &'a TicketType,
    pub ticket_number: i64,
    pub owner_id: serenity::UserId,
    pub parent_channel_id: serenity::ChannelId,
    pub form_responses: Option<serde_json::Map<String, serde_json::Value>>,
    pub reported_message_id: Option<serenity::MessageId>,
    pub reported_message_url: Option<String>,
    pub reported_message_content: Option<String>,
    pub reported_author_id: Option<serenity::UserId>,
}

pub struct OpenedThread {
    pub thread: serenity::GuildChannel,
}

pub async fn open_thread(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    guild_id: serenity::GuildId,
    opts: OpenThreadOptions<'_>,
) -> Result<OpenedThread> {
    let username = opts
        .owner_id
        .to_user(ctx)
        .await
        .map(|u| u.name)
        .unwrap_or_else(|_| "user".into());

    let thread_name = opts
        .ticket_type
        .thread_name_pattern
        .replace("{number}", &format!("{:04}", opts.ticket_number))
        .replace("{username}", &sanitise_name(&username))
        .replace("{type}", &sanitise_name(&opts.ticket_type.name));

    let thread = opts
        .parent_channel_id
        .create_thread(
            ctx,
            serenity::CreateThread::new(&thread_name)
                .kind(serenity::ChannelType::PrivateThread)
                .auto_archive_duration(serenity::AutoArchiveDuration::OneWeek)
                .invitable(false),
        )
        .await?;

    ctx.http
        .add_thread_channel_member(thread.id, opts.owner_id)
        .await?;

    let form_json = opts
        .form_responses
        .as_ref()
        .and_then(|m| serde_json::to_string(m).ok());

    let reported_msg_id = opts.reported_message_id.map(|id| id.to_string());
    let reported_author_id = opts.reported_author_id.map(|id| id.to_string());

    let ticket = data
        .db
        .create_ticket(
            opts.ticket_number,
            &guild_id.to_string(),
            Some(opts.ticket_type.id),
            &thread.id.to_string(),
            &opts.parent_channel_id.to_string(),
            &opts.owner_id.to_string(),
            form_json.as_deref(),
            reported_msg_id.as_deref(),
            opts.reported_message_url.as_deref(),
            opts.reported_message_content.as_deref(),
            reported_author_id.as_deref(),
        )
        .await?;

    // Build ping content (sent as plain message so it actually notifies)
    let ping_roles = opts.ticket_type.ping_role_ids();
    let mut ping_parts: Vec<String> = vec![format!("<@{}>", opts.owner_id)];
    ping_parts.extend(ping_roles.iter().map(|r| format!("<@&{}>", r)));
    let ping_content = ping_parts.join(" ");

    thread
        .id
        .send_message(ctx, serenity::CreateMessage::new().content(ping_content))
        .await
        .ok();

    // Build CV2 ticket card
    let color = colours::from_hex(&opts.ticket_type.color);
    let header = format!(
        "**{}{} — #{:04}**",
        opts.ticket_type
            .emoji
            .as_deref()
            .map(|e| format!("{} ", e))
            .unwrap_or_default(),
        opts.ticket_type.label,
        opts.ticket_number,
    );

    let default_welcome = format!(
        "Welcome, <@{}>!\nA member of staff will be with you shortly.",
        opts.owner_id
    );
    let mut welcome = opts
        .ticket_type
        .welcome_message
        .clone()
        .unwrap_or(default_welcome)
        .replace("{user}", &format!("<@{}>", opts.owner_id))
        .replace("{username}", &username)
        .replace("{type}", &opts.ticket_type.label);

    if let Some(ref fr) = form_json {
        if let Ok(map) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(fr) {
            welcome.push_str("\n\n**Your responses:**");
            for (k, v) in &map {
                let val = v.as_str().unwrap_or("");
                if !val.is_empty() {
                    let indented = val.replace('\n', "\n> ");
                    welcome.push_str(&format!("\n\n**{}:**\n> {}", k, indented));
                }
            }
        }
    }

    if opts.reported_message_id.is_some() {
        welcome.push_str(&format!(
            "\n\n**⚑ Reported message:**\n> {}",
            opts.reported_message_content
                .as_deref()
                .unwrap_or("*(no content)*"),
        ));
        if let Some(author) = &reported_author_id {
            welcome.push_str(&format!("\nAuthor: <@{}>", author));
        }
        if let Some(ref url) = opts.reported_message_url {
            welcome.push_str(&format!("\n[Jump to message]({})", url));
        }
    }

    let claim_btn = Button::new(cid_claim_btn(ticket.id), "Claim", ButtonStyle::Secondary).emoji("✋");
    let close_btn = Button::new(CID_CLOSE_BTN, "Close", ButtonStyle::Danger).emoji("🔒");

    let card = Container::new(vec![
        ui::text(header),
        ui::separator(false, Spacing::Small),
        ui::text(welcome),
        ui::separator(true, Spacing::Small),
        ui::action_row(vec![claim_btn.into(), close_btn.into()]),
    ])
    .accent(color.0);

    ui::send(&ctx.http, thread.id, &[card.into()]).await.ok();

    // Auto-add staff members
    let staff_roles = data
        .db
        .get_type_roles(opts.ticket_type.id)
        .await
        .unwrap_or_default();
    let staff_role_ids: HashSet<serenity::RoleId> = staff_roles
        .iter()
        .filter_map(|r| r.parse::<u64>().ok().map(serenity::RoleId::new))
        .collect();

    if opts.ticket_type.auto_add_staff && !staff_role_ids.is_empty() {
        let mut after: Option<serenity::UserId> = None;
        loop {
            let members = guild_id
                .members(ctx, Some(1000), after)
                .await
                .unwrap_or_default();
            if members.is_empty() {
                break;
            }
            for member in &members {
                if member.user.bot || member.user.id == opts.owner_id {
                    continue;
                }
                if member.roles.iter().any(|rid| staff_role_ids.contains(rid)) {
                    ctx.http
                        .add_thread_channel_member(thread.id, member.user.id)
                        .await
                        .ok();
                }
            }
            after = members.last().map(|m| m.user.id);
            if members.len() < 1000 {
                break;
            }
        }
    }

    // Per-type staff alert
    if let Some(ref ch) = opts.ticket_type.staff_alert_channel_id {
        if let Ok(cid) = ch.parse::<u64>() {
            let mentions = staff_roles
                .iter()
                .map(|r| format!("<@&{}>", r))
                .collect::<Vec<_>>()
                .join(" ");
            let alert = Container::new(vec![ui::text(format!(
                "**🎫 New {} Ticket**\nTicket **#{:04}** opened by <@{}>\nThread: <#{}>",
                opts.ticket_type.label, opts.ticket_number, opts.owner_id, thread.id
            ))])
            .accent(colours::BLURPLE.0);
            let ch = serenity::ChannelId::new(cid);
            if !mentions.is_empty() {
                ch.send_message(ctx, serenity::CreateMessage::new().content(&mentions))
                    .await
                    .ok();
            }
            ui::send(&ctx.http, ch, &[alert.into()]).await.ok();
        } else {
            warn!(
                "Invalid staff alert channel id on ticket type {}: {}",
                opts.ticket_type.id, ch
            );
        }
    }

    Ok(OpenedThread { thread })
}

pub async fn execute_close(
    http: &serenity::Http,
    data: &Arc<BotData>,
    ticket: &Ticket,
    closed_by: serenity::UserId,
    reason: Option<&str>,
) -> Result<()> {
    let ticket_type = if let Some(tid) = ticket.ticket_type_id {
        data.db.get_ticket_type_by_id(tid).await.ok().flatten()
    } else {
        None
    };

    data.db
        .close_ticket(ticket.id, &closed_by.to_string(), reason)
        .await?;

    let thread_id = ticket.thread_id.parse::<u64>().map(serenity::ChannelId::new)?;

    let close_card = Container::new(vec![ui::text(format!(
        "**🔒 Ticket Closed**\nClosed by <@{}>{}",
        closed_by,
        reason.map(|r| format!("\n**Reason:** {}", r)).unwrap_or_default(),
    ))])
    .accent(colours::RED.0);

    ui::send(http, thread_id, &[close_card.into()]).await.ok();

    thread_id
        .edit_thread(http, serenity::EditThread::new().archived(true).locked(true))
        .await
        .ok();

    // Post to ticket log channel with "View Thread" link
    let config = data.db.get_or_create_guild(&ticket.guild_id).await?;
    let log_ch_id = config.ticket_log_channel_id.or(config.log_channel_id);
    if let Some(ref log_id) = log_ch_id {
        if let Ok(cid) = log_id.parse::<u64>() {
            let type_label = ticket_type.as_ref().map(|t| t.label.as_str()).unwrap_or("General");
            let priority = crate::context::Priority::from_str(&ticket.priority);
            let tags = data.db.get_tags(ticket.id).await.unwrap_or_default();

            let thread_url = format!(
                "https://discord.com/channels/{}/{}/{}",
                ticket.guild_id, ticket.parent_channel_id, ticket.thread_id
            );

            let mut lines = vec![
                format!("**📋 Ticket #{:04} Closed**", ticket.ticket_number),
                format!(
                    "Category: {} · Priority: {} {}",
                    type_label,
                    priority.emoji(),
                    priority.label()
                ),
                format!("Owner: <@{}> · Closed by: <@{}>", ticket.owner_id, closed_by),
                format!("Reason: {}", reason.unwrap_or("No reason given")),
                format!("Opened: {}", ticket.created_at),
            ];
            if !tags.is_empty() {
                lines.push(format!("Tags: {}", tags.join(", ")));
            }
            lines.push(format!("[View Thread]({})", thread_url));

            let log_card = Container::new(vec![ui::text(lines.join("\n"))]).accent(colours::GREY.0);

            ui::send(http, serenity::ChannelId::new(cid), &[log_card.into()]).await.ok();
        }
    }

    Ok(())
}

/// Read intake-form responses out of a submitted modal, keyed by field label.
pub fn collect_form_responses(
    components: &[serenity::ActionRow],
    fields: &[crate::db::FormField],
) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();
    for field in fields {
        let cid = format!("ff_{}", field.id);
        if let Some(val) = modal_field(components, &cid) {
            map.insert(field.label.clone(), serde_json::Value::String(val.to_string()));
        }
    }
    map
}
