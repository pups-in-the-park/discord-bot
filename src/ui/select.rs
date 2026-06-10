//! Select-menu components (types 3, 5, 6, 7, 8). All five are valid in both
//! messages (inside an Action Row) and modals (inside a Label).

use serde::Serialize;

use super::component::Component;
use super::{is_false, Emoji, Type};

/// A pre-selected default for an auto-populated select (`{ "id", "type" }`).
#[derive(Serialize)]
pub struct DefaultValue {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: &'static str,
}

// ── String Select (type 3) ─────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct SelectOption {
    pub label: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<Emoji>,
    #[serde(skip_serializing_if = "is_false")]
    pub default: bool,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            description: None,
            emoji: None,
            default: false,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn emoji(mut self, emoji: impl Into<String>) -> Self {
        self.emoji = Some(Emoji::new(emoji));
        self
    }

    pub fn default(mut self, default: bool) -> Self {
        self.default = default;
        self
    }
}

#[derive(Serialize)]
pub struct StringSelect {
    #[serde(rename = "type")]
    kind: Type<3>,
    custom_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    placeholder: Option<String>,
    min_values: u8,
    max_values: u8,
    options: Vec<SelectOption>,
}

impl StringSelect {
    pub fn new(custom_id: impl Into<String>, mut options: Vec<SelectOption>) -> Self {
        // Discord rejects string selects with more than 25 options; enforce the
        // cap here so no view code can produce an invalid payload.
        options.truncate(25);
        Self {
            kind: Type,
            custom_id: custom_id.into(),
            placeholder: None,
            min_values: 1,
            max_values: 1,
            options,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn min_values(mut self, n: u8) -> Self {
        self.min_values = n;
        self
    }

    /// Clamped to the number of options (Discord rejects `max_values` above it).
    pub fn max_values(mut self, n: u8) -> Self {
        self.max_values = n.min(self.options.len().max(1) as u8);
        self
    }
}

impl From<StringSelect> for Component {
    fn from(s: StringSelect) -> Self {
        Component::StringSelect(s)
    }
}

// ── Snowflake selects (role / user / channel / mentionable) ────────────────────
//
// These differ only by the numeric `type` and (for channel) `channel_types`, so a
// macro keeps them consistent without repeating the builder surface.

macro_rules! snowflake_select {
    ($name:ident, $tag:literal, $default_kind:literal) => {
        #[derive(Serialize)]
        pub struct $name {
            #[serde(rename = "type")]
            kind: Type<$tag>,
            custom_id: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            placeholder: Option<String>,
            min_values: u8,
            max_values: u8,
            #[serde(skip_serializing_if = "Vec::is_empty")]
            default_values: Vec<DefaultValue>,
        }

        impl $name {
            pub fn new(custom_id: impl Into<String>) -> Self {
                Self {
                    kind: Type,
                    custom_id: custom_id.into(),
                    placeholder: None,
                    min_values: 0,
                    max_values: 1,
                    default_values: Vec::new(),
                }
            }

            pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
                self.placeholder = Some(placeholder.into());
                self
            }

            pub fn min_values(mut self, n: u8) -> Self {
                self.min_values = n;
                self
            }

            pub fn max_values(mut self, n: u8) -> Self {
                self.max_values = n;
                self
            }

            /// Pre-select the given snowflake IDs (the auto-populated defaults).
            pub fn defaults<I, S>(mut self, ids: I) -> Self
            where
                I: IntoIterator<Item = S>,
                S: Into<String>,
            {
                self.default_values = ids
                    .into_iter()
                    .map(|id| DefaultValue { id: id.into(), kind: $default_kind })
                    .collect();
                self
            }
        }

        impl From<$name> for Component {
            fn from(s: $name) -> Self {
                Component::$name(s)
            }
        }
    };
}

snowflake_select!(RoleSelect, 6, "role");
snowflake_select!(UserSelect, 5, "user");
snowflake_select!(MentionableSelect, 7, "mentionable");

// ── Channel Select (type 8) — adds channel_types ───────────────────────────────

/// Discord channel-type discriminants used to filter a channel select. Values
/// match Discord's API (`GUILD_TEXT=0`, `GUILD_ANNOUNCEMENT=5`,
/// `ANNOUNCEMENT_THREAD=10`, `PUBLIC_THREAD=11`, `PRIVATE_THREAD=12`, `GUILD_FORUM=15`).
#[derive(Clone, Copy)]
pub enum ChannelType {
    Text,
    Announcement,
    AnnouncementThread,
    PublicThread,
    PrivateThread,
    Forum,
}

impl ChannelType {
    fn as_int(self) -> u8 {
        match self {
            ChannelType::Text => 0,
            ChannelType::Announcement => 5,
            ChannelType::AnnouncementThread => 10,
            ChannelType::PublicThread => 11,
            ChannelType::PrivateThread => 12,
            ChannelType::Forum => 15,
        }
    }
}

#[derive(Serialize)]
pub struct ChannelSelect {
    #[serde(rename = "type")]
    kind: Type<8>,
    custom_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    placeholder: Option<String>,
    min_values: u8,
    max_values: u8,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    channel_types: Vec<u8>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    default_values: Vec<DefaultValue>,
}

impl ChannelSelect {
    pub fn new(custom_id: impl Into<String>, channel_types: &[ChannelType]) -> Self {
        Self {
            kind: Type,
            custom_id: custom_id.into(),
            placeholder: None,
            min_values: 0,
            max_values: 1,
            channel_types: channel_types.iter().map(|c| c.as_int()).collect(),
            default_values: Vec::new(),
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn max_values(mut self, n: u8) -> Self {
        self.max_values = n;
        self
    }

    /// Pre-select a single channel (the auto-populated default), if present.
    pub fn default(mut self, id: Option<impl Into<String>>) -> Self {
        if let Some(id) = id {
            self.default_values = vec![DefaultValue { id: id.into(), kind: "channel" }];
        }
        self
    }
}

impl From<ChannelSelect> for Component {
    fn from(s: ChannelSelect) -> Self {
        Component::ChannelSelect(s)
    }
}
