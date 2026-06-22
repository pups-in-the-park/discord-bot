//! Anti-raid detection (exponential decay scoring) and per-channel auto-slowmode (token bucket).

use poise::serenity_prelude as serenity;
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::Mutex;

// ── Sensitivity presets ───────────────────────────────────────────────────────

pub struct SensitivityPreset {
    pub decay_rate: f64,
    pub threshold: f64,
}

pub const SENSITIVITY_LOW: SensitivityPreset    = SensitivityPreset { decay_rate: 0.05, threshold: 8.0 };
pub const SENSITIVITY_MEDIUM: SensitivityPreset = SensitivityPreset { decay_rate: 0.15, threshold: 5.0 };
pub const SENSITIVITY_HIGH: SensitivityPreset   = SensitivityPreset { decay_rate: 0.30, threshold: 3.0 };

/// The sensitivity key ("low"/"medium"/"high") whose preset threshold is nearest
/// to a stored `join_threshold`. Single source for displaying a config's
/// sensitivity — keeps UI labels in sync if the presets above are ever tuned.
pub fn sensitivity_key(join_threshold: f64) -> &'static str {
    let presets = [
        ("low", SENSITIVITY_LOW.threshold),
        ("medium", SENSITIVITY_MEDIUM.threshold),
        ("high", SENSITIVITY_HIGH.threshold),
    ];
    presets
        .iter()
        .min_by(|a, b| {
            (join_threshold - a.1)
                .abs()
                .total_cmp(&(join_threshold - b.1).abs())
        })
        .map(|(k, _)| *k)
        .unwrap_or("medium")
}

// ── Per-guild join scoring ─────────────────────────────────────────────────────

#[derive(Default)]
struct GuildRaidState {
    score: f64,
    last_join: Option<Instant>,
}

// ── Per-channel token bucket ──────────────────────────────────────────────────

struct ChannelPressure {
    tokens: f64,
    last_update: Instant,
    capacity: f64,
    refill_rate: f64,
    current_slowmode_secs: u16,
    last_edit: Option<Instant>,
}

impl ChannelPressure {
    fn new(capacity: f64, window_secs: f64) -> Self {
        Self {
            tokens: capacity,
            last_update: Instant::now(),
            capacity,
            refill_rate: capacity / window_secs,
            current_slowmode_secs: 0,
            last_edit: None,
        }
    }

    /// Record a message. Returns the new pressure ratio (0.0–1.0).
    fn record_message(&mut self) -> f64 {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        self.last_update = now;

        self.tokens = (self.tokens + self.refill_rate * elapsed).min(self.capacity);
        self.tokens = (self.tokens - 1.0).max(0.0);

        (self.capacity - self.tokens) / self.capacity
    }

    /// Returns the target slowmode seconds for the current pressure ratio.
    fn target_slowmode(pressure: f64) -> u16 {
        if pressure < 0.5 { 0 }
        else if pressure < 0.75 { 5 }
        else if pressure < 0.90 { 15 }
        else { 30 }
    }

    /// Returns Some(new_slowmode_secs) if the slowmode tier changed and enough
    /// time has passed since the last edit (5s cooldown).
    fn slowmode_update_needed(&mut self, pressure: f64) -> Option<u16> {
        let target = Self::target_slowmode(pressure);
        if target == self.current_slowmode_secs {
            return None;
        }
        let now = Instant::now();
        if let Some(last) = self.last_edit {
            if now.duration_since(last).as_secs() < 5 {
                return None;
            }
        }
        self.current_slowmode_secs = target;
        self.last_edit = Some(now);
        Some(target)
    }
}

// ── RaidState (shared in BotData) ─────────────────────────────────────────────

pub struct RaidState {
    guilds: Mutex<HashMap<serenity::GuildId, GuildRaidState>>,
    channels: Mutex<HashMap<serenity::ChannelId, ChannelPressure>>,
}

impl RaidState {
    pub fn new() -> Self {
        Self {
            guilds: Mutex::new(HashMap::new()),
            channels: Mutex::new(HashMap::new()),
        }
    }

    /// Record a member join event. Returns `(new_score, triggered)`.
    /// `triggered` is true if the score crossed the threshold this join.
    pub async fn record_join(
        &self,
        guild_id: serenity::GuildId,
        account_age_days: f64,
        decay_rate: f64,
        threshold: f64,
        was_active: bool,
    ) -> (f64, bool) {
        let mut guilds = self.guilds.lock().await;
        let state = guilds.entry(guild_id).or_default();

        let now = Instant::now();
        let elapsed = state
            .last_join
            .map(|t| now.duration_since(t).as_secs_f64())
            .unwrap_or(0.0);
        state.last_join = Some(now);

        let decayed = state.score * (-decay_rate * elapsed).exp();
        let weight = if account_age_days < 1.0 { 2.0 }
                     else if account_age_days < 7.0 { 1.5 }
                     else { 1.0 };
        state.score = decayed + weight;

        let triggered = !was_active && state.score >= threshold;
        (state.score, triggered)
    }

    /// Decay the current score and check if auto-recovery applies.
    /// Returns `(score, should_clear)` — should_clear if score drops below 20% of threshold.
    pub async fn check_decay(
        &self,
        guild_id: serenity::GuildId,
        decay_rate: f64,
        threshold: f64,
    ) -> (f64, bool) {
        let mut guilds = self.guilds.lock().await;
        let state = guilds.entry(guild_id).or_default();

        let now = Instant::now();
        let elapsed = state
            .last_join
            .map(|t| now.duration_since(t).as_secs_f64())
            .unwrap_or(0.0);

        state.score *= (-decay_rate * elapsed).exp();
        state.last_join = Some(now);

        let should_clear = state.score < threshold * 0.2;
        (state.score, should_clear)
    }

    /// Reset the raid score for a guild (manual clear).
    pub async fn reset(&self, guild_id: serenity::GuildId) {
        let mut guilds = self.guilds.lock().await;
        if let Some(state) = guilds.get_mut(&guild_id) {
            state.score = 0.0;
        }
    }

    /// Record a message in a channel and return the target slowmode secs if it
    /// changed tier (None if no update needed). `capacity` and `window_secs` come
    /// from the guild's slowmode_config.
    pub async fn record_message(
        &self,
        channel_id: serenity::ChannelId,
        capacity: f64,
        window_secs: f64,
    ) -> Option<u16> {
        let mut channels = self.channels.lock().await;
        let pressure = channels
            .entry(channel_id)
            .or_insert_with(|| ChannelPressure::new(capacity, window_secs));

        let ratio = pressure.record_message();
        pressure.slowmode_update_needed(ratio)
    }
}
