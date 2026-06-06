//! The typed kit must serialize to exactly the JSON shape Discord expects (these
//! literals are the contract). Comparison is key-order-independent via
//! `serde_json::Value`; if a typed builder drifts from Discord's shape, a test fails.

use super::*;

fn to_value<T: serde::Serialize>(t: &T) -> serde_json::Value {
    serde_json::to_value(t).unwrap()
}

#[test]
fn container_with_text_separator_and_button_emits_expected_json() {
    let typed = Container::new(vec![
        text("**Header**"),
        separator(false, Spacing::Small),
        action_row(vec![Button::new("t:close", "Close", ButtonStyle::Danger)
            .emoji("🔒")
            .into()]),
    ])
    .accent(0x5865F2);

    let expected = serde_json::json!({
        "type": 17,
        "accent_color": 0x5865F2,
        "components": [
            { "type": 10, "content": "**Header**" },
            { "type": 14, "divider": false, "spacing": 1 },
            { "type": 1, "components": [
                { "type": 2, "custom_id": "t:close", "label": "Close", "style": 4, "emoji": { "name": "🔒" } }
            ]},
        ],
    });

    assert_eq!(to_value(&Component::from(typed)), expected);
}

#[test]
fn channel_select_emits_expected_json() {
    let typed = ChannelSelect::new(
        "setup:log:fallback",
        &[ChannelType::Text, ChannelType::Announcement, ChannelType::PrivateThread],
    )
    .placeholder("Select a channel…")
    .default(Some("123456789"));

    let expected = serde_json::json!({
        "type": 8,
        "custom_id": "setup:log:fallback",
        "placeholder": "Select a channel…",
        "min_values": 0,
        "max_values": 1,
        "channel_types": [0, 5, 12],
        "default_values": [{ "id": "123456789", "type": "channel" }],
    });

    assert_eq!(to_value(&Component::from(typed)), expected);
}

#[test]
fn role_select_emits_expected_json() {
    let typed = RoleSelect::new("setup:mod:staff")
        .placeholder("Select up to 10 staff roles…")
        .max_values(10)
        .defaults(["111".to_string(), "222".to_string()]);

    let expected = serde_json::json!({
        "type": 6,
        "custom_id": "setup:mod:staff",
        "placeholder": "Select up to 10 staff roles…",
        "min_values": 0,
        "max_values": 10,
        "default_values": [
            { "id": "111", "type": "role" },
            { "id": "222", "type": "role" },
        ],
    });

    assert_eq!(to_value(&Component::from(typed)), expected);
}

#[test]
fn string_select_emits_expected_json() {
    let typed = StringSelect::new(
        "setup:raid:sensitivity",
        vec![SelectOption::new("medium", "Medium").description("Balanced").default(true)],
    )
    .placeholder("Select sensitivity…");

    let expected = serde_json::json!({
        "type": 3,
        "custom_id": "setup:raid:sensitivity",
        "placeholder": "Select sensitivity…",
        "min_values": 1,
        "max_values": 1,
        "options": [
            { "label": "Medium", "value": "medium", "description": "Balanced", "default": true },
        ],
    });

    assert_eq!(to_value(&Component::from(typed)), expected);
}

// ── #[derive(Cv2Modal)] round-trip ──────────────────────────────────────────────

/// Mirrors the legacy `m:cat:basic` modal: DB-prefilled category fields. Proves the
/// derive emits defaults outbound and recovers edits inbound.
#[derive(Debug, PartialEq, pip_macros::Cv2Modal)]
#[modal(title = "🏷️ Edit Basic Info")]
struct CategoryBasicModal {
    #[field(label = "Button Label", placeholder = "e.g. General Support")]
    cat_label: String,
    #[field(label = "Emoji (optional)", placeholder = "e.g. 🎫")]
    cat_emoji: Option<String>,
    #[field(label = "Accent Color (hex)", placeholder = "e.g. 5865F2", required = false)]
    cat_color: String,
    #[field(label = "Description", style = paragraph)]
    cat_desc: Option<String>,
}

#[test]
fn cv2modal_into_modal_emits_db_defaults() {
    let populated = CategoryBasicModal {
        cat_label: "General Support".into(),
        cat_emoji: Some("🎫".into()),
        cat_color: "5865F2".into(),
        cat_desc: None,
    };

    let json = to_value(&populated.into_modal("m:cat:basic:42"));

    assert_eq!(json["custom_id"], "m:cat:basic:42");
    assert_eq!(json["title"], "🏷️ Edit Basic Info");

    let rows = json["components"].as_array().unwrap();
    // One Label (type 18) per field, each wrapping a text input (type 4).
    assert_eq!(rows.len(), 4);
    let input = |row: &serde_json::Value| row["component"].clone();

    assert_eq!(rows[0]["type"], 18); // Label
    assert_eq!(rows[0]["label"], "Button Label"); // label lives on the Label
    assert_eq!(input(&rows[0])["type"], 4);
    assert_eq!(input(&rows[0])["custom_id"], "cat_label");
    assert_eq!(input(&rows[0])["value"], "General Support"); // DB default
    assert_eq!(input(&rows[0])["required"], true); // String → required
    assert_eq!(input(&rows[0])["style"], 1); // short

    assert_eq!(input(&rows[1])["value"], "🎫");
    assert_eq!(input(&rows[1])["required"], false); // Option → optional, sent explicitly

    assert_eq!(input(&rows[2])["required"], false); // explicit required=false override, sent explicitly

    // None optional → no `value` key, paragraph style.
    assert!(input(&rows[3]).get("value").is_none());
    assert_eq!(input(&rows[3])["style"], 2);
}

#[test]
fn cv2modal_from_submission_round_trips_through_serenity() {
    use poise::serenity_prelude as serenity;

    // Exactly the JSON Discord sends back for a modal-v2 (`Label`-wrapped)
    // submission. `ModalComponent`'s deserializer borrows `&RawValue`, so this must
    // be parsed from a string (not a `serde_json::Value`).
    let submitted = r#"[
        { "type": 18, "component": { "type": 4, "custom_id": "cat_label", "value": "Billing Help" } },
        { "type": 18, "component": { "type": 4, "custom_id": "cat_emoji", "value": "💳" } },
        { "type": 18, "component": { "type": 4, "custom_id": "cat_color", "value": "" } },
        { "type": 18, "component": { "type": 4, "custom_id": "cat_desc", "value": "  payment questions  " } }
    ]"#;
    let components: Vec<serenity::ModalComponent> = serde_json::from_str(submitted).unwrap();

    let parsed = CategoryBasicModal::from_components(&components).unwrap();

    assert_eq!(
        parsed,
        CategoryBasicModal {
            cat_label: "Billing Help".into(),
            cat_emoji: Some("💳".into()),
            cat_color: String::new(),       // empty submitted, required=false → ""
            cat_desc: Some("payment questions".into()), // trimmed
        }
    );
}
