//! Non-UI ticket logic: opening a ticket thread, closing a ticket, and reading
//! intake-form responses out of a submitted modal.

use std::sync::Arc;

use anyhow::Result;
use poise::serenity_prelude as serenity;
use tracing::warn;

use crate::context::{colours, BotData};
use crate::db::{Ticket, TicketType};
use crate::ui;
use crate::util::{modal_field, sanitise_name};

use super::view;

pub struct OpenThreadOptions<'a> {
    pub ticket_type: &'a TicketType,
    pub ticket_number: i64,
    pub owner_id: serenity::UserId,
    /// The owner on panel flows, the staff member on context-menu flows.
    /// Staff-opened tickets are staff-close-only.
    pub opened_by: serenity::UserId,
    pub parent_channel_id: serenity::ChannelId,
    pub form_responses: Option<serde_json::Map<String, serde_json::Value>>,
    pub reported_message_id: Option<serenity::MessageId>,
    pub reported_message_url: Option<String>,
    pub reported_message_content: Option<String>,
    pub reported_author_id: Option<serenity::UserId>,
}

pub struct OpenedThread {
    pub thread: serenity::GuildThread,
}

pub async fn open_thread(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    guild_id: serenity::GuildId,
    opts: OpenThreadOptions<'_>,
) -> Result<OpenedThread> {
    // A stale panel/select/modal can reference a category deleted after it was
    // rendered; enforce the invariant here so no entry point can miss it.
    anyhow::ensure!(opts.ticket_type.active, "Ticket category is no longer available");

    let username = opts
        .owner_id
        .to_user(ctx)
        .await
        .map(|u| u.name.to_string())
        .unwrap_or_else(|_| "user".to_string());

    let thread_name = opts
        .ticket_type
        .thread_name_pattern
        .replace("{number}", &format!("{:04}", opts.ticket_number))
        .replace("{username}", &sanitise_name(&username))
        .replace("{type}", &sanitise_name(&opts.ticket_type.name));

    let thread = match opts
        .parent_channel_id
        .create_thread(
            &ctx.http,
            serenity::CreateThread::new(thread_name)
                .kind(serenity::ChannelType::PrivateThread)
                .auto_archive_duration(serenity::AutoArchiveDuration::OneWeek)
                .invitable(false),
        )
        .await
    {
        Ok(t) => t,
        Err(e) => {
            anyhow::bail!(
                "I couldn't create a private thread in <#{}>. Make sure it's a standard \
                 text channel (private threads aren't allowed in announcement channels) \
                 and that I have **Create Private Threads** and **Send Messages in \
                 Threads** there. ({e})",
                opts.parent_channel_id,
            );
        }
    };

    // From here on the thread exists on Discord but isn't recorded yet. If a required
    // step fails, delete the thread so we don't strand an orphan with no DB row (which
    // would also leak the ticket number and confuse "max open" counting).
    if let Err(e) = ctx
        .http
        .add_thread_channel_member(thread.id, opts.owner_id)
        .await
    {
        thread.id.widen().delete(&ctx.http, None).await.ok();
        return Err(e.into());
    }

    let form_json = opts
        .form_responses
        .as_ref()
        .and_then(|m| serde_json::to_string(m).ok());

    let reported_msg_id = opts.reported_message_id.map(|id| id.to_string());
    let reported_author_id = opts.reported_author_id.map(|id| id.to_string());

    let ticket = match data
        .db
        .create_ticket(
            opts.ticket_number,
            &guild_id.to_string(),
            Some(opts.ticket_type.id),
            &thread.id.to_string(),
            &opts.parent_channel_id.to_string(),
            &opts.owner_id.to_string(),
            &opts.opened_by.to_string(),
            form_json.as_deref(),
            reported_msg_id.as_deref(),
            opts.reported_message_url.as_deref(),
            opts.reported_message_content.as_deref(),
            reported_author_id.as_deref(),
        )
        .await
    {
        Ok(t) => t,
        Err(e) => {
            thread.id.widen().delete(&ctx.http, None).await.ok();
            return Err(e);
        }
    };

    let staff_roles = data
        .db
        .get_type_roles(opts.ticket_type.id)
        .await
        .unwrap_or_default();

    // In-thread ping: the owner, plus the staff roles when Auto-Add is on —
    // mentioning a role in a private thread pulls its members in, so one
    // mention both notifies and adds. (Non-mentionable roles need the bot to
    // have Mention Everyone; warned about at panel publish time.)
    let mut ping_parts: Vec<String> = vec![format!("<@{}>", opts.owner_id)];
    if opts.ticket_type.auto_add_staff {
        ping_parts.extend(staff_roles.iter().map(|r| format!("<@&{}>", r)));
    }
    let ping_content = ping_parts.join(" ");

    thread
        .id
        .widen()
        .send_message(
            &ctx.http,
            serenity::CreateMessage::new().content(ping_content).allowed_mentions(
                serenity::CreateAllowedMentions::new().all_users(true).all_roles(true),
            ),
        )
        .await
        .ok();

    // Post the CV2 ticket card and remember its message id so claim/priority
    // changes can re-render it.
    let card = view::build_ticket_card(opts.ticket_type, &ticket, &username);
    match ui::send(&ctx.http, serenity::ChannelId::new(thread.id.get()), &card).await {
        Ok(msg) => {
            data.db
                .set_ticket_card_message(ticket.id, &msg.id.to_string())
                .await
                .ok();
        }
        Err(e) => warn!("Couldn't post the ticket card in thread {}: {e}", thread.id),
    }

    // Staff alert card. Only ping alongside it when auto-add is off and notify
    // is on — auto-add already pinged in-thread, and notify-off means the
    // admin asked for a quiet channel.
    if let Some(ref ch) = opts.ticket_type.staff_alert_channel_id {
        if let Ok(cid) = ch.parse::<u64>() {
            let alert_ch = serenity::ChannelId::new(cid);
            let ping_alert = !opts.ticket_type.auto_add_staff && opts.ticket_type.notify_staff;
            if ping_alert && !staff_roles.is_empty() {
                let mentions = staff_roles
                    .iter()
                    .map(|r| format!("<@&{}>", r))
                    .collect::<Vec<_>>()
                    .join(" ");
                alert_ch
                    .widen()
                    .send_message(
                        &ctx.http,
                        serenity::CreateMessage::new().content(mentions).allowed_mentions(
                            serenity::CreateAllowedMentions::new().all_roles(true),
                        ),
                    )
                    .await
                    .ok();
            }
            let alert = view::build_staff_alert_card(opts.ticket_type, &ticket);
            ui::send(&ctx.http, alert_ch, &alert).await.ok();
        } else {
            warn!(
                "Invalid staff alert channel id on ticket type {}: {}",
                opts.ticket_type.id, ch
            );
        }
    }

    Ok(OpenedThread { thread })
}

/// Reason a member may not open a ticket of this type, or `None`. Checked at
/// the panel button *and* again at form submit — a rapid re-click (or a block
/// added meanwhile) could otherwise slip past the limit/blocklist.
pub async fn ticket_open_block_reason(
    data: &Arc<BotData>,
    guild_id: serenity::GuildId,
    user_id: serenity::UserId,
    ticket_type: &TicketType,
) -> Result<Option<String>> {
    // A stale panel/select/modal can reference a deactivated category.
    if !ticket_type.active {
        return Ok(Some("This ticket category is no longer available.".to_string()));
    }
    // Blocklist (category-specific, also covered by the `tickets` umbrella).
    if let Some(block) = data
        .db
        .get_active_block(
            &guild_id.to_string(),
            &user_id.to_string(),
            &format!("ticket:{}", ticket_type.name),
        )
        .await?
    {
        return Ok(Some(crate::features::blocklist::view::blocked_text(&block)));
    }
    // Max open per user (0 = unlimited).
    let open_count = data
        .db
        .count_open_tickets_for_user(&guild_id.to_string(), &user_id.to_string(), ticket_type.id)
        .await?;
    if ticket_type.max_open_per_user > 0 && open_count >= ticket_type.max_open_per_user {
        return Ok(Some(format!(
            "You already have {} open ticket(s) of this type. Please wait for them to be resolved.",
            open_count,
        )));
    }
    Ok(None)
}

/// Re-render a ticket's in-thread card from its current DB state (after a claim,
/// unclaim, or priority change). A missing card id (pre-migration tickets) or a
/// deleted card message is not an error — the card just isn't refreshed.
pub async fn refresh_ticket_card(
    http: &serenity::Http,
    data: &Arc<BotData>,
    ticket: &Ticket,
) -> Result<()> {
    let Some(ref card_id) = ticket.card_message_id else {
        return Ok(());
    };
    let Some(type_id) = ticket.ticket_type_id else {
        return Ok(());
    };
    let Some(cat) = data.db.get_ticket_type_by_id(type_id).await? else {
        return Ok(());
    };
    let Ok(owner_id) = ticket.owner_id.parse::<u64>() else {
        return Ok(());
    };
    let username = serenity::UserId::new(owner_id)
        .to_user(http)
        .await
        .map(|u| u.name.to_string())
        .unwrap_or_else(|_| "user".to_string());

    let card = view::build_ticket_card(&cat, ticket, &username);
    let (Ok(channel_id), Ok(message_id)) = (ticket.thread_id.parse::<u64>(), card_id.parse::<u64>())
    else {
        return Ok(());
    };
    ui::edit(
        http,
        serenity::ChannelId::new(channel_id),
        serenity::MessageId::new(message_id),
        &card,
    )
    .await
    .ok();
    Ok(())
}

pub async fn execute_close(
    http: &serenity::Http,
    data: &Arc<BotData>,
    ticket: &Ticket,
    closed_by: serenity::UserId,
    reason: Option<&str>,
) -> Result<()> {
    // Every close path routes through here, so a stale Close button re-closing
    // an already-closed ticket is a harmless no-op.
    if ticket.status == crate::db::TicketStatus::Closed {
        return Ok(());
    }

    let ticket_type = if let Some(tid) = ticket.ticket_type_id {
        data.db.get_ticket_type_by_id(tid).await.ok().flatten()
    } else {
        None
    };

    data.db
        .close_ticket(ticket.id, &closed_by.to_string(), reason)
        .await?;

    let thread_id = ticket.thread_id.parse::<u64>().map(serenity::ChannelId::new)?;

    // Read-only "closed" notice → embed.
    let close_embed = serenity::CreateEmbed::new()
        .colour(colours::RED)
        .title("🔒 Ticket Closed")
        .description(format!(
            "Closed by <@{}>{}",
            closed_by,
            reason.map(|r| format!("\n**Reason:** {}", r)).unwrap_or_default(),
        ));
    thread_id
        .widen()
        .send_message(http, serenity::CreateMessage::new().embed(close_embed))
        .await
        .ok();

    serenity::ThreadId::new(thread_id.get())
        .edit(http, serenity::EditThread::new().archived(true).locked(true))
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

            // A thread link is 2-segment (`channels/{guild}/{thread}`). The
            // 3-segment form is a *message* jump-link, which a private thread has
            // no starter message for, so it would resolve nowhere.
            let thread_url = format!(
                "https://discord.com/channels/{}/{}",
                ticket.guild_id, ticket.thread_id
            );

            // Ticket log entry → embed (logs are the canonical read-only surface).
            // Scalars go in fields; the "View Thread" markdown link is the body.
            let mut log_embed = serenity::CreateEmbed::new()
                .colour(colours::GREY)
                .title(format!("📋 Ticket #{:04} Closed", ticket.ticket_number))
                .field("Category", type_label, true)
                .field("Priority", format!("{} {}", priority.emoji(), priority.label()), true)
                .field("Owner", format!("<@{}>", ticket.owner_id), true)
                .field("Closed by", format!("<@{}>", closed_by), true)
                .field("Reason", reason.unwrap_or("No reason given"), false)
                .field("Opened", ticket.created_at.to_string(), true)
                .description(format!("[View Thread]({})", thread_url));
            if !tags.is_empty() {
                log_embed = log_embed.field("Tags", tags.join(", "), true);
            }

            serenity::ChannelId::new(cid)
                .widen()
                .send_message(http, serenity::CreateMessage::new().embed(log_embed))
                .await
                .ok();
        }
    }

    Ok(())
}

/// Re-render one panel's live message from current data. No-op when the panel
/// isn't published; the edit itself is best-effort. This is the single
/// implementation of the live-refresh sequence — every path that changes what a
/// published panel should show goes through here.
pub async fn republish_panel(http: &serenity::Http, data: &Arc<BotData>, panel: &crate::db::Panel) {
    let (Some(mid), Some(cid)) = (panel.message_id.as_ref(), panel.channel_id.as_ref()) else {
        return;
    };
    let (Ok(c), Ok(m)) = (cid.parse::<u64>(), mid.parse::<u64>()) else {
        return;
    };
    let Ok(types) = data.db.get_panel_types(panel.id).await else {
        return;
    };
    let tree = super::view::build_panel_cv2(panel, &types);
    ui::edit(http, serenity::ChannelId::new(c), serenity::MessageId::new(m), &tree)
        .await
        .ok();
}

/// Refresh the live message of the panel that carries the given category, if
/// any. Called after a category is deleted so its button/option disappears.
pub async fn republish_panels_containing(http: &serenity::Http, data: &Arc<BotData>, type_id: i64) {
    if let Ok(Some(panel)) = data.db.get_panel_for_type(type_id).await {
        republish_panel(http, data, &panel).await;
    }
}

/// Where a ticket of this category opens when there is no interaction channel
/// to inherit (context-menu / staff-on-behalf paths): the category's panel's
/// published channel. Errors carry a user-facing message when the category has
/// no panel or the panel isn't published.
pub async fn resolve_parent_for_type(
    data: &Arc<BotData>,
    ticket_type: &TicketType,
) -> Result<serenity::ChannelId> {
    // Per-category override wins — lets a staff-only category exist off-panel.
    if let Some(c) = ticket_type.ticket_channel_id.as_ref().and_then(|s| s.parse::<u64>().ok()) {
        return Ok(serenity::ChannelId::new(c));
    }
    let panel = data.db.get_panel_for_type(ticket_type.id).await?;
    let Some(panel) = panel else {
        anyhow::bail!(
            "The **{}** category isn't on any ticket panel and has no Ticket Channel set. \
             Either set a Ticket Channel on its Behaviour tab, or add it to a panel and \
             publish the panel.",
            ticket_type.label,
        );
    };
    let channel = panel
        .channel_id
        .as_ref()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|_| panel.message_id.is_some());
    match channel {
        Some(c) => Ok(serenity::ChannelId::new(c)),
        None => anyhow::bail!(
            "The **{}** category is on the **{}** panel, but that panel hasn't been \
             published to a channel yet. Publish it first.",
            ticket_type.label,
            panel.title,
        ),
    }
}

/// Allocate the next ticket number and open a ticket with no intake responses.
/// Shared by every entry point whose category has no (renderable) form.
pub async fn open_ticket_no_form(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    guild_id: serenity::GuildId,
    ticket_type: &TicketType,
    owner_id: serenity::UserId,
    opened_by: serenity::UserId,
    parent_channel_id: serenity::ChannelId,
) -> Result<OpenedThread> {
    let ticket_number = data.db.next_ticket_number(&guild_id.to_string()).await?;
    open_thread(
        ctx,
        data,
        guild_id,
        OpenThreadOptions {
            ticket_type,
            ticket_number,
            owner_id,
            opened_by,
            parent_channel_id,
            form_responses: None,
            reported_message_id: None,
            reported_message_url: None,
            reported_message_content: None,
            reported_author_id: None,
        },
    )
    .await
}

/// Close policy shared by `/ticket close`, the card's Close button, and the
/// close modal: staff always may; the owner only if they opened the ticket
/// themselves (staff-opened tickets are staff-close-only).
pub async fn user_may_close(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    guild_id: Option<serenity::GuildId>,
    ticket: &crate::db::Ticket,
    user_id: serenity::UserId,
) -> bool {
    if ticket.owner_id == user_id.to_string() && !ticket.staff_opened() {
        return true;
    }
    // The opener may always close. The context menus are gated by Discord
    // perms, which may not match the configured staff-role list — without this
    // a ticket could end up closeable by no one.
    if ticket.opened_by.as_deref() == Some(user_id.to_string().as_str()) {
        return true;
    }
    match guild_id {
        Some(g) => crate::permissions::is_mod_staff(ctx, data, g, user_id).await,
        None => false,
    }
}

/// The matching refusal copy for [`user_may_close`].
pub fn close_denied_message(ticket: &crate::db::Ticket) -> &'static str {
    if ticket.staff_opened() {
        "This ticket was opened by staff — only staff can close it."
    } else {
        "Only staff or the ticket owner can close this ticket."
    }
}

/// Read intake-form responses out of a submitted modal, keyed by field label.
/// Handles every question type: text inputs, dropdowns, and checkbox groups.
pub fn collect_form_responses(
    components: &[serenity::ModalComponent],
    fields: &[crate::db::FormField],
) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();
    for field in fields {
        let cid = format!("ff_{}", field.id);
        let value = match field.style.as_str() {
            "select" => {
                let picked = crate::ui::read_multi_select(components, &cid);
                (!picked.is_empty()).then(|| picked.join(", "))
            }
            "checkbox" => {
                let picked = crate::ui::read_checkbox_group(components, &cid);
                (!picked.is_empty()).then(|| picked.join(", "))
            }
            _ => modal_field(components, &cid).map(str::to_string),
        };
        if let Some(val) = value {
            if !val.is_empty() {
                map.insert(field.label.clone(), serde_json::Value::String(val));
            }
        }
    }
    map
}
