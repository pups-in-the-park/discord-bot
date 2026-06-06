//! Modal payloads and the modal-only components (`Label`, `TextInput`).
//!
//! Modern Discord modals wrap each interactive component in a [`Label`] (type 18)
//! rather than the now-deprecated Action Row. Any select type is also valid inside
//! a modal — build one with the `ui::*Select` builders and wrap it in a `Label`.

use serde::Serialize;

use super::component::Component;
use super::Type;

/// A modal interaction-response payload. `components` are top-level modal items —
/// usually [`Label`]s wrapping an input/select, or bare text displays.
#[derive(Serialize)]
pub struct Modal {
    pub custom_id: String,
    pub title: String,
    pub components: Vec<Component>,
}

impl Modal {
    pub fn new(custom_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self { custom_id: custom_id.into(), title: title.into(), components: Vec::new() }
    }

    pub fn component(mut self, component: impl Into<Component>) -> Self {
        self.components.push(component.into());
        self
    }

    /// Wrap a component in a [`Label`] and append it.
    pub fn labelled(self, label: impl Into<String>, component: impl Into<Component>) -> Self {
        self.component(Label::new(label, component))
    }

    /// Append a `Label`-wrapped text input (modal-components-v2 form). serenity
    /// `next` only deserializes `Label`-wrapped modal fields, so this is the
    /// round-trippable form. `value` pre-fills the field.
    pub fn text_row(
        self,
        custom_id: impl Into<String>,
        label: impl Into<String>,
        style: TextInputStyle,
        placeholder: impl Into<String>,
        required: bool,
        value: Option<&str>,
    ) -> Self {
        let input = TextInput::new(custom_id, style)
            .placeholder(placeholder)
            .required(required)
            .value(value);
        self.component(Label::new(label, input))
    }
}

// ── Label (type 18) ────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct Label {
    #[serde(rename = "type")]
    kind: Type<18>,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    component: Box<Component>,
}

impl Label {
    pub fn new(label: impl Into<String>, component: impl Into<Component>) -> Self {
        Self {
            kind: Type,
            label: label.into(),
            description: None,
            component: Box::new(component.into()),
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

impl From<Label> for Component {
    fn from(l: Label) -> Self {
        Component::Label(l)
    }
}

// ── Text Input (type 4) ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub enum TextInputStyle {
    Short,
    Paragraph,
}

impl TextInputStyle {
    fn as_int(self) -> u8 {
        match self {
            TextInputStyle::Short => 1,
            TextInputStyle::Paragraph => 2,
        }
    }
}

/// A text input. In the legacy Action-Row form it carries its own `label`; inside
/// a modern [`Label`]-wrapped modal the label lives on the `Label` and is omitted
/// here (serenity 0.12.5 only reads the Action-Row form back, so the macro emits
/// that — see `pip_macros`).
#[derive(Serialize)]
pub struct TextInput {
    #[serde(rename = "type")]
    kind: Type<4>,
    custom_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    style: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_length: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_length: Option<u16>,
    // Always serialized: Discord defaults `required` to `true` when omitted, so an
    // optional field (`required = false`) must send the flag explicitly or it renders
    // as required.
    required: bool,
}

impl TextInput {
    pub fn new(custom_id: impl Into<String>, style: TextInputStyle) -> Self {
        Self {
            kind: Type,
            custom_id: custom_id.into(),
            label: None,
            style: style.as_int(),
            value: None,
            placeholder: None,
            min_length: None,
            max_length: None,
            required: false,
        }
    }

    /// Set the inline label (Action-Row form). Omit for the `Label`-wrapped form.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Pre-fill the input (the DB-backed default value).
    pub fn value(mut self, value: Option<impl Into<String>>) -> Self {
        self.value = value.map(Into::into);
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn min_length(mut self, n: u16) -> Self {
        self.min_length = Some(n);
        self
    }

    pub fn max_length(mut self, n: u16) -> Self {
        self.max_length = Some(n);
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }
}

impl From<TextInput> for Component {
    fn from(t: TextInput) -> Self {
        Component::TextInput(t)
    }
}
