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

/// A select option for a ticket category: label plus optional description/emoji.
fn category_option(t: &TicketType) -> SelectOption {
    let mut opt = SelectOption::new(t.id.to_string(), &t.label);
    if let Some(ref d) = t.description {
        opt = opt.description(d);
    }
    if let Some(ref e) = t.emoji {
        opt = opt.emoji(e);
    }
    opt
}

/// Build a CV2 panel message. Returns the component tree (pass to `ui::send`).
pub fn build_panel_cv2(panel: &crate::db::Panel, types: &[TicketType]) -> Vec<Component> {
    use crate::context::cid_panel_btn;
    use crate::context::CID_PANEL_SELECT;

    let color = colours::from_hex(&panel.color);

    let mut header = format!("**{}**", panel.title);
    if let Some(ref d) = panel.description {
        header.push_str(&format!("\n{}", d));
    }

    let action = if types.is_empty() {
        // A live panel can end up with no categories (all deselected or deleted);
        // an empty select/button row is invalid, so show a notice instead.
        ui::text("-# No ticket categories are available right now.")
    } else if panel.layout == "select" {
        let options: Vec<SelectOption> = types.iter().take(25).map(category_option).collect();
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

/// Channel types a panel can be published to. Standard text channels only:
/// tickets open as private threads, which Discord forbids in announcement
/// channels, so offering those would let every ticket-open fail.
const PANEL_PUBLISH_CHANNELS: &[ChannelType] = &[ChannelType::Text];

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
            .take(25)
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
/// `linked_ids` are the ones currently on this panel; `taken_ids` are linked
/// to a different panel (a category can only be on one, so selecting those is
/// rejected).
pub fn build_panel_config_form(
    panel: &crate::db::Panel,
    all_cats: &[TicketType],
    linked_ids: &[i64],
    taken_ids: &[i64],
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
        // Discord caps a select at 25 options. Order currently-linked categories
        // first so they're always selectable — otherwise a category pushed past the
        // 25th slot would be silently unlinked the next time staff change this
        // selection, since the submission only carries the visible options.
        let mut ordered: Vec<&TicketType> = all_cats.iter().collect();
        ordered.sort_by_key(|c| !linked_ids.contains(&c.id));
        let truncated = ordered.len() > 25;
        let opts: Vec<SelectOption> = ordered
            .iter()
            .take(25)
            .map(|c| {
                let mut opt = SelectOption::new(c.id.to_string(), &c.label).default(linked_ids.contains(&c.id));
                if let Some(ref e) = c.emoji {
                    opt = opt.emoji(e);
                }
                if taken_ids.contains(&c.id) {
                    opt = opt.description("On another panel — remove it there first");
                }
                opt
            })
            .collect();
        let max = opts.len() as u8;
        body.push(ui::action_row(vec![StringSelect::new(cid_panel_cfg(id, "cats"), opts)
            .placeholder("Select categories to show…")
            .min_values(0)
            .max_values(max)
            .into()]));
        if truncated {
            body.push(ui::text("-# ⚠️ Only the first 25 categories are shown (Discord's limit); currently-linked ones are listed first so they stay selectable."));
        }
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

/// The panel basics modal (title / description / accent colour), shared by the
/// create-panel and edit-basics flows. Pass the panel to pre-fill current values.
pub fn build_panel_basics_modal(
    custom_id: String,
    title: &str,
    panel: Option<&crate::db::Panel>,
) -> ui::Modal {
    ui::Modal::new(custom_id, title)
        .text_row("pnl_title", "Panel title", ui::TextInputStyle::Short, "e.g. Support", true, panel.map(|p| p.title.as_str()))
        .text_row("pnl_desc", "Description", ui::TextInputStyle::Paragraph, "Shown under the title", false, panel.and_then(|p| p.description.as_deref()))
        .text_row("pnl_color", "Accent colour (hex)", ui::TextInputStyle::Short, "e.g. 5865F2", false, panel.map(|p| p.color.as_str()))
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
/// Returns `None` when no question is renderable (no fields, or only option-less
/// selects/checkboxes) — Discord rejects a modal with zero components, so the
/// caller must open the ticket directly instead.
pub fn build_open_modal(
    modal_id: String,
    ticket_type: &TicketType,
    fields: &[crate::db::FormField],
) -> Option<serenity::CreateModal<'static>> {
    let components: Vec<serenity::CreateModalComponent<'static>> = fields
        .iter()
        .take(5)
        .filter_map(build_intake_field)
        .collect();
    if components.is_empty() {
        return None;
    }

    Some(
        serenity::CreateModal::new(modal_id, format!("Open a {} ticket", ticket_type.label))
            .components(components),
    )
}

/// Discord modal `Label` text must be 1–45 characters; clamp to stay valid.
fn cap_label(s: &str) -> String {
    s.chars().take(45).collect()
}

/// The colour dropdown used by the category basic-info modal. Preselects the
/// palette entry matching `current`; a stored colour outside the palette is
/// offered as an extra pre-selected option so re-saving can't clobber it.
fn colour_select(current: &str) -> serenity::CreateModalComponent<'static> {
    let current = current.trim_start_matches('#').to_uppercase();
    let mut options: Vec<serenity::CreateSelectMenuOption> = COLOUR_PALETTE
        .iter()
        .map(|(name, hex)| {
            serenity::CreateSelectMenuOption::new(*name, *hex)
                .default_selection(hex.eq_ignore_ascii_case(&current))
        })
        .collect();
    if !COLOUR_PALETTE.iter().any(|(_, hex)| hex.eq_ignore_ascii_case(&current)) {
        options.push(
            serenity::CreateSelectMenuOption::new(format!("Current (#{})", current), current.clone())
                .default_selection(true),
        );
    }
    serenity::CreateModalComponent::Label(serenity::CreateLabel::select_menu(
        "Accent colour",
        serenity::CreateSelectMenu::new(
            "cat_color",
            serenity::CreateSelectMenuKind::String { options: options.into() },
        )
        .min_values(1)
        .max_values(1),
    ))
}

/// The shared category basic-info modal: label, emoji, description, and accent
/// colour. With `None` it's the create modal (`m:cat:create`, used by the hub
/// button and the `/ticket category create` command); with a category it's the
/// prefilled edit modal (`m:cat:basic:{id}`).
pub fn build_category_basic_modal(cat: Option<&TicketType>) -> serenity::CreateModal<'static> {
    let (modal_id, title) = match cat {
        Some(c) => (format!("m:cat:basic:{}", c.id), "🏷️ Edit Basic Info"),
        None => ("m:cat:create".to_string(), "➕ New category"),
    };
    serenity::CreateModal::new(modal_id, title).components(vec![
        crate::util::modal_input(
            "Name members see (button label)",
            "cat_label",
            false,
            true,
            Some("e.g. General Support"),
            cat.map(|c| c.label.as_str()),
        ),
        crate::util::modal_input(
            "Emoji",
            "cat_emoji",
            false,
            false,
            Some("e.g. 🎫"),
            cat.and_then(|c| c.emoji.as_deref()),
        ),
        crate::util::modal_input(
            "Short description",
            "cat_description",
            true,
            false,
            Some("Shown in menus to help members choose"),
            cat.and_then(|c| c.description.as_deref()),
        ),
        colour_select(cat.map(|c| c.color.as_str()).unwrap_or("5865F2")),
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
            // Discord defaults an omitted `required` to true, so an optional
            // question must send required:false explicitly (min_values alone
            // doesn't do it for selects).
            let menu = serenity::CreateSelectMenu::new(
                id,
                serenity::CreateSelectMenuKind::String { options: options.into() },
            )
            .min_values(if f.required { 1 } else { 0 })
            .max_values(1)
            .required(f.required);
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

/// Accent-colour palette offered as a dropdown in the category basic-info modal.
const COLOUR_PALETTE: &[(&str, &str)] = &[
    ("Blurple", "5865F2"),
    ("Green", "57F287"),
    ("Yellow", "FEE75C"),
    ("Orange", "E67E22"),
    ("Red", "ED4245"),
    ("Grey", "99AAB5"),
];

/// Question types offered by the Questions-tab add select, as `(value, label, description)`.
const QUESTION_TYPES: &[(&str, &str, &str)] = &[
    ("short", "Short answer", "A single line of text"),
    ("paragraph", "Paragraph", "A longer, multi-line answer"),
    ("select", "Dropdown", "Pick one from a list of choices"),
    ("checkbox", "Checkboxes", "Pick any number from a list"),
];

/// The single add/edit-question modal. The question type is chosen from a select
/// *before* this opens (a modal submit can't open another modal), so one modal
/// collects everything: label, required, placeholder, and — for dropdown/checkbox
/// questions — the choices. With a field it's the prefilled edit modal
/// (`m:ff:edit:{id}`); without, it creates a new `style` question
/// (`m:ff:new:{type_id}:{style}`).
pub fn build_question_modal(
    type_id: i64,
    style: &str,
    existing: Option<&crate::db::FormField>,
) -> serenity::CreateModal<'static> {
    let (modal_id, title) = match existing {
        Some(f) => (
            crate::ids::cid_form_field_edit_modal(f.id),
            format!("✏️ Edit {} question", question_type_label(style)),
        ),
        None => (
            crate::ids::cid_form_field_new_modal(type_id, style),
            format!("➕ New {} question", question_type_label(style)),
        ),
    };

    let label = serenity::CreateModalComponent::Label(serenity::CreateLabel::input_text(
        "Question",
        {
            let mut input = serenity::CreateInputText::new(serenity::InputTextStyle::Short, "ff_label")
                .placeholder("e.g. Describe your issue")
                .max_length(45)
                .required(true);
            if let Some(f) = existing {
                input = input.value(f.label.clone());
            }
            input
        },
    ));
    let required = crate::util::single_checkbox(
        "Answer required?",
        "ff_required",
        "Members must answer this",
        existing.map(|f| f.required).unwrap_or(true),
    );
    let placeholder_hint = if matches!(style, "select" | "checkbox") {
        "Shown before a choice is picked"
    } else {
        "Shown in the empty answer box"
    };
    let placeholder = crate::util::modal_input(
        "Placeholder hint",
        "ff_placeholder",
        false,
        false,
        Some(placeholder_hint),
        existing.and_then(|f| f.placeholder.as_deref()),
    );

    let mut components = vec![label, required, placeholder];
    if matches!(style, "select" | "checkbox") {
        let current = existing.map(|f| f.options_vec().join("\n")).unwrap_or_default();
        components.push(crate::util::modal_input(
            "Choices — one per line",
            "ff_options",
            true,
            true,
            Some("Bug\nBilling\nOther"),
            if current.is_empty() { None } else { Some(current.as_str()) },
        ));
    }

    serenity::CreateModal::new(modal_id, title).components(components)
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

/// How many categories the hub lists as individual Section rows before falling
/// back to a select. Each Section costs ~3 components against Discord's
/// 40-per-message cap, so a big list must collapse into one dropdown.
const HUB_SECTION_LIMIT: usize = 10;

/// The category management hub: each category as a row with an inline Configure
/// button (a select fallback beyond [`HUB_SECTION_LIMIT`]) and a button to create
/// a new one. `question_counts` maps `ticket_type_id → intake question count`
/// (from [`crate::db::Db::count_form_fields_by_type`]). The friendly entry point
/// replacing the old `create`/`list`/`configure`/`delete` slash commands.
pub fn build_category_hub(
    cats: &[TicketType],
    question_counts: &std::collections::HashMap<i64, i64>,
) -> Vec<Component> {
    use crate::context::{cid_cat_hub_configure, CID_CAT_HUB_CREATE, CID_CAT_HUB_SELECT};

    let line = |c: &TicketType| {
        let emoji = c.emoji.as_deref().map(|e| format!("{} ", e)).unwrap_or_default();
        let form = match question_counts.get(&c.id).copied().unwrap_or(0) {
            0 => String::new(),
            1 => " · 📝 1 question".to_string(),
            n => format!(" · 📝 {} questions", n),
        };
        let desc = c.description.as_deref().unwrap_or("No description");
        format!("{}**{}**\n-# {}{}", emoji, c.label, desc, form)
    };

    let mut items: Vec<Component> = vec![
        ui::text(
            "## 🗂️ Ticket Categories\n-# A category is a type of ticket members can open (e.g. *General Support*). Configure one below, or create a new one.",
        ),
        ui::separator(true, Spacing::Small),
    ];

    if cats.is_empty() {
        items.push(ui::text("*No categories yet.* Create your first one to get started."));
    } else if cats.len() <= HUB_SECTION_LIMIT {
        for c in cats {
            items.push(
                Section::new(
                    vec![ui::text(line(c))],
                    Button::new(cid_cat_hub_configure(c.id), "Configure", ButtonStyle::Secondary)
                        .emoji("⚙️")
                        .into(),
                )
                .into(),
            );
        }
    } else {
        let lines: Vec<String> = cats.iter().map(line).collect();
        items.push(ui::text(lines.join("\n")));
        let options: Vec<SelectOption> = cats.iter().take(25).map(category_option).collect();
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

/// The three pages of the category config panel, swapped in place via the tab row
/// (`cat:cfg:{id}:tab:{tab}`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ConfigTab {
    #[default]
    Overview,
    Behaviour,
    Questions,
}

impl ConfigTab {
    const ALL: [ConfigTab; 3] = [ConfigTab::Overview, ConfigTab::Behaviour, ConfigTab::Questions];

    /// The `{tab}` custom_id segment.
    pub fn id(self) -> &'static str {
        match self {
            ConfigTab::Overview => "overview",
            ConfigTab::Behaviour => "behaviour",
            ConfigTab::Questions => "questions",
        }
    }

    /// Parse a `{tab}` custom_id segment (unknown values fall back to Overview).
    pub fn parse(s: &str) -> Self {
        match s {
            "behaviour" => ConfigTab::Behaviour,
            "questions" => ConfigTab::Questions,
            _ => ConfigTab::Overview,
        }
    }

    fn label(self) -> &'static str {
        match self {
            ConfigTab::Overview => "Overview",
            ConfigTab::Behaviour => "Behaviour",
            ConfigTab::Questions => "Questions",
        }
    }

    fn emoji(self) -> &'static str {
        match self {
            ConfigTab::Overview => "🏷️",
            ConfigTab::Behaviour => "⚙️",
            ConfigTab::Questions => "📝",
        }
    }
}

/// Palette name for a stored colour, or the raw `#RRGGBB` when it isn't a preset.
fn colour_name(hex: &str) -> String {
    let h = hex.trim_start_matches('#').to_uppercase();
    COLOUR_PALETTE
        .iter()
        .find(|(_, p)| p.eq_ignore_ascii_case(&h))
        .map(|(name, _)| (*name).to_string())
        .unwrap_or_else(|| format!("#{}", h))
}

/// A config-panel Section row: current value as text, an ✏️ edit button
/// (`cat:cfg:{type_id}:{field}`) as the accessory.
fn edit_row(type_id: i64, field: &str, text: String) -> Component {
    Section::new(
        vec![ui::text(text)],
        Button::new(format!("cat:cfg:{}:{}", type_id, field), "Edit", ButtonStyle::Secondary)
            .emoji("✏️")
            .into(),
    )
    .into()
}

/// The Overview tab: identity (basic info), welcome message, thread name, delete.
fn config_tab_overview(cat: &TicketType) -> Vec<Component> {
    let id = cat.id;

    let basic = format!(
        "### 🏷️ Basic Info\n-# **{}**{} · {} accent\n-# {}",
        cat.label,
        cat.emoji.as_deref().map(|e| format!(" · {}", e)).unwrap_or_default(),
        colour_name(&cat.color),
        cat.description.as_deref().unwrap_or("*No description — shown in menus to help members choose.*"),
    );

    let welcome_preview = match cat.welcome_message.as_deref() {
        Some(w) => {
            let mut p: String = w.chars().take(120).collect();
            if p.len() < w.len() {
                p.push('…');
            }
            p
        }
        None => "*Not set — no greeting is posted.*".to_string(),
    };
    let welcome = format!(
        "### 💬 Welcome Message\n-# Posted in the thread when it opens.\n-# {}",
        welcome_preview,
    );

    let thread = format!("### 🧵 Thread Name\n-# `{}`", cat.thread_name_pattern);

    vec![
        edit_row(id, "btn_basic", basic),
        edit_row(id, "btn_welcome", welcome),
        edit_row(id, "btn_thread", thread),
        ui::separator(true, Spacing::Small),
        ui::action_row(vec![Button::new(
            format!("cat:cfg:{}:delete", id),
            "Delete Category",
            ButtonStyle::Danger,
        )
        .emoji("🗑️")
        .into()]),
    ]
}

/// The Behaviour tab: staff roles, limits, and the alert channel.
fn config_tab_behaviour(cat: &TicketType, staff_role_ids: &[String]) -> Vec<Component> {
    let id = cat.id;

    let max_open_text = if cat.max_open_per_user <= 0 {
        "Unlimited".to_string()
    } else {
        cat.max_open_per_user.to_string()
    };
    let auto_close_text = match cat.auto_close_hours {
        Some(h) if h > 0 => format!("After {} hours of inactivity", h),
        _ => "Disabled".to_string(),
    };
    let change_row = |field: &str, text: String| -> Component {
        Section::new(
            vec![ui::text(text)],
            Button::new(format!("cat:cfg:{}:{}", id, field), "Change", ButtonStyle::Secondary).into(),
        )
        .into()
    };

    let staff_select = RoleSelect::new(format!("cat:cfg:{}:ping_roles", id))
        .placeholder("Select staff roles…")
        .max_values(10)
        .defaults(staff_role_ids.iter().cloned());
    let alert_select = ChannelSelect::new(format!("cat:cfg:{}:alert_ch", id), ALERT_CHANNELS)
        .placeholder("Select a channel…")
        .default(cat.staff_alert_channel_id.as_deref());

    vec![
        ui::text("### 👥 Staff Roles\n-# Pinged and pulled into each new ticket thread when it opens."),
        ui::action_row(vec![staff_select.into()]),
        ui::text("### 📢 Staff Alert Channel\n-# A card with a Join button is posted here for each new ticket (no extra ping)."),
        ui::action_row(vec![alert_select.into()]),
        ui::separator(false, Spacing::Small),
        change_row("num_max_open", format!("### 🔢 Max Open Per User — **{}**", max_open_text)),
        change_row("num_auto_close", format!("### ⏰ Auto-Close — **{}**", auto_close_text)),
    ]
}

/// The Questions tab: one Section per intake question with an inline Edit button,
/// a remove select, and an add-question type select.
fn config_tab_questions(cat: &TicketType, fields: &[crate::db::FormField]) -> Vec<Component> {
    let id = cat.id;

    let intro = format!(
        "### 📝 Intake Form Questions ({}/5)\n-# Shown to the member before the ticket opens. No questions = no form.",
        fields.len(),
    );
    let mut items: Vec<Component> = vec![ui::text(intro)];

    if fields.is_empty() {
        items.push(ui::text(
            "*No questions yet* — members open this ticket without filling anything in. Add up to 5 below.",
        ));
    } else {
        for (i, f) in fields.iter().enumerate() {
            let needs = if f.needs_options() && f.options_vec().is_empty() {
                " · ⚠️ needs choices"
            } else {
                ""
            };
            items.push(
                Section::new(
                    vec![ui::text(format!(
                        "{}. **{}**\n-# {} · {}{}",
                        i + 1,
                        f.label,
                        question_type_label(&f.style),
                        if f.required { "required" } else { "optional" },
                        needs,
                    ))],
                    Button::new(
                        format!("cat:cfg:{}:ff_edit:{}", id, f.id),
                        "Edit",
                        ButtonStyle::Secondary,
                    )
                    .emoji("✏️")
                    .into(),
                )
                .into(),
            );
        }
        let opts: Vec<SelectOption> = fields
            .iter()
            .map(|f| SelectOption::new(f.id.to_string(), &f.label))
            .collect();
        items.push(ui::action_row(vec![StringSelect::new(
            format!("cat:cfg:{}:ff_remove", id),
            opts,
        )
        .placeholder("🗑️ Remove a question…")
        .into()]));
    }

    if fields.len() >= 5 {
        items.push(ui::text("-# Maximum of 5 questions reached (Discord's modal limit)."));
    } else {
        let type_opts: Vec<SelectOption> = QUESTION_TYPES
            .iter()
            .map(|(val, lbl, desc)| SelectOption::new(*val, *lbl).description(*desc))
            .collect();
        items.push(ui::action_row(vec![StringSelect::new(
            format!("cat:cfg:{}:ff_add", id),
            type_opts,
        )
        .placeholder("➕ Add a question…")
        .into()]));
    }

    items
}

/// Build the ephemeral CV2 configuration panel for a ticket category: shared
/// header + tab row, then the requested tab's Section rows. Every `TicketType`
/// field is reachable from one of the three tabs.
pub fn build_category_config_form(
    cat: &TicketType,
    staff_role_ids: &[String],
    fields: &[crate::db::FormField],
    tab: ConfigTab,
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

    let tab_row = ui::action_row(
        ConfigTab::ALL
            .iter()
            .map(|t| {
                Button::new(
                    format!("cat:cfg:{}:tab:{}", id, t.id()),
                    t.label(),
                    if *t == tab { ButtonStyle::Primary } else { ButtonStyle::Secondary },
                )
                .emoji(t.emoji())
                .disabled(*t == tab)
                .into()
            })
            .collect(),
    );

    let mut body: Vec<Component> = vec![
        Section::new(vec![ui::text(header)], back_btn.into()).into(),
        tab_row,
        ui::separator(false, Spacing::Small),
    ];
    body.extend(match tab {
        ConfigTab::Overview => config_tab_overview(cat),
        ConfigTab::Behaviour => config_tab_behaviour(cat, staff_role_ids),
        ConfigTab::Questions => config_tab_questions(cat, fields),
    });

    vec![Container::new(body).accent(colours::from_hex(&cat.color).0).into()]
}

/// The in-thread CV2 ticket card: header, welcome body (with form responses and
/// any reported-message context), a claim/priority status line, and the
/// Claim/Close buttons plus a priority select. Everything it shows derives from
/// the persisted `Ticket` row so the card can be re-rendered after claim,
/// unclaim, or priority changes.
pub fn build_ticket_card(
    cat: &TicketType,
    ticket: &crate::db::Ticket,
    username: &str,
) -> Vec<Component> {
    use crate::context::{cid_claim_btn, cid_priority_select, Priority, CID_CLOSE_BTN};

    let header = format!(
        "**{}{} — #{:04}**",
        cat.emoji.as_deref().map(|e| format!("{} ", e)).unwrap_or_default(),
        cat.label,
        ticket.ticket_number,
    );

    let default_welcome = format!(
        "Welcome, <@{}>!\nA member of staff will be with you shortly.",
        ticket.owner_id
    );
    let mut welcome = cat
        .welcome_message
        .clone()
        .unwrap_or(default_welcome)
        .replace("{user}", &format!("<@{}>", ticket.owner_id))
        .replace("{username}", username)
        .replace("{type}", &cat.label);

    if let Some(ref fr) = ticket.form_responses {
        if let Ok(map) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(fr) {
            welcome.push_str("\n\n**Your responses:**");
            for (k, v) in &map {
                let val = v.as_str().unwrap_or("");
                if !val.is_empty() {
                    let indented = val.replace('\n', "\n> ");
                    welcome.push_str(&format!("\n\n**{}:**\n> {}", k, indented));
                }
            }
        }
    }

    if ticket.reported_message_id.is_some() {
        welcome.push_str(&format!(
            "\n\n**⚑ Reported message:**\n> {}",
            ticket.reported_message_content.as_deref().unwrap_or("*(no content)*"),
        ));
        if let Some(ref author) = ticket.reported_author_id {
            welcome.push_str(&format!("\nAuthor: <@{}>", author));
        }
        if let Some(ref url) = ticket.reported_message_url {
            welcome.push_str(&format!("\n[Jump to message]({})", url));
        }
    }

    let priority = Priority::from_str(&ticket.priority);
    let mut status_parts: Vec<String> = vec![];
    if let Some(ref claimer) = ticket.claimed_by {
        status_parts.push(format!("✋ Claimed by <@{}>", claimer));
    }
    if !matches!(priority, Priority::Normal) {
        status_parts.push(format!("{} {} priority", priority.emoji(), priority.label()));
    }

    let claimed = ticket.claimed_by.is_some();
    let claim_btn = Button::new(
        cid_claim_btn(ticket.id),
        if claimed { "Claimed" } else { "Claim" },
        ButtonStyle::Secondary,
    )
    .emoji("✋")
    .disabled(claimed);
    let close_btn = Button::new(CID_CLOSE_BTN, "Close", ButtonStyle::Danger).emoji("🔒");

    let priority_opts: Vec<SelectOption> = [Priority::Low, Priority::Normal, Priority::High, Priority::Urgent]
        .iter()
        .map(|p| {
            SelectOption::new(p.as_str(), format!("{} {}", p.emoji(), p.label()))
                .default(p.as_str() == priority.as_str())
        })
        .collect();
    let priority_select = StringSelect::new(cid_priority_select(ticket.id), priority_opts)
        .placeholder("Set priority… (staff)");

    let mut body: Vec<Component> = vec![
        ui::text(header),
        ui::separator(false, Spacing::Small),
        ui::text(welcome),
    ];
    if !status_parts.is_empty() {
        body.push(ui::text(format!("-# {}", status_parts.join(" · "))));
    }
    body.push(ui::separator(true, Spacing::Small));
    body.push(ui::action_row(vec![claim_btn.into(), close_btn.into()]));
    body.push(ui::action_row(vec![priority_select.into()]));

    vec![Container::new(body).accent(colours::from_hex(&cat.color).0).into()]
}

/// The staff-alert-channel card posted when a ticket opens: a single read-mostly
/// notice with a Join button (idempotent — it doubles as "take me to the ticket").
/// The caller sends a separate role-ping message alongside this card so a
/// configured alert channel actually notifies staff.
pub fn build_staff_alert_card(cat: &TicketType, ticket: &crate::db::Ticket) -> Vec<Component> {
    vec![Container::new(vec![
        ui::text(format!("## 🎫 New {} Ticket", cat.label)),
        ui::text(format!(
            "-# **#{:04}** opened by <@{}> · <#{}>",
            ticket.ticket_number, ticket.owner_id, ticket.thread_id,
        )),
        ui::action_row(vec![Button::new(
            crate::ids::cid_ticket_join(ticket.id),
            "Join Ticket",
            ButtonStyle::Primary,
        )
        .emoji("🎟️")
        .into()]),
    ])
    .accent(colours::from_hex(&cat.color).0)
    .into()]
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
