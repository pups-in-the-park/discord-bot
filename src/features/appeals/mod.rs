//! Appeals: a moderation-action DM carries an "Appeal" button; submitting opens a
//! private appeal thread and posts a card to `#appeals`. Staff run `/appeal
//! accept|deny` inside the thread to resolve it and DM the appellant.

pub mod commands;
pub mod components;
pub mod modals;
pub mod router;
pub mod view;

pub use commands::appeal;
