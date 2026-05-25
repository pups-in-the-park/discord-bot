//! General-purpose commands with no shared state: `/ping`, `/help`.

pub mod commands;

pub use commands::{help, ping};
