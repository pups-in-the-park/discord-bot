use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::{parse_report_msg_modal, parse_report_user_modal};
use crate::util::modal_field;

/// A report-reason modal was submitted, from either the user or message entry
/// points (`m:rusr:` / `m:rep:`). Creates the report and posts the `#reports` card.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    mi: &serenity::ModalInteraction,
) -> Result<(), anyhow::Error> {
    let Some(guild_id) = mi.guild_id else {
        return Ok(());
    };
    let reason = modal_field(&mi.data.components, crate::ids::REPORT_REASON_FIELD)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    // Profile-part checkboxes (present only on the user/profile report modal).
    let parts = crate::ui::read_checkbox_group(&mi.data.components, crate::ids::REPORT_PARTS_FIELD);
    let profile_parts = (!parts.is_empty()).then(|| parts.join(","));
    let id = mi.data.custom_id.as_str();

    let (target_id, msg_id, msg_url, msg_content) =
        if let Some(target) = parse_report_user_modal(id) {
            (target.to_string(), None::<String>, None::<String>, None::<String>)
        } else if let Some((msg_id_val, chan_id, author_id)) = parse_report_msg_modal(id) {
            let msg_url =
                format!("https://discord.com/channels/{}/{}/{}", guild_id, chan_id, msg_id_val);
            // Best-effort: fetch the reported message so the card can show its content.
            let content = ctx
                .http
                .get_message(serenity::ChannelId::new(chan_id).widen(), serenity::MessageId::new(msg_id_val))
                .await
                .ok()
                .map(|m| m.content.to_string())
                .filter(|c| !c.is_empty());
            (author_id.to_string(), Some(msg_id_val.to_string()), Some(msg_url), content)
        } else {
            return Ok(());
        };

    let gid = guild_id.to_string();
    let reporter_id = mi.user.id.to_string();

    if let Some(block) = data.db.get_active_block(&gid, &reporter_id, "reports").await? {
        ephemeral_reply(ctx, mi, &crate::features::blocklist::view::blocked_text(&block)).await?;
        return Ok(());
    }

    // A report is only actionable if there's a reports channel to surface its card.
    let guild_cfg = data.db.get_or_create_guild(&gid).await?;
    let Some(reports_ch) =
        guild_cfg.reports_channel_id.as_deref().and_then(|s| s.parse::<u64>().ok())
    else {
        ephemeral_reply(
            ctx,
            mi,
            "Reporting isn't set up on this server yet. Ask an admin to configure a reports channel in `/setup tickets`.",
        )
        .await?;
        return Ok(());
    };

    // Dedup: one open report per reporter+target at a time.
    if data.db.has_open_report(&gid, &reporter_id, &target_id).await? {
        ephemeral_reply(ctx, mi, "You already have a pending report against this user — staff are reviewing it.").await?;
        return Ok(());
    }

    let report = data
        .db
        .create_report(
            &gid,
            &reporter_id,
            &target_id,
            msg_id.as_deref(),
            msg_url.as_deref(),
            msg_content.as_deref(),
            reason.as_deref(),
            profile_parts.as_deref(),
        )
        .await?;

    let target_user = serenity::UserId::new(target_id.parse::<u64>().unwrap_or(0))
        .to_user(ctx)
        .await
        .unwrap_or_else(|_| serenity::User::default());
    super::super::view::post_report_card(
        ctx,
        data,
        serenity::ChannelId::new(reports_ch),
        &report,
        &target_user,
    )
    .await;

    ephemeral_reply(ctx, mi, "Your report has been received and will be reviewed by the moderation team.").await?;
    Ok(())
}

/// Reply to a modal submission with a short ephemeral message.
async fn ephemeral_reply(
    ctx: &serenity::Context,
    mi: &serenity::ModalInteraction,
    msg: &str,
) -> Result<(), anyhow::Error> {
    mi.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content(msg),
        ),
    )
    .await?;
    Ok(())
}
