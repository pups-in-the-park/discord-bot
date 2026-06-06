use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::{colours, BotData};
use crate::ids::cid_report_action_chosen;
use crate::ui::{self, SelectOption, StringSelect};
use crate::util::respond_ephemeral;

/// "Take Action" button: present the ephemeral action-picker select.
pub async fn handle(
    ctx: &serenity::Context,
    data: &Arc<BotData>,
    ci: &serenity::ComponentInteraction,
    report_id: i64,
) -> Result<(), anyhow::Error> {
    let Some(guild_id) = ci.guild_id else {
        return Ok(());
    };
    if !crate::permissions::is_mod_staff(ctx, data, guild_id, ci.user.id).await {
        respond_ephemeral(ctx, ci, "Only staff can manage reports.").await;
        return Ok(());
    }

    let report = data
        .db
        .get_report_by_id(report_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Report not found"))?;

    if report.status != "open" {
        respond_ephemeral(ctx, ci, "This report has already been resolved.").await;
        return Ok(());
    }
    if super::super::acting_on_own_report(ci.user.id, &report) {
        respond_ephemeral(ctx, ci, super::super::SELF_ACTION_REFUSAL).await;
        return Ok(());
    }

    // Message-specific actions only apply when the report references a message.
    // The action picker is inline (not a modal select) on purpose: the chosen
    // action drives which modal fields appear next — only an inline select can do
    // that. Built as a CV2 container via the ui:: kit, per the UI conventions.
    let mut options = Vec::new();
    if report.message_url.is_some() {
        options.push(SelectOption::new("del", "Delete message"));
        options.push(SelectOption::new("dw", "Delete & Warn"));
    }
    options.push(SelectOption::new("warn", "Warn"));
    options.push(SelectOption::new("timeout", "Timeout"));
    options.push(SelectOption::new("kick", "Kick"));
    options.push(SelectOption::new("ban", "Ban"));

    let select = StringSelect::new(cid_report_action_chosen(report_id), options)
        .placeholder("Choose action…");
    let card = ui::Container::new(vec![
        ui::text("**Select the action to take against this user:**"),
        ui::action_row(vec![select.into()]),
    ])
    .accent(colours::BLURPLE.0);
    ui::respond_ephemeral(&ctx.http, ci, &[card.into()]).await?;
    Ok(())
}
