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
