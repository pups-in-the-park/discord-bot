//! Moderation: warn/timeout/kick/ban/unban/history via slash commands and
//! context menus. The context menus open reason modals (handled by `router`);
//! [`service::send_action_dm`] is shared with the reports "take action" flow.

mod ban_cache;
pub mod commands;
pub mod context_menus;
pub mod modals;
pub mod router;
pub mod service;
pub mod view;

pub use ban_cache::BanCache;
pub use commands::{ban, history, kick, timeout, unban, untimeout, warn};
pub use context_menus::{ban_user, delete_and_warn, kick_user, timeout_user, view_history, warn_user};
