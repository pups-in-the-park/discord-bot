//! `/setup`: each subcommand opens an ephemeral CV2 settings form. Selects and
//! toggles save immediately and re-render the form in place; numeric fields open a
//! small modal. The `build_setup_*_form` builders live in [`view`].

pub mod commands;
pub mod components;
pub mod modals;
pub mod router;
pub mod view;

pub use commands::setup;
