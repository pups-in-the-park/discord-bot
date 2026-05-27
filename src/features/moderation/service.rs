//! Non-UI moderation helpers shared across entry points. `send_action_dm` is the
//! one cross-feature export — slash commands, context menus, and the reports
//! "take action" flow all notify the target through it.

use poise::serenity_prelude as serenity;

use crate::context::colours;
use crate::ids::cid_appeal_btn;
use crate::ui::{self, Button, ButtonStyle, Container};

/// DM a user that a moderation action was taken against them, with an optional
/// "Appeal this action" button when the infraction is appealable.
pub async fn send_action_dm(
    http: &serenity::Http,
    user: &serenity::User,
    guild_id: serenity::GuildId,
    action_label: &str,
    reason: &str,
    appeal_info: Option<(i64, serenity::GuildId)>,
) {
    let dm_channel = match user.create_dm_channel(http).await {
        Ok(c) => c,
        Err(_) => return,
    };

    let mut components: Vec<ui::Component> = vec![ui::text(format!(
        "**You have received a moderation action**\n\
        Action: {}\nReason: {}\nServer: {}",
        action_label, reason, guild_id,
    ))];

    let mut btns: Vec<ui::Component> = vec![];
    if let Some((infraction_id, gid)) = appeal_info {
        btns.push(
            Button::new(
                cid_appeal_btn(infraction_id, gid.get()),
                "Appeal this action",
                ButtonStyle::Secondary,
            )
            .emoji("📝")
            .into(),
        );
    }

    if !btns.is_empty() {
        components.push(ui::action_row(btns));
    }

    let card = Container::new(components).accent(colours::ORANGE.0);
    ui::send(http, dm_channel.id, &[card.into()]).await.ok();
}
