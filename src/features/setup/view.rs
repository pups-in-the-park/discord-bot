//! Ephemeral CV2 settings forms for each `/setup` section. Pre-fills current
//! values so selections are visible when the form opens. Selects/toggles save
//! immediately (see `components`); numeric fields open a small modal.

use crate::context::{
    colours, CID_CAT_HUB_OPEN, CID_PANEL_HUB_OPEN, CID_SETUP_APPEALS, CID_SETUP_LOG_CHANNELS,
    CID_SETUP_MOD_STAFF, CID_SETUP_RAID, CID_SETUP_SLOWMODE, CID_SETUP_TICKET,
};
use crate::db::{GuildConfig, ModConfig, RaidConfig, SlowmodeConfig};
use crate::ui::{
    self, Button, ButtonStyle, ChannelSelect, ChannelType, Component, Container, RoleSelect,
    Section, SelectOption, Spacing, StringSelect,
};

/// Map a raid `join_threshold` to its human sensitivity label, derived from the
/// raid feature's own presets so the dashboard can't drift from what's stored.
fn raid_sensitivity_label(join_threshold: f64) -> &'static str {
    match crate::features::raid::sensitivity_key(join_threshold) {
        "low" => "Low",
        "high" => "High",
        _ => "Medium",
    }
}

/// The `/setup overview` dashboard: one status line per configuration area with a
/// quick-link button into it. ✅ = ready, ⚠️ = needs attention. The buttons reuse the
/// existing section ids (`section_button::handle` opens each form) plus the category
/// and panel hub openers.
pub fn build_setup_hub(
    guild: &GuildConfig,
    modc: &ModConfig,
    raid: &RaidConfig,
    slow: &SlowmodeConfig,
    num_categories: usize,
    num_published_panels: usize,
    num_panels: usize,
) -> Vec<Component> {
    // A status row: text on the left, a quick-link button on the right.
    let row = |body: String, btn_id: &str, btn_label: &str, style: ButtonStyle| -> Component {
        Section::new(vec![ui::text(body)], Button::new(btn_id, btn_label, style).into()).into()
    };

    let tick = |ok: bool| if ok { "✅" } else { "⚠️" };

    let cats_body = format!(
        "**🗂️ Categories** {} · {} defined",
        tick(num_categories > 0),
        num_categories
    );
    // Tickets open in the channel their panel is published to, so a published
    // panel is the make-or-break setting.
    let panels_ready = num_published_panels > 0;
    let panels_body = if panels_ready {
        format!(
            "**📋 Panels** ✅ · {} published of {}",
            num_published_panels, num_panels
        )
    } else {
        format!(
            "**📋 Panels** ⚠️ · {} published of {} — **tickets can't open until a panel is published**",
            num_published_panels, num_panels
        )
    };
    let panels_style = if panels_ready { ButtonStyle::Secondary } else { ButtonStyle::Primary };

    // Moderation & safety.
    let staff_n = modc.staff_roles().len();
    let staff_body = if staff_n > 0 {
        format!("**👮 Mod staff** ✅ · {} role(s) can moderate", staff_n)
    } else {
        "**👮 Mod staff** ⚠️ · Administrators only".to_string()
    };
    let appeals_ok = guild.appeals_channel_id.is_some();
    let appeals_body = format!(
        "**📨 Appeals & concerns** {} · appeals {} · concerns {}",
        tick(appeals_ok),
        if guild.appeals_channel_id.is_some() { "set" } else { "unset" },
        if guild.concerns_channel_id.is_some() { "set" } else { "unset" },
    );
    let reports_body = match &guild.reports_channel_id {
        Some(id) => format!("**🚨 Reports channel** ✅ · reports go to <#{}>", id),
        None => "**🚨 Reports channel** ⚠️ · not set".to_string(),
    };
    let raid_body = format!(
        "**🛡️ Anti-raid** ✅ · sensitivity **{}**",
        raid_sensitivity_label(raid.join_threshold)
    );
    let slow_body = format!(
        "**⏱️ Auto-slowmode** {} · {}",
        if slow.enabled { "✅" } else { "▫️" },
        if slow.enabled { "on" } else { "off" }
    );

    // Logging.
    let logs_set = [
        &guild.log_channel_id,
        &guild.mod_log_channel_id,
        &guild.chat_log_channel_id,
        &guild.ticket_log_channel_id,
    ]
    .iter()
    .filter(|c| c.is_some())
    .count();
    let logs_body = format!("**📋 Log channels** {} · {} of 4 set", tick(logs_set > 0), logs_set);

    vec![Container::new(vec![
        ui::text(
            "## 🛠️ Server Setup\n-# Configure pip for your server. ✅ ready · ⚠️ needs attention. Use the buttons to jump into each area.",
        ),
        ui::separator(true, Spacing::Small),
        ui::text("### 🎫 Ticket system\n-# Set these up in order. Tickets open in the channel each panel is published to."),
        row(cats_body, CID_CAT_HUB_OPEN, "Manage", ButtonStyle::Secondary),
        row(panels_body, CID_PANEL_HUB_OPEN, "Manage", panels_style),
        ui::separator(false, Spacing::Small),
        ui::text("### 🛡️ Moderation & safety"),
        row(staff_body, CID_SETUP_MOD_STAFF, "Configure", ButtonStyle::Secondary),
        row(reports_body, CID_SETUP_TICKET, "Configure", ButtonStyle::Secondary),
        row(appeals_body, CID_SETUP_APPEALS, "Configure", ButtonStyle::Secondary),
        row(raid_body, CID_SETUP_RAID, "Configure", ButtonStyle::Secondary),
        row(slow_body, CID_SETUP_SLOWMODE, "Configure", ButtonStyle::Secondary),
        ui::separator(false, Spacing::Small),
        ui::text("### 📋 Logging"),
        row(logs_body, CID_SETUP_LOG_CHANNELS, "Configure", ButtonStyle::Secondary),
    ])
    .accent(colours::BLURPLE.0)
    .into()]
}

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
        ui::text("## 📋 Log Channels\n-# Select where each log type is posted. Leave empty to disable that log."),
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
    vec![Container::new(vec![
        ui::text("## 🚨 Reports\n-# Configure where reports appear. Ticket threads open in the channel each panel is published to."),
        ui::separator(false, Spacing::Small),
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
        ui::text("## ⚖️ Moderation Staff\n-# Select which roles have mod access. Leave empty to restrict to Administrators only."),
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
        ui::text("## 📢 Appeals & Concerns\n-# Where banned/timed-out members appeal, and where staff review concerns."),
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
    let sensitivity = crate::features::raid::sensitivity_key(raid_cfg.join_threshold);
    let select = StringSelect::new(
        "setup:raid:sensitivity",
        vec![
            SelectOption::new("low", "Low").description("Only a large burst of joins triggers it").default(sensitivity == "low"),
            SelectOption::new("medium", "Medium").description("Balanced — recommended for most servers").default(sensitivity == "medium"),
            SelectOption::new("high", "High").description("Even a small spike of joins triggers it").default(sensitivity == "high"),
        ],
    )
    .placeholder("Select sensitivity…");
    vec![Container::new(vec![
        ui::text("## 🛡️ Anti-Raid Protection\n-# Automatic protection against coordinated join attacks."),
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
        ui::text("## ⏱️ Auto-Slowmode\n-# Automatically slows down channels when message volume spikes."),
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
