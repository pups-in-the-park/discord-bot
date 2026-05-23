//! Message-context CV2 components: containers, text, separators, rows, buttons.

use serde::Serialize;

use super::select::{ChannelSelect, MentionableSelect, RoleSelect, StringSelect, UserSelect};
use super::{is_false, Emoji, Type};

/// Any CV2 component. Serializes untagged — each variant already carries its own
/// numeric `type` field, so the wrapper adds nothing to the JSON.
#[derive(Serialize)]
#[serde(untagged)]
pub enum Component {
    Container(Container),
    Section(Section),
    ActionRow(ActionRow),
    TextDisplay(TextDisplay),
    Separator(Separator),
    Button(Button),
    StringSelect(StringSelect),
    RoleSelect(RoleSelect),
    ChannelSelect(ChannelSelect),
    UserSelect(UserSelect),
    MentionableSelect(MentionableSelect),
    Label(super::modal::Label),
    TextInput(super::modal::TextInput),
}

// ── Container (type 17) ────────────────────────────────────────────────────────

/// Top-level CV2 wrapper. `accent_color` draws the coloured side bar.
#[derive(Serialize)]
pub struct Container {
    #[serde(rename = "type")]
    kind: Type<17>,
    components: Vec<Component>,
    #[serde(skip_serializing_if = "Option::is_none")]
    accent_color: Option<u32>,
}

impl Container {
    pub fn new(components: Vec<Component>) -> Self {
        Self { kind: Type, components, accent_color: None }
    }

    pub fn accent(mut self, color: u32) -> Self {
        self.accent_color = Some(color);
        self
    }
}

impl From<Container> for Component {
    fn from(c: Container) -> Self {
        Component::Container(c)
    }
}

// ── Text Display (type 10) ─────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct TextDisplay {
    #[serde(rename = "type")]
    kind: Type<10>,
    content: String,
}

impl TextDisplay {
    pub fn new(content: impl Into<String>) -> Self {
        Self { kind: Type, content: content.into() }
    }
}

impl From<TextDisplay> for Component {
    fn from(t: TextDisplay) -> Self {
        Component::TextDisplay(t)
    }
}

/// Shorthand for a [`TextDisplay`] component.
pub fn text(content: impl Into<String>) -> Component {
    Component::TextDisplay(TextDisplay::new(content))
}

// ── Separator (type 14) ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub enum Spacing {
    Small,
    Large,
}

impl Spacing {
    fn as_int(self) -> u8 {
        match self {
            Spacing::Small => 1,
            Spacing::Large => 2,
        }
    }
}

#[derive(Serialize)]
pub struct Separator {
    #[serde(rename = "type")]
    kind: Type<14>,
    divider: bool,
    spacing: u8,
}

impl Separator {
    pub fn new(divider: bool, spacing: Spacing) -> Self {
        Self { kind: Type, divider, spacing: spacing.as_int() }
    }
}

impl From<Separator> for Component {
    fn from(s: Separator) -> Self {
        Component::Separator(s)
    }
}

/// Shorthand for a [`Separator`] component.
pub fn separator(divider: bool, spacing: Spacing) -> Component {
    Component::Separator(Separator::new(divider, spacing))
}

// ── Action Row (type 1) ────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ActionRow {
    #[serde(rename = "type")]
    kind: Type<1>,
    components: Vec<Component>,
}

impl ActionRow {
    pub fn new(components: Vec<Component>) -> Self {
        Self { kind: Type, components }
    }
}

impl From<ActionRow> for Component {
    fn from(r: ActionRow) -> Self {
        Component::ActionRow(r)
    }
}

/// Shorthand for an [`ActionRow`] wrapping the given components.
pub fn action_row(components: Vec<Component>) -> Component {
    Component::ActionRow(ActionRow::new(components))
}

// ── Section (type 9) ───────────────────────────────────────────────────────────

/// A section pairs text components with a single accessory (button or thumbnail).
#[derive(Serialize)]
pub struct Section {
    #[serde(rename = "type")]
    kind: Type<9>,
    components: Vec<Component>,
    accessory: Box<Component>,
}

impl Section {
    pub fn new(components: Vec<Component>, accessory: Component) -> Self {
        Self { kind: Type, components, accessory: Box::new(accessory) }
    }
}

impl From<Section> for Component {
    fn from(s: Section) -> Self {
        Component::Section(s)
    }
}

// ── Button (type 2) ────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub enum ButtonStyle {
    Primary,
    Secondary,
    Success,
    Danger,
    Link,
}

impl ButtonStyle {
    fn as_int(self) -> u8 {
        match self {
            ButtonStyle::Primary => 1,
            ButtonStyle::Secondary => 2,
            ButtonStyle::Success => 3,
            ButtonStyle::Danger => 4,
            ButtonStyle::Link => 5,
        }
    }
}

#[derive(Serialize)]
pub struct Button {
    #[serde(rename = "type")]
    kind: Type<2>,
    style: u8,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    emoji: Option<Emoji>,
    #[serde(skip_serializing_if = "is_false")]
    disabled: bool,
}

impl Button {
    pub fn new(custom_id: impl Into<String>, label: impl Into<String>, style: ButtonStyle) -> Self {
        Self {
            kind: Type,
            style: style.as_int(),
            label: label.into(),
            custom_id: Some(custom_id.into()),
            url: None,
            emoji: None,
            disabled: false,
        }
    }

    pub fn link(url: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            kind: Type,
            style: ButtonStyle::Link.as_int(),
            label: label.into(),
            custom_id: None,
            url: Some(url.into()),
            emoji: None,
            disabled: false,
        }
    }

    pub fn emoji(mut self, emoji: impl Into<String>) -> Self {
        self.emoji = Some(Emoji::new(emoji));
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl From<Button> for Component {
    fn from(b: Button) -> Self {
        Component::Button(b)
    }
}
