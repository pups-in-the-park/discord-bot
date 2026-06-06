//! `/blocklist` group: bar users from tickets, reports, concerns, or appeals —
//! globally per feature or per ticket category, with an expiry and a clear
//! user-facing reason.

pub mod commands;
pub mod view;

pub use commands::blocklist;
