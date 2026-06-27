//! A tiny time-boxed cache of a guild's ban list. The `/unban` autocomplete
//! fires once per keystroke and each miss fetches up to 1000 bans, so without
//! this a single search re-downloads the whole list several times over.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use poise::serenity_prelude as serenity;

/// How long a fetched ban list stays fresh: long enough to cover a burst of
/// autocomplete keystrokes, short enough that a recent unban drops out quickly.
const TTL: Duration = Duration::from_secs(15);

/// A shared, immutable snapshot of a guild's ban list.
pub type CachedBans = Arc<Vec<serenity::Ban>>;

#[derive(Default)]
pub struct BanCache {
    guilds: Mutex<HashMap<serenity::GuildId, (Instant, CachedBans)>>,
}

impl BanCache {
    /// The cached ban list for `guild`, if one was stored within the TTL.
    pub fn get(&self, guild: serenity::GuildId) -> Option<CachedBans> {
        let map = self.guilds.lock().unwrap();
        map.get(&guild)
            .and_then(|(at, bans)| (at.elapsed() < TTL).then(|| Arc::clone(bans)))
    }

    /// Store a freshly fetched ban list for `guild` and hand back the shared copy.
    pub fn put(&self, guild: serenity::GuildId, bans: Vec<serenity::Ban>) -> CachedBans {
        let bans = Arc::new(bans);
        self.guilds
            .lock()
            .unwrap()
            .insert(guild, (Instant::now(), Arc::clone(&bans)));
        bans
    }
}
