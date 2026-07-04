use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::parse_panel_cfg;
use crate::ui;
use crate::util::respond_ephemeral;

use super::super::service::republish_panel;
use super::super::view::{
    build_panel_config_form, build_panel_cv2, build_panel_delete_confirm, build_panel_hub,
};

/// Render the panel configure form in place from fresh data. Shared by every
/// flow that lands on the form (hub select, config controls, basics modal,
/// panel creation).
pub(in crate::features::tickets) async fn show_config_form(
    http: &serenity::Http,
    data: &Arc<BotData>,
    interaction_id: serenity::InteractionId,
    token: &str,
    panel_id: i64,
    g: &str,
) -> Result<(), anyhow::Error> {
    if let Some(tree) = config_form_tree(data, panel_id, g).await? {
        ui::update(http, interaction_id, token, &tree).await?;
    }
    Ok(())
}

/// Build the panel configure form from fresh data (or `None` if the panel is
/// gone). Split out so a caller that has already deferred can push it via
/// `ui::update_original` instead of a type-7 response.
async fn config_form_tree(
    data: &Arc<BotData>,
    panel_id: i64,
    g: &str,
) -> Result<Option<Vec<ui::Component>>, anyhow::Error> {
    let Some(panel) = data.db.get_panel(panel_id).await? else {
        return Ok(None);
    };
    let all_cats = data.db.get_ticket_types(g).await?;
    let linked: Vec<i64> = data.db.get_panel_types(panel_id).await?.iter().map(|t| t.id).collect();
    let taken = data.db.get_type_ids_on_other_panels(g, panel_id).await?;
    Ok(Some(build_panel_config_form(&panel, &all_cats, &linked, &taken)))
}

/// If the panel is already posted, re-render that live message to match current
/// settings/categories.
async fn republish_if_live(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    panel_id: i64,
) -> Result<(), anyhow::Error> {
    if let Some(panel) = data.db.get_panel(panel_id).await? {
        republish_panel(&ctx.http, data, &panel).await;
    }
    Ok(())
}

/// Controls inside the panel configure form (`pnl:cfg:{id}:{field}`).
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
) -> Result<(), anyhow::Error> {
    let (panel_id, field) =
        parse_panel_cfg(&ci.data.custom_id).ok_or_else(|| anyhow::anyhow!("Invalid panel config ID"))?;

    let Some(guild_id) = ci.guild_id else {
        return Ok(());
    };
    if !crate::permissions::is_mod_staff(ctx, data, guild_id, ci.user.id).await {
        respond_ephemeral(ctx, ci, "Only staff can configure ticket panels.").await;
        return Ok(());
    }
    let g = guild_id.to_string();

    match field.as_str() {
        // Edit title / description / colour via modal.
        "basics" => {
            let panel = data.db.get_panel(panel_id).await?
                .ok_or_else(|| anyhow::anyhow!("Panel not found"))?;
            let modal = super::super::view::build_panel_basics_modal(
                crate::ids::cid_panel_basics_modal(panel_id),
                "✏️ Edit Panel Basics",
                Some(&panel),
            );
            ui::open_modal(&ctx.http, ci, &modal).await?;
        }

        // Change layout (buttons / dropdown).
        "layout" => {
            let layout = if let serenity::ComponentInteractionDataKind::StringSelect { values } = &ci.data.kind {
                if values.first().map(|s| s.as_str()) == Some("select") { "select" } else { "buttons" }
            } else {
                "buttons"
            };
            if let Some(panel) = data.db.get_panel(panel_id).await? {
                data.db.update_panel(panel_id, &panel.title, panel.description.as_deref(), &panel.color, layout).await?;
            }
            republish_if_live(ctx, data, panel_id).await?;
            show_config_form(&ctx.http, data, ci.id, &ci.token, panel_id, &g).await?;
        }

        // Choose which categories appear: replace the link set (the fix for the
        // old additive-only publish, where removed categories never unlinked).
        "cats" => {
            let selected: Vec<i64> = if let serenity::ComponentInteractionDataKind::StringSelect { values } = &ci.data.kind {
                values.iter().filter_map(|s| s.parse().ok()).collect()
            } else {
                vec![]
            };
            // A category can live on only one panel (context-menu opens resolve
            // category -> panel -> channel). Block rather than silently steal
            // the category from another live panel.
            let conflicts = data.db.find_panel_type_conflicts(panel_id, &selected).await?;
            if !conflicts.is_empty() {
                let lines: Vec<String> = conflicts
                    .iter()
                    .map(|(label, other)| {
                        format!(
                            "⚠️ **{label}** is already on the **{other}** panel. A category can only be on one panel — remove it there first.",
                        )
                    })
                    .collect();
                respond_ephemeral(ctx, ci, &lines.join("\n")).await;
                return Ok(());
            }
            data.db.set_panel_types(panel_id, &selected).await?;
            republish_if_live(ctx, data, panel_id).await?;
            show_config_form(&ctx.http, data, ci.id, &ci.token, panel_id, &g).await?;
        }

        // Publish (or re-publish) to the selected channel.
        "pub" => {
            let channel: Option<String> = if let serenity::ComponentInteractionDataKind::ChannelSelect { values } = &ci.data.kind {
                values.first().map(|c| c.to_string())
            } else {
                None
            };
            let Some(channel) = channel else {
                ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge).await?;
                return Ok(());
            };
            let types = data.db.get_panel_types(panel_id).await?;
            if types.is_empty() {
                respond_ephemeral(ctx, ci, "Add at least one category to this panel before publishing.").await;
                return Ok(());
            }
            let panel = data.db.get_panel(panel_id).await?
                .ok_or_else(|| anyhow::anyhow!("Panel not found"))?;
            let tree = build_panel_cv2(&panel, &types);
            let Ok(chan_id) = channel.parse::<u64>() else {
                respond_ephemeral(ctx, ci, "Couldn't read that channel.").await;
                return Ok(());
            };
            let chan = serenity::ChannelId::new(chan_id);

            // Non-blocking warnings, surfaced as follow-ups after publishing.
            let mut warnings: Vec<String> = Vec::new();
            // Buttons layout renders at most 5 categories (view::build_panel_cv2
            // takes 5); the rest would silently have no button.
            if panel.layout != "select" && types.len() > 5 {
                warnings.push(format!(
                    "⚠️ Buttons layout shows at most 5 categories, but this panel has {}. The extras won't appear — switch to the dropdown layout to show them all.",
                    types.len(),
                ));
            }

            // Best-effort pre-flight from the cache: right channel type and the
            // perms the bot needs to post the panel and open ticket threads
            // there. Cache misses fall through — the channel picker already
            // restricts to text channels, and thread creation reports a friendly
            // error at open time.
            let bot_id = ctx.cache.current_user().id;
            if let Ok(bot_member) = guild_id.member(ctx, bot_id).await {
                // Copy what we need out of the cache guard before any await.
                let checked = ctx.cache.guild(guild_id).and_then(|guild| {
                    guild
                        .channels
                        .get(&chan)
                        .map(|c| (c.base.kind, guild.user_permissions_in(c, &bot_member)))
                });
                if let Some((kind, perms)) = checked {
                    // Private ticket threads can't be created in announcement
                    // channels, so a text channel is the only valid target.
                    if !matches!(kind, serenity::ChannelType::Text) {
                        respond_ephemeral(ctx, ci, "Panels can only be published to standard text channels — private ticket threads can't be opened in announcement channels.").await;
                        return Ok(());
                    }
                    let required = [
                        (serenity::Permissions::VIEW_CHANNEL, "View Channel"),
                        (serenity::Permissions::SEND_MESSAGES, "Send Messages"),
                        (serenity::Permissions::CREATE_PRIVATE_THREADS, "Create Private Threads"),
                        (serenity::Permissions::SEND_MESSAGES_IN_THREADS, "Send Messages in Threads"),
                    ];
                    let missing: Vec<&str> = required
                        .iter()
                        .filter(|(p, _)| !perms.contains(*p))
                        .map(|(_, name)| *name)
                        .collect();
                    if !missing.is_empty() {
                        respond_ephemeral(
                            ctx,
                            ci,
                            &format!(
                                "I can't use <#{}> for tickets — I'm missing: **{}**. Grant them and publish again.",
                                chan,
                                missing.join(", "),
                            ),
                        )
                        .await;
                        return Ok(());
                    }
                    // Staff are pinged and pulled into a new private thread by
                    // mentioning their roles. A non-mentionable role only resolves
                    // if the bot has Mention Everyone, so without it that team
                    // gets no ping and — worse — isn't added to the thread at all.
                    let mut any_staff_roles = false;
                    for t in &types {
                        if !data.db.get_type_roles(t.id).await.unwrap_or_default().is_empty() {
                            any_staff_roles = true;
                            break;
                        }
                    }
                    if !perms.contains(serenity::Permissions::MENTION_EVERYONE) && any_staff_roles {
                        warnings.push(
                            "⚠️ I don't have **Mention @everyone, @here and All Roles** in that channel. Any staff role that isn't set to *Allow anyone to @mention this role* won't be pinged **or added to new ticket threads**, so that team won't be able to see those tickets. Either grant me that permission, or make the staff roles mentionable.".to_string(),
                        );
                    }
                }
            }

            // Posting the panel + tidying the old message is several HTTP calls;
            // ack as a deferred update first so we don't blow the 3s window, then
            // edit the config form once the work is done.
            ui::defer_update(&ctx.http, ci.id, &ci.token).await?;

            // Edit the existing message if it's in the same channel; otherwise post anew.
            let same_channel = panel.channel_id.as_deref() == Some(channel.as_str());
            let msg = match (same_channel, panel.message_id.as_ref().and_then(|m| m.parse::<u64>().ok())) {
                (true, Some(mid)) => {
                    match ui::edit(&ctx.http, chan, serenity::MessageId::new(mid), &tree).await {
                        Ok(m) => m,
                        Err(_) => ui::send(&ctx.http, chan, &tree).await?,
                    }
                }
                _ => ui::send(&ctx.http, chan, &tree).await?,
            };
            // Moving channels abandons the previous live message; delete it so a
            // stale panel with working buttons doesn't linger and diverge.
            if !same_channel {
                if let (Some(Ok(old_ch)), Some(Ok(old_mid))) = (
                    panel.channel_id.as_deref().map(str::parse::<u64>),
                    panel.message_id.as_deref().map(str::parse::<u64>),
                ) {
                    serenity::ChannelId::new(old_ch)
                        .widen()
                        .delete_message(&ctx.http, serenity::MessageId::new(old_mid), None)
                        .await
                        .ok();
                }
            }
            data.db.update_panel_message(panel_id, &channel, &msg.id.to_string()).await?;
            if let Some(tree) = config_form_tree(data, panel_id, &g).await? {
                ui::update_original(&ctx.http, &ci.token, &tree).await?;
            }
            for warning in warnings {
                ci.create_followup(
                    &ctx.http,
                    serenity::CreateInteractionResponseFollowup::new().ephemeral(true).content(warning),
                )
                .await
                .ok();
            }
        }

        // Delete flow.
        "delete" => {
            let panel = data.db.get_panel(panel_id).await?
                .ok_or_else(|| anyhow::anyhow!("Panel not found"))?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_panel_delete_confirm(&panel)).await?;
        }
        "delete_no" => {
            show_config_form(&ctx.http, data, ci.id, &ci.token, panel_id, &g).await?;
        }
        "delete_yes" => {
            data.db.delete_panel(panel_id).await?;
            let panels = data.db.get_panels(&g).await?;
            ui::update(&ctx.http, ci.id, &ci.token, &build_panel_hub(&panels)).await?;
        }

        _ => {
            ci.create_response(&ctx.http, serenity::CreateInteractionResponse::Acknowledge).await?;
        }
    }
    Ok(())
}
