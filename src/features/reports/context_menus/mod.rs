//! Context-menu entry points into reporting. They live with the reports feature
//! (not in a shared `context_menu.rs`) so the whole flow is in one place.

mod report_message;
mod report_user;

pub use report_message::report_message;
pub use report_user::report_user;
