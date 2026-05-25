//! Feature-first source tree. Each feature owns its commands, component/modal
//! handlers, message builders (`view`), non-UI logic (`service`), and a `router`
//! that claims the custom-id prefixes it handles. `handlers::dispatch` walks the
//! routers; unmigrated features still fall through to the legacy monoliths.

pub mod appeals;
pub mod blocklist;
pub mod concerns;
pub mod general;
pub mod moderation;
pub mod raid;
pub mod reports;
pub mod roles;
pub mod setup;
pub mod tickets;
