use poise::serenity_prelude as serenity;

use crate::context::{colours, Context, Error};
use crate::ids::cid_report_user_modal;

/// Report a user.
#[poise::command(slash_command, guild_only)]
pub async fn user(
    ctx: Context<'_>,
    #[description = "User to report"] target: serenity::User,
    #[description = "Reason (optional — a modal will open if omitted)"] reason: Option<String>,
) -> Result<(), Error> {
    if target.bot() {
        return Err(Error::user("You can't report a bot."));
    }
    if target.id == ctx.author().id {
        return Err(Error::user("You can't report yourself."));
    }

    if let Some(reason) = reason {
        return submit_user_report(ctx, target, reason).await;
    }

    crate::util::modal_response(
        ctx,
        super::super::view::build_report_modal(
            cid_report_user_modal(target.id.get()),
            format!("Report {}", target.name),
        ),
    )
    .await?;
    Ok(())
}

/// Create a report from an explicit reason and post the `#reports` card.
pub async fn submit_user_report(
    ctx: Context<'_>,
    target: serenity::User,
    reason: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let gid = guild_id.to_string();
    let reporter_id = ctx.author().id.to_string();
    let target_id = target.id.to_string();

    if let Some(block) = ctx.data().db.get_active_block(&gid, &reporter_id, "reports").await? {
        return Err(Error::user(crate::features::blocklist::view::blocked_text(&block)));
    }

    // A report is only actionable if there's a reports channel to surface its card.
    let guild_cfg = ctx.data().db.get_or_create_guild(&gid).await?;
    let Some(reports_ch) =
        guild_cfg.reports_channel_id.as_deref().and_then(|s| s.parse::<u64>().ok())
    else {
        return Err(Error::user(
            "Reporting isn't set up on this server yet. Ask an admin to configure a reports channel in `/setup tickets`.",
        ));
    };

    // Dedup: one open report per reporter+target at a time.
    if ctx.data().db.has_open_report(&gid, &reporter_id, &target_id).await? {
        return Err(Error::user(
            "You already have a pending report against this user — staff are reviewing it.",
        ));
    }

    let report = ctx
        .data()
        .db
        .create_report(
            &gid,
            &reporter_id,
            &target_id,
            None,
            None,
            None,
            if reason.is_empty() { None } else { Some(reason.as_str()) },
            None,
        )
        .await?;

    super::super::view::post_report_card(
        ctx.serenity_context(),
        &ctx.data(),
        serenity::ChannelId::new(reports_ch),
        &report,
        &target,
    )
    .await;

    ctx.send(
        poise::CreateReply::default().ephemeral(true).embed(
            serenity::CreateEmbed::new()
                .colour(colours::GREEN)
                .title("Report Submitted")
                .description(
                    "Your report has been received and will be reviewed by the moderation team.",
                ),
        ),
    )
    .await?;
    Ok(())
}
