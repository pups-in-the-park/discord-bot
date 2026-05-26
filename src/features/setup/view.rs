//! Ephemeral CV2 settings forms for each `/setup` section. Pre-fills current
//! values so selections are visible when the form opens. Selects/toggles save
//! immediately (see `components`); numeric fields open a small modal.

use crate::context::colours;
use crate::db::{GuildConfig, ModConfig, RaidConfig, SlowmodeConfig};
use crate::ui::{
    self, Button, ButtonStyle, ChannelSelect, ChannelType, Component, Container, RoleSelect,
    SelectOption, Spacing, StringSelect,
};

/// Channel types accepted by most setup channel pickers: text, announcement, private thread.
const TEXT_CHANNELS: &[ChannelType] =
    &[ChannelType::Text, ChannelType::Announcement, ChannelType::PrivateThread];

/// A channel select pre-filled with `current`, accepting [`TEXT_CHANNELS`].
fn channel_pick(custom_id: &str, current: Option<&str>) -> Component {
    ChannelSelect::new(custom_id, TEXT_CHANNELS)
        .placeholder("Select a channel…")
        .default(current)
        .into()
}

/// An "edit numeric value" button (opens a text modal).
fn edit_button(custom_id: &str, label: &str) -> Component {
    Button::new(custom_id, label, ButtonStyle::Secondary).emoji("✏️").into()
}

pub fn build_setup_log_form(cfg: &GuildConfig) -> Vec<Component> {
    vec![Container::new(vec![
        ui::text("**📋 Log Channels**\nSelect where each log type is posted. Leave empty to disable that log."),
        ui::separator(false, Spacing::Small),
        ui::text("**📍 Fallback Log** · All logs go here when no specific channel is set."),
        ui::action_row(vec![channel_pick("setup:log:fallback", cfg.log_channel_id.as_deref())]),
        ui::text("**⚖️ Moderation Log** · Warns, timeouts, kicks, bans, and unbans."),
        ui::action_row(vec![channel_pick("setup:log:mod", cfg.mod_log_channel_id.as_deref())]),
        ui::text("**💬 Chat Log** · Deleted and edited messages."),
        ui::action_row(vec![channel_pick("setup:log:chat", cfg.chat_log_channel_id.as_deref())]),
        ui::text("**🎫 Ticket Log** · Ticket opens, closes, and activity."),
        ui::action_row(vec![channel_pick("setup:log:ticket", cfg.ticket_log_channel_id.as_deref())]),
    ])
    .accent(colours::BLURPLE.0)
    .into()]
}

pub fn build_setup_ticket_form(cfg: &GuildConfig) -> Vec<Component> {
    let parent = ChannelSelect::new("setup:ticket:channel", &[ChannelType::Text, ChannelType::Announcement])
        .placeholder("Select a channel…")
        .default(cfg.ticket_channel_id.as_deref());
    vec![Container::new(vec![
        ui::text("**🎫 Ticket Settings**\nConfigure where ticket threads are created and where reports appear."),
        ui::separator(false, Spacing::Small),
        ui::text("**📂 Ticket Parent Channel** · New ticket threads are created inside this channel."),
        ui::action_row(vec![parent.into()]),
        ui::text("**🚨 Reports Channel** · Where reported messages and users appear for staff review."),
        ui::action_row(vec![channel_pick("setup:ticket:reports", cfg.reports_channel_id.as_deref())]),
    ])
    .accent(colours::BLURPLE.0)
    .into()]
}

pub fn build_setup_mod_form(cfg: &ModConfig) -> Vec<Component> {
    let staff_roles = cfg.staff_roles();
    let dm_btn = |id: &str, label: &str, on: bool| -> Component {
        Button::new(
            id,
            format!("{} {} DMs: {}", if on { "✅" } else { "❌" }, label, if on { "On" } else { "Off" }),
            if on { ButtonStyle::Success } else { ButtonStyle::Danger },
        )
        .into()
    };
    let staff_select = RoleSelect::new("setup:mod:staff")
        .placeholder("Select up to 10 staff roles…")
        .max_values(10)
        .defaults(staff_roles.iter().cloned());
    vec![Container::new(vec![
        ui::text("**⚖️ Moderation Staff**\nSelect which roles have mod access. Leave empty to restrict to Administrators only."),
        ui::separator(false, Spacing::Small),
        ui::text("**👮 Staff Roles**"),
        ui::action_row(vec![staff_select.into()]),
        ui::separator(false, Spacing::Small),
        ui::text("**💌 DM Notifications** · Toggle whether users receive a DM when a moderation action is taken."),
        ui::action_row(vec![
            dm_btn("setup:dm:warn", "Warn", cfg.dm_on_warn),
            dm_btn("setup:dm:timeout", "Timeout", cfg.dm_on_timeout),
            dm_btn("setup:dm:kick", "Kick", cfg.dm_on_kick),
            dm_btn("setup:dm:ban", "Ban", cfg.dm_on_ban),
        ]),
    ])
    .accent(colours::BLURPLE.0)
    .into()]
}

pub fn build_setup_appeals_form(guild_cfg: &GuildConfig, cooldown_days: i64) -> Vec<Component> {
    vec![Container::new(vec![
        ui::text("**📢 Appeals & Concerns**"),
        ui::separator(false, Spacing::Small),
        ui::text("**📨 Appeals Channel** · Where ban/timeout appeal threads are created."),
        ui::action_row(vec![channel_pick("setup:appeals:ch", guild_cfg.appeals_channel_id.as_deref())]),
        ui::text("**⚠️ Concerns Channel** · Where staff can review user concerns and feedback."),
        ui::action_row(vec![channel_pick("setup:concerns:ch", guild_cfg.concerns_channel_id.as_deref())]),
        ui::separator(false, Spacing::Small),
        ui::text(format!("**⏳ Waiting Period Before Appeal:** {} days after an action is taken before a user can appeal it", cooldown_days)),
        ui::action_row(vec![edit_button("setup:num:appeal_cooldown", "Change Waiting Period")]),
    ])
    .accent(colours::BLURPLE.0)
    .into()]
}

pub fn build_setup_raid_form(raid_cfg: &RaidConfig) -> Vec<Component> {
    let sensitivity = if raid_cfg.join_threshold >= 7.0 {
        "low"
    } else if raid_cfg.join_threshold <= 4.0 {
        "high"
    } else {
        "medium"
    };
    let select = StringSelect::new(
        "setup:raid:sensitivity",
        vec![
            SelectOption::new("low", "Low").description("Higher threshold — fewer false positives").default(sensitivity == "low"),
            SelectOption::new("medium", "Medium").description("Balanced detection (recommended default)").default(sensitivity == "medium"),
            SelectOption::new("high", "High").description("Lower threshold — triggers more easily").default(sensitivity == "high"),
        ],
    )
    .placeholder("Select sensitivity…");
    vec![Container::new(vec![
        ui::text("**🛡️ Anti-Raid Protection**\nAutomatic protection against coordinated join attacks."),
        ui::separator(false, Spacing::Small),
        ui::text("**🎚️ Detection Sensitivity**"),
        ui::action_row(vec![select.into()]),
        ui::separator(false, Spacing::Small),
        ui::text(format!("**⏱️ Slowmode Duration:** {} seconds applied during a detected raid", raid_cfg.slowmode_secs)),
        ui::action_row(vec![edit_button("setup:num:raid_slowmode", "Change Slowmode Duration")]),
    ])
    .accent(colours::BLURPLE.0)
    .into()]
}

pub fn build_setup_slowmode_form(cfg: &SlowmodeConfig) -> Vec<Component> {
    let on = cfg.enabled;
    let toggle = Button::new(
        "setup:slowmode:toggle",
        format!("{} Auto-Slowmode: {}", if on { "✅" } else { "❌" }, if on { "Enabled" } else { "Disabled" }),
        if on { ButtonStyle::Success } else { ButtonStyle::Danger },
    );
    vec![Container::new(vec![
        ui::text("**⏱️ Auto-Slowmode**\nAutomatically slows down channels when message volume spikes."),
        ui::separator(false, Spacing::Small),
        ui::action_row(vec![toggle.into()]),
        ui::separator(false, Spacing::Small),
        ui::text(format!(
            "**📊 Thresholds:** {} messages in {} seconds triggers slowmode",
            cfg.capacity, cfg.window_secs
        )),
        ui::action_row(vec![edit_button("setup:num:slowmode_config", "Change Thresholds")]),
    ])
    .accent(colours::BLURPLE.0)
    .into()]
}
