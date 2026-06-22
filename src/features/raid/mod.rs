//! Raid protection: [`service`] holds the join-scoring and per-channel slowmode
//! state (`RaidState`, shared in `BotData`) plus the sensitivity presets. The
//! member-join/message detection runs in the events layer; this feature owns the
//! "Clear Raid Mode" button. The alert card and channel-slowmode side effects
//! stay in `events` (they use its private log-channel resolution).

pub mod components;
pub mod router;
pub mod service;

pub use service::{sensitivity_key, RaidState, SENSITIVITY_HIGH, SENSITIVITY_LOW, SENSITIVITY_MEDIUM};
