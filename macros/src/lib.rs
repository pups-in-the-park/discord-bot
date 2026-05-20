//! `#[derive(Cv2Modal)]` — turns a plain struct into a DB-prefillable Discord modal.
//!
//! The struct's *field values* become the modal's prefilled defaults, so the
//! typical flow is: load a row from the DB → build the struct → `.into_modal(id)`
//! → the user sees current values pre-filled. `from_submission` reads the user's
//! edits back into the same struct.
//!
//! ## Why Action-Row text inputs (not Label-wrapped)
//!
//! serenity 0.12.5 deserializes inbound modal submissions as `Vec<ActionRow>` of
//! `InputText`/`SelectMenu`; a `Label` (type 18) decodes to an empty
//! `ActionRow{kind: Unknown(18)}`, dropping the nested value. So the modern
//! Label-wrapped form (and every select-in-modal, which *requires* Label) is
//! send-only on this serenity version and cannot be read back. This derive emits
//! the legacy Action-Row text-input form, which round-trips correctly. Scope is
//! therefore text inputs only (`String` = required, `Option<String>` = optional).
//!
//! ```ignore
//! #[derive(Cv2Modal)]
//! #[modal(title = "🏷️ Edit Basic Info")]
//! struct CategoryBasicModal {
//!     #[field(label = "Button Label", placeholder = "e.g. General Support")]
//!     label: String,
//!     #[field(label = "Emoji (optional)", placeholder = "e.g. 🎫")]
//!     emoji: Option<String>,
//! }
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

/// Parsed `#[field(...)]` attributes for one struct field.
struct FieldSpec {
    ident: syn::Ident,
    custom_id: String,
    label: String,
    placeholder: Option<String>,
    paragraph: bool,
    required: bool,
    min_length: Option<u16>,
    max_length: Option<u16>,
    is_option: bool,
}

#[proc_macro_derive(Cv2Modal, attributes(modal, field))]
pub fn derive_cv2_modal(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_ident = &input.ident;

    // ── #[modal(title = "...")] ────────────────────────────────────────────────
    let mut title: Option<String> = None;
    for attr in &input.attrs {
        if attr.path().is_ident("modal") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("title") {
                    title = Some(meta.value()?.parse::<syn::LitStr>()?.value());
                    Ok(())
                } else {
                    Err(meta.error("unknown `modal` attribute (expected `title`)"))
                }
            })?;
        }
    }
    let title = title.ok_or_else(|| {
        syn::Error::new_spanned(struct_ident, "missing `#[modal(title = \"...\")]`")
    })?;

    // ── fields ─────────────────────────────────────────────────────────────────
    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(named) => &named.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    struct_ident,
                    "Cv2Modal requires a struct with named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                struct_ident,
                "Cv2Modal can only be derived for structs",
            ))
        }
    };

    let mut specs = Vec::new();
    for field in fields {
        specs.push(parse_field(field)?);
    }

    // ── into_modal body: one Action-Row text input per field ───────────────────
    let row_exprs = specs.iter().map(|f| {
        let id = &f.custom_id;
        let label = &f.label;
        let style = if f.paragraph {
            quote! { crate::ui::TextInputStyle::Paragraph }
        } else {
            quote! { crate::ui::TextInputStyle::Short }
        };
        let required = f.required;
        let ident = &f.ident;
        let value = if f.is_option {
            quote! { .value(self.#ident.clone()) }
        } else {
            quote! { .value(Some(self.#ident.clone())) }
        };
        let placeholder = match &f.placeholder {
            Some(p) => quote! { .placeholder(#p) },
            None => quote! {},
        };
        let min = match f.min_length {
            Some(n) => quote! { .min_length(#n) },
            None => quote! {},
        };
        let max = match f.max_length {
            Some(n) => quote! { .max_length(#n) },
            None => quote! {},
        };
        quote! {
            crate::ui::action_row(vec![
                crate::ui::TextInput::new(#id, #style)
                    .label(#label)
                    .required(#required)
                    #placeholder
                    #min
                    #max
                    #value
                    .into()
            ])
        }
    });

    // ── from_submission body: read each field back by custom_id ────────────────
    let read_exprs = specs.iter().map(|f| {
        let ident = &f.ident;
        let id = &f.custom_id;
        if f.is_option {
            quote! {
                #ident: crate::ui::read_text(__components, #id)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(::std::string::ToString::to_string)
            }
        } else {
            quote! {
                #ident: crate::ui::read_text(__components, #id)
                    .unwrap_or("")
                    .trim()
                    .to_string()
            }
        }
    });

    Ok(quote! {
        impl #struct_ident {
            /// The modal title from `#[modal(title = ...)]`.
            pub const MODAL_TITLE: &'static str = #title;

            /// Render this (DB-populated) instance as a prefilled modal payload.
            pub fn into_modal(
                &self,
                custom_id: impl ::std::convert::Into<::std::string::String>,
            ) -> crate::ui::Modal {
                let mut __modal = crate::ui::Modal::new(custom_id, Self::MODAL_TITLE);
                #( __modal = __modal.component(#row_exprs); )*
                __modal
            }

            /// Parse a submitted modal interaction back into this struct.
            pub fn from_submission(
                mi: &::poise::serenity_prelude::ModalInteraction,
            ) -> ::anyhow::Result<Self> {
                Self::from_components(&mi.data.components)
            }

            /// Core parse: read field values out of submitted Action-Row components.
            pub fn from_components(
                __components: &[::poise::serenity_prelude::ActionRow],
            ) -> ::anyhow::Result<Self> {
                Ok(Self {
                    #( #read_exprs, )*
                })
            }
        }
    })
}

fn parse_field(field: &syn::Field) -> syn::Result<FieldSpec> {
    let ident = field
        .ident
        .clone()
        .ok_or_else(|| syn::Error::new_spanned(field, "field must be named"))?;

    let is_option = type_is_option(&field.ty);

    let mut label: Option<String> = None;
    let mut placeholder: Option<String> = None;
    let mut paragraph = false;
    let mut required: Option<bool> = None;
    let mut custom_id: Option<String> = None;
    let mut min_length: Option<u16> = None;
    let mut max_length: Option<u16> = None;

    for attr in &field.attrs {
        if !attr.path().is_ident("field") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("label") {
                label = Some(meta.value()?.parse::<syn::LitStr>()?.value());
            } else if meta.path.is_ident("placeholder") {
                placeholder = Some(meta.value()?.parse::<syn::LitStr>()?.value());
            } else if meta.path.is_ident("id") {
                custom_id = Some(meta.value()?.parse::<syn::LitStr>()?.value());
            } else if meta.path.is_ident("required") {
                required = Some(meta.value()?.parse::<syn::LitBool>()?.value());
            } else if meta.path.is_ident("style") {
                let v = meta.value()?;
                let s = if v.peek(syn::LitStr) {
                    v.parse::<syn::LitStr>()?.value()
                } else {
                    v.parse::<syn::Ident>()?.to_string()
                };
                paragraph = match s.as_str() {
                    "paragraph" => true,
                    "short" => false,
                    other => {
                        return Err(meta.error(format!(
                            "unknown style `{other}` (expected `short` or `paragraph`)"
                        )))
                    }
                };
            } else if meta.path.is_ident("min_length") {
                min_length = Some(meta.value()?.parse::<syn::LitInt>()?.base10_parse()?);
            } else if meta.path.is_ident("max_length") {
                max_length = Some(meta.value()?.parse::<syn::LitInt>()?.base10_parse()?);
            } else {
                return Err(meta.error("unknown `field` attribute"));
            }
            Ok(())
        })?;
    }

    let label = label.ok_or_else(|| {
        syn::Error::new_spanned(field, "missing `#[field(label = \"...\")]`")
    })?;

    Ok(FieldSpec {
        custom_id: custom_id.unwrap_or_else(|| ident.to_string()),
        // Optional fields default to not-required; the attr overrides either way.
        required: required.unwrap_or(!is_option),
        ident,
        label,
        placeholder,
        paragraph,
        min_length,
        max_length,
        is_option,
    })
}

/// True if `ty` is exactly `Option<...>`.
fn type_is_option(ty: &syn::Type) -> bool {
    if let syn::Type::Path(tp) = ty {
        if tp.qself.is_none() {
            if let Some(seg) = tp.path.segments.last() {
                return seg.ident == "Option";
            }
        }
    }
    false
}
