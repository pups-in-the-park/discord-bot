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
pub use modal::{Label, Modal, TextInput, TextInputStyle};
pub use respond::{
    edit, forward_message, open_modal, respond_ephemeral, send, slash_respond, update,
};
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

use poise::serenity_prelude::{LabelComponent, ModalComponent};

/// Read a submitted text-input value out of a modal submission by `custom_id`.
///
/// serenity `next` models modal submissions as `FixedArray<ModalComponent>` where
/// each interactive field is a `Label` wrapping the actual `InputText`/`SelectMenu`
/// (Discord's modal-components-v2 shape). Used by the `#[derive(Cv2Modal)]`-generated
/// `from_submission` and the feature modal handlers.
pub fn read_text<'a>(components: &'a [ModalComponent], custom_id: &str) -> Option<&'a str> {
    components.iter().find_map(|c| match c {
        ModalComponent::Label(label) => match &label.component {
            LabelComponent::InputText(t) if t.custom_id.as_str() == custom_id => {
                Some(t.value.as_str())
            }
            _ => None,
        },
        _ => None,
    })
}

/// Read the selected values of a `Label`-wrapped select menu in a modal submission
/// by `custom_id`. Returns an empty vec if the field is absent or nothing was chosen.
pub fn read_multi_select(components: &[ModalComponent], custom_id: &str) -> Vec<String> {
    components
        .iter()
        .find_map(|c| match c {
            ModalComponent::Label(label) => match &label.component {
                LabelComponent::SelectMenu(s) if s.custom_id.as_str() == custom_id => {
                    Some(s.values.iter().map(|v| v.to_string()).collect::<Vec<_>>())
                }
                _ => None,
            },
            _ => None,
        })
        .unwrap_or_default()
}

/// Read the checked values of a `Label`-wrapped checkbox group in a modal
/// submission by `custom_id`. Empty vec if the field is absent or nothing checked.
pub fn read_checkbox_group(components: &[ModalComponent], custom_id: &str) -> Vec<String> {
    components
        .iter()
        .find_map(|c| match c {
            ModalComponent::Label(label) => match &label.component {
                LabelComponent::CheckboxGroup(g) if g.custom_id.as_str() == custom_id => {
                    Some(g.values.iter().map(|v| v.to_string()).collect::<Vec<_>>())
                }
                _ => None,
            },
            _ => None,
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
