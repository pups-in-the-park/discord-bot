//! A tiny time-boxed cache of a guild's ban list. The `/unban` autocomplete
//! fires once per keystroke and each miss fetches up to 1000 bans, so without
//! this a single search re-downloads the whole list several times over.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
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
    /// Bumped on every `invalidate`; `put` refuses to store if it changed since
    /// the fetch started, so an in-flight fetch can't resurrect a stale list.
    generation: AtomicU64,
}

impl BanCache {
    /// The cached ban list for `guild`, if one was stored within the TTL.
    pub fn get(&self, guild: serenity::GuildId) -> Option<CachedBans> {
        let map = self.guilds.lock().unwrap();
        map.get(&guild)
            .and_then(|(at, bans)| (at.elapsed() < TTL).then(|| Arc::clone(bans)))
    }

    /// Read this before starting a fetch, pass it to [`put`](Self::put).
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    /// Drop the cached list for `guild` — call after any ban/unban so
    /// autocomplete doesn't serve a stale list for the rest of the TTL.
    pub fn invalidate(&self, guild: serenity::GuildId) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        self.guilds.lock().unwrap().remove(&guild);
    }

    /// Store a fetched ban list and hand back the shared copy. If anything
    /// invalidated since `observed_generation`, the list is returned but not
    /// cached — it may predate the ban/unban.
    pub fn put(
        &self,
        guild: serenity::GuildId,
        bans: Vec<serenity::Ban>,
        observed_generation: u64,
    ) -> CachedBans {
        let bans = Arc::new(bans);
        if self.generation.load(Ordering::Acquire) == observed_generation {
            self.guilds
                .lock()
                .unwrap()
                .insert(guild, (Instant::now(), Arc::clone(&bans)));
        }
        bans
    }
}
