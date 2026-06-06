//! Ticket message/CV2 builders: the published panel, the category configuration
//! form, and the dynamic intake-form modal.

use poise::serenity_prelude as serenity;

use crate::context::colours;
use crate::db::TicketType;
use crate::ui::{
    self, Button, ButtonStyle, ChannelSelect, ChannelType, Component, Container, RoleSelect,
    Section, SelectOption, Spacing, StringSelect,
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

/// Channel types a panel can be published to: text or announcement channels.
const PANEL_PUBLISH_CHANNELS: &[ChannelType] = &[ChannelType::Text, ChannelType::Announcement];

/// The panel management hub: a select to configure an existing panel and a button
/// to create a new one.
pub fn build_panel_hub(panels: &[crate::db::Panel]) -> Vec<Component> {
    use crate::context::{CID_PANEL_HUB_CREATE, CID_PANEL_HUB_SELECT};

    let mut items: Vec<Component> = vec![
        ui::text(
            "## 📋 Ticket Panels\n-# A panel is the message members click to open a ticket. Pick one to configure and publish, or create a new one.",
        ),
        ui::separator(true, Spacing::Small),
    ];

    if panels.is_empty() {
        items.push(ui::text("*No panels yet.* Create one, choose its categories, then publish it to a channel."));
    } else {
        let lines: Vec<String> = panels
            .iter()
            .map(|p| {
                let layout = if p.layout == "select" { "dropdown" } else { "buttons" };
                let pub_state = if p.message_id.is_some() { "✅ published" } else { "⚠️ not published" };
                format!("**{}**\n-# {} · {}", p.title, layout, pub_state)
            })
            .collect();
        items.push(ui::text(lines.join("\n")));
        let options: Vec<SelectOption> = panels
            .iter()
            .map(|p| {
                let layout = if p.layout == "select" { "dropdown" } else { "buttons" };
                SelectOption::new(p.id.to_string(), &p.title).description(layout)
            })
            .collect();
        items.push(ui::action_row(vec![StringSelect::new(CID_PANEL_HUB_SELECT, options)
            .placeholder("Configure a panel…")
            .into()]));
    }

    items.push(ui::action_row(vec![Button::new(
        CID_PANEL_HUB_CREATE,
        "Create Panel",
        ButtonStyle::Primary,
    )
    .emoji("➕")
    .into()]));

    vec![Container::new(items).accent(colours::BLURPLE.0).into()]
}

/// The panel configure form: edit basics, choose layout, pick which categories
/// appear, and publish to a channel. `all_cats` are the guild's categories;
/// `linked_ids` are the ones currently on this panel.
pub fn build_panel_config_form(
    panel: &crate::db::Panel,
    all_cats: &[TicketType],
    linked_ids: &[i64],
) -> Vec<Component> {
    use crate::context::{cid_panel_cfg, CID_PANEL_HUB_BACK};

    let id = panel.id;
    let color = colours::from_hex(&panel.color);

    let published = match (&panel.message_id, &panel.channel_id) {
        (Some(_), Some(ch)) => format!("✅ Published in <#{}>", ch),
        _ => "⚠️ Not published yet".to_string(),
    };
    let header = format!(
        "## 📋 {} Panel\n-# {}\n{}",
        panel.title,
        panel.description.as_deref().unwrap_or("No description set"),
        published,
    );
    let back_btn = Button::new(CID_PANEL_HUB_BACK, "Panels", ButtonStyle::Secondary).emoji("⬅️");

    let layout_select = StringSelect::new(
        cid_panel_cfg(id, "layout"),
        vec![
            SelectOption::new("buttons", "Buttons (up to 5 categories)").default(panel.layout != "select"),
            SelectOption::new("select", "Dropdown menu (any number)").default(panel.layout == "select"),
        ],
    )
    .placeholder("Choose a layout…");

    let mut body: Vec<Component> = vec![
        Section::new(vec![ui::text(header)], back_btn.into()).into(),
        ui::separator(false, Spacing::Small),
        ui::text("### ✏️ Basics\n-# Title, description, and accent colour."),
        ui::action_row(vec![Button::new(cid_panel_cfg(id, "basics"), "Edit Basics", ButtonStyle::Secondary).emoji("✏️").into()]),
        ui::text("### 🔘 Layout\n-# How categories appear to members."),
        ui::action_row(vec![layout_select.into()]),
        ui::text("### 🗂️ Categories on this panel\n-# Which ticket types members can open here."),
    ];

    if all_cats.is_empty() {
        body.push(ui::text("*No categories exist yet.* Create one in **Categories** first, then they'll appear here."));
    } else {
        let opts: Vec<SelectOption> = all_cats
            .iter()
            .map(|c| {
                let mut opt = SelectOption::new(c.id.to_string(), &c.label).default(linked_ids.contains(&c.id));
                if let Some(ref e) = c.emoji {
                    opt = opt.emoji(e);
                }
                opt
            })
            .collect();
        let max = all_cats.len().min(25) as u8;
        body.push(ui::action_row(vec![StringSelect::new(cid_panel_cfg(id, "cats"), opts)
            .placeholder("Select categories to show…")
            .min_values(0)
            .max_values(max)
            .into()]));
    }

    body.push(ui::separator(false, Spacing::Small));
    body.push(ui::text("### 🚀 Publish\n-# Select a channel to post (or re-post) this panel there."));
    body.push(ui::action_row(vec![ChannelSelect::new(cid_panel_cfg(id, "pub"), PANEL_PUBLISH_CHANNELS)
        .placeholder("Select a channel to publish to…")
        .default(panel.channel_id.as_deref())
        .into()]));
    body.push(ui::separator(true, Spacing::Small));
    body.push(ui::action_row(vec![Button::new(cid_panel_cfg(id, "delete"), "Delete Panel", ButtonStyle::Danger).emoji("🗑️").into()]));

    vec![Container::new(body).accent(color.0).into()]
}

/// A small confirmation prompt before deleting a panel.
pub fn build_panel_delete_confirm(panel: &crate::db::Panel) -> Vec<Component> {
    use crate::context::cid_panel_cfg;
    let id = panel.id;
    vec![Container::new(vec![
        ui::text(format!(
            "## 🗑️ Delete the {} panel?\n-# This removes the panel and its settings. Any already-posted panel message stays in the channel until you delete it manually.",
            panel.title
        )),
        ui::action_row(vec![
            Button::new(cid_panel_cfg(id, "delete_yes"), "Delete", ButtonStyle::Danger).into(),
            Button::new(cid_panel_cfg(id, "delete_no"), "Cancel", ButtonStyle::Secondary).into(),
        ]),
    ])
    .accent(colours::RED.0)
    .into()]
}

/// Build the dynamic intake-form modal for a ticket category (fields from DB).
pub fn build_open_modal(
    modal_id: String,
    ticket_type: &TicketType,
    fields: &[crate::db::FormField],
) -> serenity::CreateModal<'static> {
    let components: Vec<serenity::CreateModalComponent<'static>> = fields
        .iter()
        .take(5)
        .filter_map(build_intake_field)
        .collect();

    serenity::CreateModal::new(modal_id, format!("Open a {} ticket", ticket_type.label))
        .components(components)
}

/// Discord modal `Label` text must be 1–45 characters; clamp to stay valid.
fn cap_label(s: &str) -> String {
    s.chars().take(45).collect()
}

/// The shared "create category" modal (step 1 of the wizard). Used by both the
/// category hub button and the `/ticket category create` command.
pub fn build_category_create_modal() -> serenity::CreateModal<'static> {
    serenity::CreateModal::new("m:cat:create", "➕ New category — step 1 of 2").components(vec![
        crate::util::modal_input("Name members see (button label)", "cat_label", false, true, Some("e.g. General Support"), None),
        crate::util::modal_input("Emoji", "cat_emoji", false, false, Some("e.g. 🎫"), None),
        crate::util::modal_input("Short description", "cat_description", true, false, Some("Shown in menus to help members choose"), None),
    ])
}

/// Build one member-facing intake field from its stored definition. Returns `None`
/// for a select/checkbox question that has no options yet (skipped so the modal
/// stays valid).
fn build_intake_field(f: &crate::db::FormField) -> Option<serenity::CreateModalComponent<'static>> {
    let id = format!("ff_{}", f.id);
    let field_label = cap_label(&f.label);
    match f.style.as_str() {
        "select" => {
            let opts = f.options_vec();
            if opts.is_empty() {
                return None;
            }
            let options: Vec<serenity::CreateSelectMenuOption> = opts
                .iter()
                .map(|o| serenity::CreateSelectMenuOption::new(o.clone(), o.clone()))
                .collect();
            let menu = serenity::CreateSelectMenu::new(
                id,
                serenity::CreateSelectMenuKind::String { options: options.into() },
            )
            .min_values(if f.required { 1 } else { 0 })
            .max_values(1);
            let mut menu = menu;
            if let Some(ref ph) = f.placeholder {
                menu = menu.placeholder(ph.clone());
            }
            Some(serenity::CreateModalComponent::Label(serenity::CreateLabel::select_menu(
                field_label,
                menu,
            )))
        }
        "checkbox" => {
            let opts = f.options_vec();
            if opts.is_empty() {
                return None;
            }
            let options: Vec<serenity::CreateCheckboxGroupOption> = opts
                .iter()
                .map(|o| serenity::CreateCheckboxGroupOption::new(o.clone(), o.clone()))
                .collect();
            let count = options.len() as u8;
            let group = serenity::CreateCheckboxGroup::new(id, options)
                .min_values(if f.required { 1 } else { 0 })
                .max_values(count);
            Some(serenity::CreateModalComponent::Label(serenity::CreateLabel::checkbox_group(
                field_label,
                group,
            )))
        }
        _ => {
            let style = if f.style == "paragraph" {
                serenity::InputTextStyle::Paragraph
            } else {
                serenity::InputTextStyle::Short
            };
            let mut input = serenity::CreateInputText::new(style, id).required(f.required);
            if let Some(ref ph) = f.placeholder {
                input = input.placeholder(ph.clone());
            }
            if let Some(min) = f.min_length {
                input = input.min_length(min as u16);
            }
            if let Some(max) = f.max_length {
                input = input.max_length(max as u16);
            }
            Some(serenity::CreateModalComponent::Label(serenity::CreateLabel::input_text(
                field_label,
                input,
            )))
        }
    }
}

/// Accent-colour palette offered as a dropdown in the category wizard.
const COLOUR_PALETTE: &[(&str, &str)] = &[
    ("Blurple", "5865F2"),
    ("Green", "57F287"),
    ("Yellow", "FEE75C"),
    ("Orange", "E67E22"),
    ("Red", "ED4245"),
    ("Grey", "99AAB5"),
];

/// Question types offered in the add-question wizard, as `(value, label, description)`.
const QUESTION_TYPES: &[(&str, &str, &str)] = &[
    ("short", "Short answer", "A single line of text"),
    ("paragraph", "Paragraph", "A longer, multi-line answer"),
    ("select", "Dropdown", "Pick one from a list of choices"),
    ("checkbox", "Checkboxes", "Pick any number from a list"),
];

/// Step 1 of the add-question wizard: a modal capturing the label, the question
/// **type** (dropdown) and whether it's **required** (checkbox).
pub fn build_question_type_modal(type_id: i64) -> serenity::CreateModal<'static> {
    let label = serenity::CreateModalComponent::Label(serenity::CreateLabel::input_text(
        "Question",
        serenity::CreateInputText::new(serenity::InputTextStyle::Short, "ff_label")
            .placeholder("e.g. Describe your issue")
            .max_length(45)
            .required(true),
    ));

    let type_opts: Vec<serenity::CreateSelectMenuOption> = QUESTION_TYPES
        .iter()
        .enumerate()
        .map(|(i, (val, lbl, desc))| {
            serenity::CreateSelectMenuOption::new(*lbl, *val)
                .description(*desc)
                .default_selection(i == 0)
        })
        .collect();
    let type_menu = serenity::CreateSelectMenu::new(
        "ff_type",
        serenity::CreateSelectMenuKind::String { options: type_opts.into() },
    )
    .min_values(1)
    .max_values(1);
    let type_field = serenity::CreateModalComponent::Label(serenity::CreateLabel::select_menu(
        "Answer type",
        type_menu,
    ));

    let required = serenity::CreateModalComponent::Label(serenity::CreateLabel::checkbox_group(
        "Answer required?",
        serenity::CreateCheckboxGroup::new(
            "ff_required",
            vec![serenity::CreateCheckboxGroupOption::new("Members must answer this", "yes")
                .default_selection(true)],
        )
        .min_values(0)
        .max_values(1),
    ));

    serenity::CreateModal::new(crate::ids::cid_form_field_type_modal(type_id), "➕ Add a question")
        .components(vec![label, type_field, required])
}

/// Step 2 of the add-question wizard (dropdown/checkbox only): capture the choices
/// and an optional placeholder for an already-created field.
pub fn build_question_options_modal(field: &crate::db::FormField) -> serenity::CreateModal<'static> {
    let existing = field.options_vec().join("\n");
    let options = serenity::CreateModalComponent::Label(serenity::CreateLabel::input_text(
        "Choices — one per line",
        serenity::CreateInputText::new(serenity::InputTextStyle::Paragraph, "ff_options")
            .placeholder("Bug\nBilling\nOther")
            .value(existing)
            .required(true),
    ));
    let placeholder = serenity::CreateModalComponent::Label(serenity::CreateLabel::input_text(
        "Placeholder hint",
        serenity::CreateInputText::new(serenity::InputTextStyle::Short, "ff_placeholder")
            .placeholder("Shown before a choice is picked")
            .required(false),
    ));
    serenity::CreateModal::new(
        crate::ids::cid_form_field_options_modal(field.id),
        "Add the choices",
    )
    .components(vec![options, placeholder])
}

/// Step 2 of the category-creation wizard: appearance & behaviour, using a colour
/// **dropdown** and a behaviour **checkbox group**.
pub fn build_category_step2_modal(cat: &TicketType) -> serenity::CreateModal<'static> {
    let current = cat.color.trim_start_matches('#').to_uppercase();
    let colour_opts: Vec<serenity::CreateSelectMenuOption> = COLOUR_PALETTE
        .iter()
        .map(|(name, hex)| {
            serenity::CreateSelectMenuOption::new(*name, *hex)
                .default_selection(hex.eq_ignore_ascii_case(&current))
        })
        .collect();
    let colour = serenity::CreateModalComponent::Label(serenity::CreateLabel::select_menu(
        "Accent colour",
        serenity::CreateSelectMenu::new(
            "cat_color",
            serenity::CreateSelectMenuKind::String { options: colour_opts.into() },
        )
        .min_values(1)
        .max_values(1),
    ));

    let welcome = serenity::CreateModalComponent::Label(serenity::CreateLabel::input_text(
        "Welcome message",
        serenity::CreateInputText::new(serenity::InputTextStyle::Paragraph, "cat_welcome")
            .placeholder("Welcome {user}! Staff will be with you shortly.")
            .value(cat.welcome_message.clone().unwrap_or_default())
            .required(false),
    ));

    let pattern = serenity::CreateModalComponent::Label(serenity::CreateLabel::input_text(
        "Thread name pattern",
        serenity::CreateInputText::new(serenity::InputTextStyle::Short, "cat_pattern")
            .placeholder("ticket-{number}-{username}")
            .value(cat.thread_name_pattern.clone())
            .required(false),
    ));

    let behaviour = serenity::CreateModalComponent::Label(serenity::CreateLabel::checkbox_group(
        "Behaviour — select all that apply",
        serenity::CreateCheckboxGroup::new(
            "cat_behaviour",
            vec![
                serenity::CreateCheckboxGroupOption::new("Ask intake questions first", "form")
                    .default_selection(cat.has_form),
                serenity::CreateCheckboxGroupOption::new("Auto-add staff to the thread", "staff")
                    .default_selection(cat.auto_add_staff),
            ],
        )
        .min_values(0)
        .max_values(2),
    ));

    serenity::CreateModal::new(
        crate::ids::cid_category_step2_modal(cat.id),
        "Appearance & behaviour",
    )
    .components(vec![colour, welcome, pattern, behaviour])
}

/// Transitional card after creating a category: offer to customise (step 2) or finish.
pub fn build_category_step2_card(cat: &TicketType) -> Vec<Component> {
    let id = cat.id;
    vec![Container::new(vec![
        ui::text(format!("## ✅ {} created", cat.label)),
        ui::text("### Step 2 of 2 — appearance & behaviour"),
        ui::text("-# Set an accent colour, a welcome message, the thread-name pattern, and behaviour toggles. You can skip this and tweak everything later."),
        ui::action_row(vec![
            Button::new(format!("cat:cfg:{}:wizard", id), "Customise", ButtonStyle::Primary).emoji("🎨").into(),
            Button::new(format!("cat:cfg:{}:wizard_done", id), "Skip for now", ButtonStyle::Secondary).into(),
        ]),
    ])
    .accent(colours::from_hex(&cat.color).0)
    .into()]
}

/// Transitional card after step 1 of the add-question wizard, for dropdown/checkbox
/// questions that still need their choices.
pub fn build_question_options_card(type_id: i64, field: &crate::db::FormField) -> Vec<Component> {
    let kind = if field.style == "checkbox" { "checkboxes" } else { "dropdown" };
    vec![Container::new(vec![
        ui::text("## Almost done — add the choices"),
        ui::text(format!("-# Your **{}** question is a {} and needs at least one choice.", field.label, kind)),
        ui::action_row(vec![Button::new(
            format!("cat:cfg:{}:ff_opts:{}", type_id, field.id),
            "Add choices",
            ButtonStyle::Primary,
        )
        .emoji("📝")
        .into()]),
    ])
    .accent(colours::BLURPLE.0)
    .into()]
}

/// Friendly display name for a question's stored `style`.
fn question_type_label(style: &str) -> &'static str {
    match style {
        "paragraph" => "paragraph",
        "select" => "dropdown",
        "checkbox" => "checkboxes",
        _ => "short answer",
    }
}

/// The category management hub: a select to configure an existing category and a
/// button to create a new one. The friendly entry point replacing the old
/// `create`/`list`/`configure`/`delete` slash commands.
pub fn build_category_hub(cats: &[TicketType]) -> Vec<Component> {
    use crate::context::{CID_CAT_HUB_CREATE, CID_CAT_HUB_SELECT};

    let mut items: Vec<Component> = vec![
        ui::text(
            "## 🗂️ Ticket Categories\n-# A category is a type of ticket members can open (e.g. *General Support*). Pick one to configure, or create a new one.",
        ),
        ui::separator(true, Spacing::Small),
    ];

    if cats.is_empty() {
        items.push(ui::text("*No categories yet.* Create your first one to get started."));
    } else {
        let lines: Vec<String> = cats
            .iter()
            .map(|c| {
                let emoji = c.emoji.as_deref().map(|e| format!("{} ", e)).unwrap_or_default();
                let form = if c.has_form { " · 📝 intake form" } else { "" };
                let desc = c.description.as_deref().unwrap_or("No description");
                // Title on its own line; description as subtext beneath it.
                format!("{}**{}**\n-# {}{}", emoji, c.label, desc, form)
            })
            .collect();
        items.push(ui::text(lines.join("\n")));
        let options: Vec<SelectOption> = cats
            .iter()
            .map(|c| {
                let mut opt = SelectOption::new(c.id.to_string(), &c.label);
                if let Some(ref d) = c.description {
                    opt = opt.description(d);
                }
                if let Some(ref e) = c.emoji {
                    opt = opt.emoji(e);
                }
                opt
            })
            .collect();
        items.push(ui::action_row(vec![StringSelect::new(CID_CAT_HUB_SELECT, options)
            .placeholder("Configure a category…")
            .into()]));
    }

    items.push(ui::action_row(vec![Button::new(
        CID_CAT_HUB_CREATE,
        "Create Category",
        ButtonStyle::Primary,
    )
    .emoji("➕")
    .into()]));

    vec![Container::new(items).accent(colours::BLURPLE.0).into()]
}

/// Build the ephemeral CV2 configuration form for a ticket category. Exposes
/// every `TicketType` field with appropriate component types, including the intake
/// form fields (managed inline). `fields` is the category's current form fields.
pub fn build_category_config_form(
    cat: &TicketType,
    ping_role_ids: &[String],
    fields: &[crate::db::FormField],
) -> Vec<Component> {
    use crate::context::CID_CAT_HUB_BACK;

    let id = cat.id;

    let header = format!(
        "## ⚙️ {}{}\n-# {}",
        cat.emoji.as_deref().map(|e| format!("{} ", e)).unwrap_or_default(),
        cat.label,
        cat.description.as_deref().unwrap_or("No description set"),
    );
    let back_btn = Button::new(CID_CAT_HUB_BACK, "Categories", ButtonStyle::Secondary).emoji("⬅️");

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
    let max_open_text = if cat.max_open_per_user <= 0 {
        "Unlimited".to_string()
    } else {
        cat.max_open_per_user.to_string()
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

    // Intake-form-fields section: a list, a remove-select, and an Add button (capped
    // at Discord's 5-field modal limit).
    let fields_summary = if fields.is_empty() {
        "-# No questions yet — members open this ticket without filling anything in. Add up to 5 below.".to_string()
    } else {
        let lines: Vec<String> = fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let needs = if f.needs_options() && f.options_vec().is_empty() {
                    " · ⚠️ needs choices"
                } else {
                    ""
                };
                format!(
                    "{}. **{}**\n-# {} · {}{}",
                    i + 1,
                    f.label,
                    question_type_label(&f.style),
                    if f.required { "required" } else { "optional" },
                    needs,
                )
            })
            .collect();
        lines.join("\n")
    };
    let mut form_section: Vec<Component> = vec![ui::text(format!(
        "### 📝 Intake Form Questions ({}/5)\n-# Shown to the member before the ticket opens.\n{}",
        fields.len(),
        fields_summary,
    ))];
    if !fields.is_empty() {
        let opts: Vec<SelectOption> = fields
            .iter()
            .map(|f| SelectOption::new(f.id.to_string(), &f.label))
            .collect();
        form_section.push(ui::action_row(vec![StringSelect::new(
            format!("cat:cfg:{}:ff_remove", id),
            opts,
        )
        .placeholder("Remove a question…")
        .into()]));
    }
    let add_disabled = fields.len() >= 5;
    form_section.push(ui::action_row(vec![Button::new(
        format!("cat:cfg:{}:ff_add", id),
        "Add Question",
        ButtonStyle::Secondary,
    )
    .emoji("➕")
    .disabled(add_disabled)
    .into()]));

    let mut body: Vec<Component> = vec![
        Section::new(vec![ui::text(header)], back_btn.into()).into(),
        ui::separator(false, Spacing::Small),
        ui::text("### 🏷️ Basic Info\n-# Label, emoji, accent colour, and the description shown in menus."),
        ui::action_row(vec![edit("btn_basic", "Edit Basic Info")]),
        ui::text("### 💬 Welcome Message\n-# Posted in the thread when it opens. Placeholders: `{user}` (mentions them), `{username}` (their name), `{type}` (category label)."),
        ui::action_row(vec![edit("btn_welcome", "Edit Welcome Message")]),
        ui::text(format!("### 📝 Thread Name Pattern\n-# Current: `{}` · Placeholders: `{{number}}`, `{{username}}`, `{{type}}`", cat.thread_name_pattern)),
        ui::action_row(vec![edit("btn_thread", "Edit Thread Pattern")]),
        ui::separator(false, Spacing::Small),
        ui::text("### 🔔 Ping Roles\n-# Roles mentioned when a new ticket of this type opens."),
        ui::action_row(vec![ping_select.into()]),
        ui::text("### 📢 Staff Alert Channel\n-# A notification card is posted here for each new ticket."),
        ui::action_row(vec![alert_select.into()]),
        ui::separator(false, Spacing::Small),
        ui::text("### ⚙️ Behaviour\n-# **Auto-Add Staff** pulls everyone with a ping role into the thread. **Intake Form** shows the questions below before the thread opens."),
        ui::action_row(vec![auto_staff_btn.into(), has_form_btn.into()]),
        ui::text(format!(
            "### 📊 Limits\n-# Max open per user: **{}** · Auto-close: **{}**",
            max_open_text, auto_close_text,
        )),
        ui::action_row(vec![
            Button::new(format!("cat:cfg:{}:num_max_open", id), "Set Max Open", ButtonStyle::Secondary).emoji("🔢").into(),
            Button::new(format!("cat:cfg:{}:num_auto_close", id), "Set Auto-Close", ButtonStyle::Secondary).emoji("⏰").into(),
        ]),
    ];
    body.extend(form_section);
    body.push(ui::separator(true, Spacing::Small));
    body.push(ui::action_row(vec![Button::new(
        format!("cat:cfg:{}:delete", id),
        "Delete Category",
        ButtonStyle::Danger,
    )
    .emoji("🗑️")
    .into()]));

    vec![Container::new(body).accent(colours::BLURPLE.0).into()]
}

/// A small confirmation prompt before deactivating a category.
pub fn build_category_delete_confirm(cat: &TicketType) -> Vec<Component> {
    let id = cat.id;
    vec![Container::new(vec![
        ui::text(format!(
            "## 🗑️ Delete {}?\n-# Members will no longer be able to open this ticket type. Existing tickets are unaffected, and it's removed from any panels.",
            cat.label
        )),
        ui::action_row(vec![
            Button::new(format!("cat:cfg:{}:delete_yes", id), "Delete", ButtonStyle::Danger).into(),
            Button::new(format!("cat:cfg:{}:delete_no", id), "Cancel", ButtonStyle::Secondary).into(),
        ]),
    ])
    .accent(colours::RED.0)
    .into()]
}
