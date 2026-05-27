//! Tickets: the `/ticket` command group (close/claim/priority/tag/transfer/…, plus
//! the `panel` and `category` sub-groups), the context menus for opening tickets
//! on a user's behalf, and the component/modal flows for panels and category
//! configuration. `service` holds thread open/close logic; `view` holds the panel,
//! category-config form, and intake-modal builders.

pub mod commands;
pub mod components;
pub mod context_menus;
pub mod modals;
pub mod router;
pub mod service;
pub mod view;

pub use commands::ticket;
pub use context_menus::{open_ticket_from_message, open_ticket_with_user};
