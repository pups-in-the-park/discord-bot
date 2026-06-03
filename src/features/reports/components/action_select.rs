use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::context::BotData;
use crate::ids::cid_report_action_chosen;
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
    let mut options = Vec::new();
    if report.message_url.is_some() {
        options.push(serenity::CreateSelectMenuOption::new("Delete message", "del"));
        options.push(serenity::CreateSelectMenuOption::new("Delete & Warn", "dw"));
    }
    options.push(serenity::CreateSelectMenuOption::new("Warn", "warn"));
    options.push(serenity::CreateSelectMenuOption::new("Timeout", "timeout"));
    options.push(serenity::CreateSelectMenuOption::new("Kick", "kick"));
    options.push(serenity::CreateSelectMenuOption::new("Ban", "ban"));

    ci.create_response(
        &ctx.http,
        serenity::CreateInteractionResponse::Message(
            serenity::CreateInteractionResponseMessage::new()
                .ephemeral(true)
                .content("Select the action to take against this user:")
                .components(vec![serenity::CreateComponent::ActionRow(
                    serenity::CreateActionRow::SelectMenu(
                        serenity::CreateSelectMenu::new(
                            cid_report_action_chosen(report_id),
                            serenity::CreateSelectMenuKind::String {
                                options: options.into(),
                            },
                        )
                        .placeholder("Choose action…"),
                    ),
                )]),
        ),
    )
    .await?;
    Ok(())
}
