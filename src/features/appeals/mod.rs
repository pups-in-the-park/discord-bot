//! Appeals: a moderation-action DM carries an "Appeal" button; submitting opens a
//! private appeal thread and posts a card to `#appeals`. Staff resolve the appeal
//! with the Accept/Deny buttons on that card (or the thread intro), which open a
//! response modal and DM the appellant.

pub mod components;
pub mod modals;
pub mod router;
pub mod service;
pub mod view;
