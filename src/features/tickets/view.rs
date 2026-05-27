//! Ticket message/CV2 builders: the published panel, the category configuration
//! form, and the dynamic intake-form modal.

use poise::serenity_prelude as serenity;

use crate::context::colours;
use crate::db::TicketType;
use crate::ui::{
    self, Button, ButtonStyle, ChannelSelect, ChannelType, Component, Container, RoleSelect,
    SelectOption, Spacing, StringSelect,
};

/// Channel types offered by the staff-alert channel picker: text, announcement, private thread.
const ALERT_CHANNELS: &[ChannelType] =
    &[ChannelType::Text, ChannelType::Announcement, ChannelType::PrivateThread];

/// Build a CV2 panel message. Returns the component tree (pass to `ui::send`).
pub fn build_panel_cv2(panel: &crate::db::Panel, types: &[TicketType]) -> Vec<Component> {
    use crate::context::cid_panel_btn;
    use crate::context::CID_PANEL_SELECT;

    let color = colours::from_hex(&panel.color);

    let mut header = format!("**{}**", panel.title);
    if let Some(ref d) = panel.description {
        header.push_str(&format!("\n{}", d));
    }

    let action = if panel.layout == "select" {
        let options: Vec<SelectOption> = types
            .iter()
            .map(|t| {
                let mut opt = SelectOption::new(t.id.to_string(), &t.label);
                if let Some(ref d) = t.description {
                    opt = opt.description(d);
                }
                if let Some(ref e) = t.emoji {
                    opt = opt.emoji(e);
                }
                opt
            })
            .collect();
        ui::action_row(vec![StringSelect::new(CID_PANEL_SELECT, options)
            .placeholder("Select a ticket category…")
            .into()])
    } else {
        let buttons: Vec<Component> = types
            .iter()
            .take(5)
            .map(|t| {
                let mut b = Button::new(cid_panel_btn(t.id), &t.label, ButtonStyle::Primary);
                if let Some(ref e) = t.emoji {
                    b = b.emoji(e);
                }
                b.into()
            })
            .collect();
        ui::action_row(buttons)
    };

    vec![Container::new(vec![ui::text(header), ui::separator(false, Spacing::Small), action])
        .accent(color.0)
        .into()]
}

/// Build the dynamic intake-form modal for a ticket category (fields from DB).
pub fn build_open_modal(
    modal_id: String,
    ticket_type: &TicketType,
    fields: &[crate::db::FormField],
) -> serenity::CreateModal {
    let components: Vec<serenity::CreateActionRow> = fields
        .iter()
        .take(5)
        .map(|f| {
            let style = if f.style == "paragraph" {
                serenity::InputTextStyle::Paragraph
            } else {
                serenity::InputTextStyle::Short
            };
            let mut input =
                serenity::CreateInputText::new(style, &f.label, format!("ff_{}", f.id))
                    .required(f.required);
            if let Some(ref ph) = f.placeholder {
                input = input.placeholder(ph);
            }
            if let Some(min) = f.min_length {
                input = input.min_length(min as u16);
            }
            if let Some(max) = f.max_length {
                input = input.max_length(max as u16);
            }
            serenity::CreateActionRow::InputText(input)
        })
        .collect();

    serenity::CreateModal::new(modal_id, format!("Open a {} ticket", ticket_type.label))
        .components(components)
}

/// Build the ephemeral CV2 configuration form for a ticket category. Exposes
/// every `TicketType` field with appropriate component types.
pub fn build_category_config_form(cat: &TicketType, ping_role_ids: &[String]) -> Vec<Component> {
    let id = cat.id;

    let header = format!(
        "**⚙️ {}{} Configuration**\n`{}` · {}",
        cat.emoji.as_deref().map(|e| format!("{} ", e)).unwrap_or_default(),
        cat.label,
        cat.name,
        cat.description.as_deref().unwrap_or("*No description set*"),
    );

    let auto_staff_btn = Button::new(
        format!("cat:cfg:{}:auto_staff", id),
        format!(
            "{} Auto-Add Staff: {}",
            if cat.auto_add_staff { "✅" } else { "❌" },
            if cat.auto_add_staff { "On" } else { "Off" }
        ),
        if cat.auto_add_staff { ButtonStyle::Success } else { ButtonStyle::Danger },
    );
    let has_form_btn = Button::new(
        format!("cat:cfg:{}:has_form", id),
        format!(
            "{} Intake Form: {}",
            if cat.has_form { "✅" } else { "❌" },
            if cat.has_form { "Enabled" } else { "Disabled" }
        ),
        if cat.has_form { ButtonStyle::Success } else { ButtonStyle::Danger },
    );

    let auto_close_text = match cat.auto_close_hours {
        Some(h) if h > 0 => format!("after {} hours", h),
        _ => "disabled".to_string(),
    };

    let edit = |field: &str, label: &str| -> Component {
        Button::new(format!("cat:cfg:{}:{}", id, field), label, ButtonStyle::Secondary)
            .emoji("✏️")
            .into()
    };

    let ping_select = RoleSelect::new(format!("cat:cfg:{}:ping_roles", id))
        .placeholder("Select roles to ping…")
        .max_values(10)
        .defaults(ping_role_ids.iter().cloned());
    let alert_select = ChannelSelect::new(format!("cat:cfg:{}:alert_ch", id), ALERT_CHANNELS)
        .placeholder("Select a channel…")
        .default(cat.staff_alert_channel_id.as_deref());

    vec![Container::new(vec![
        ui::text(header),
        ui::separator(false, Spacing::Small),
        ui::text("**🏷️ Basic Info** · Label, emoji, accent color, and description shown in select menus"),
        ui::action_row(vec![edit("btn_basic", "Edit Basic Info")]),
        ui::text("**💬 Welcome Message** · Posted in the ticket thread when it opens, with `{user}`, `{username}`, and `{type}` placeholders"),
        ui::action_row(vec![edit("btn_welcome", "Edit Welcome Message")]),
        ui::text(format!("**📝 Thread Name Pattern** · Current: `{}`\nAvailable: `{{number}}`, `{{username}}`, `{{type}}`", cat.thread_name_pattern)),
        ui::action_row(vec![edit("btn_thread", "Edit Thread Pattern")]),
        ui::separator(false, Spacing::Small),
        ui::text("**🔔 Ping Roles** · Roles mentioned when a new ticket of this type opens"),
        ui::action_row(vec![ping_select.into()]),
        ui::text("**📢 Staff Alert Channel** · Channel where a notification card is posted for each new ticket"),
        ui::action_row(vec![alert_select.into()]),
        ui::separator(false, Spacing::Small),
        ui::text("**⚙️ Behaviour**\n- **Auto-Add Staff** adds all users holding a ping role to the thread automatically\n- **Intake Form** shows users a questionnaire before the thread is created"),
        ui::action_row(vec![auto_staff_btn.into(), has_form_btn.into()]),
        ui::separator(false, Spacing::Small),
        ui::text(format!(
            "**📊 Limits**\nMax open per user: **{}** · Auto-close: **{}**",
            cat.max_open_per_user, auto_close_text,
        )),
        ui::action_row(vec![
            Button::new(format!("cat:cfg:{}:num_max_open", id), "Set Max Open", ButtonStyle::Secondary).emoji("🔢").into(),
            Button::new(format!("cat:cfg:{}:num_auto_close", id), "Set Auto-Close", ButtonStyle::Secondary).emoji("⏰").into(),
        ]),
    ])
    .accent(colours::BLURPLE.0)
    .into()]
}
