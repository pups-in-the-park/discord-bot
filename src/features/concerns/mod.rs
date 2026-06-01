//! Concerns: when an appeal is denied or a report is closed with no action, the
//! affected user gets a "Report a Concern" button. Submitting posts a card to the
//! admin-only `#concerns` channel, which staff mark as reviewed. No slash command.

pub mod components;
pub mod modals;
pub mod router;
pub mod view;
