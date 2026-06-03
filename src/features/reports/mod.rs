//! Reports: members flag a user or message; staff investigate in a private thread
//! and either dismiss or take a moderation action. Entry points are the `/report`
//! command and the "Report Message"/"Report User" context menus; the rest is
//! component/modal driven and routed by [`router`].

pub mod commands;
pub mod components;
pub mod context_menus;
pub mod modals;
pub mod router;
pub mod view;

pub use commands::report;
pub use context_menus::{report_message, report_user};

/// A moderator must not act on a report filed against themselves — another mod
/// handles it. Returns true (and the acting user should be refused) when the
/// acting user is the report's target.
pub(crate) fn acting_on_own_report(
    acting_user: poise::serenity_prelude::UserId,
    report: &crate::db::Report,
) -> bool {
    acting_user.to_string() == report.target_user_id
}

/// Message shown when a moderator tries to act on a report about themselves.
pub(crate) const SELF_ACTION_REFUSAL: &str =
    "You can't act on a report filed against you — another moderator will handle this.";
