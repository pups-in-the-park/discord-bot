//! Typed Components V2 (CV2) UI kit.
//!
//! Serenity 0.12 has no typed builders for the full CV2 component set, so this
//! module models the components as Rust types that `Serialize` to the exact JSON
//! Discord expects. Construction is type-checked and discoverable; the JSON shape
//! lives in one place instead of being hand-written at every call site.
//!
//! This is a deliberately *complete* model of Discord's component set, so some
//! builders (user/mentionable selects, sections, link buttons, the `Label`-wrapped
//! modal form, …) exist without a current call site — hence the module-wide
//! `dead_code`/`unused_imports` allowance on this toolkit module.
#![allow(dead_code, unused_imports)]

mod component;
mod modal;
mod respond;
mod select;

pub use component::{
    action_row, separator, text, ActionRow, Button, ButtonStyle, Component, Container, Section,
    Separator, Spacing, TextDisplay,
};
pub use modal::{Modal, TextInput, TextInputStyle};
pub use respond::{edit, open_modal, respond_ephemeral, send, slash_respond, update};
pub use select::{
    ChannelSelect, ChannelType, MentionableSelect, RoleSelect, SelectOption, StringSelect,
    UserSelect,
};

/// `MessageFlags.IS_COMPONENTS_V2` (1 << 15). Required on any message/response
/// whose body is built from CV2 components.
pub const CV2_FLAG: u64 = 1 << 15;

/// Emoji reference used by buttons and select options (`{ "name": "🔒" }`).
#[derive(Clone, serde::Serialize)]
pub struct Emoji {
    pub name: String,
}

impl Emoji {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

/// Serializes a constant numeric component `type` discriminant via the derive.
pub(crate) struct Type<const N: u8>;

impl<const N: u8> serde::Serialize for Type<N> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u8(N)
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
pub(crate) fn is_false(b: &bool) -> bool {
    !*b
}

/// Read a submitted text-input value out of a modal interaction's action rows by
/// `custom_id`. serenity 0.12.5 models modal submissions as `Vec<ActionRow>` of
/// `InputText`, so this is the only round-trippable read path. Used by the
/// `#[derive(Cv2Modal)]`-generated `from_submission`.
pub fn read_text<'a>(
    components: &'a [poise::serenity_prelude::ActionRow],
    custom_id: &str,
) -> Option<&'a str> {
    components
        .iter()
        .flat_map(|row| &row.components)
        .find_map(|c| match c {
            poise::serenity_prelude::ActionRowComponent::InputText(t)
                if t.custom_id == custom_id =>
            {
                t.value.as_deref()
            }
            _ => None,
        })
}

#[cfg(test)]
mod tests;
