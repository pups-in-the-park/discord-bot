//! Shared blocklist presentation: scope labels and the "you're blocked" notice.

use crate::db::BlocklistEntry;

/// The valid top-level (non-ticket-category) blocklist scopes.
pub const FEATURE_SCOPES: [&str; 4] = ["tickets", "reports", "concerns", "appeals"];

/// A human-readable feature name for a scope, used in user-facing copy.
pub fn scope_feature_label(scope: &str) -> String {
    match scope {
        "tickets" => "tickets".to_string(),
        "reports" => "reports".to_string(),
        "concerns" => "concerns".to_string(),
        "appeals" => "appeals".to_string(),
        s => match s.strip_prefix("ticket:") {
            Some(name) => format!("**{name}** tickets"),
            None => s.to_string(),
        },
    }
}

/// SQLite `datetime` string ("YYYY-MM-DD HH:MM:SS", UTC) -> unix timestamp.
pub fn sqlite_ts(s: &str) -> Option<i64> {
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .ok()
        .map(|d| d.and_utc().timestamp())
}

/// The message shown to a user when a block stops them using a feature.
pub fn blocked_text(entry: &BlocklistEntry) -> String {
    let feature = scope_feature_label(&entry.scope);
    let expiry = match entry.expires_at.as_deref().and_then(sqlite_ts) {
        Some(ts) => format!(" This block lifts <t:{ts}:R>."),
        None => " This block does not expire.".to_string(),
    };
    format!(
        "🚫 You're currently blocked from {feature}.\n**Reason:** {}{}",
        entry.reason, expiry
    )
}
