#![allow(dead_code)]

use std::sync::Arc;
use crate::config::AppConfig;
use crate::db::Database;
use crate::features::raid::RaidState;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum BotError {
    #[error("{0}")]
    User(String),
    #[error("Database error: {0}")]
    Db(#[from] anyhow::Error),
    #[error("Discord error: {0}")]
    Serenity(#[from] serenity::Error),
}

impl BotError {
    pub fn user(msg: impl Into<String>) -> Self { BotError::User(msg.into()) }
}

// ── Shared bot state ──────────────────────────────────────────────────────────

pub struct BotData {
    pub db: Database,
    pub config: AppConfig,
    pub raid: RaidState,
}

impl BotData {
    pub fn new(db: Database, config: AppConfig) -> Self {
        Self { db, config, raid: RaidState::new() }
    }
}

pub type Context<'a> = poise::Context<'a, Arc<BotData>, BotError>;
pub type Error = BotError;

// ── Colour constants ──────────────────────────────────────────────────────────

pub mod colours {
    use poise::serenity_prelude::Colour;
    pub const BLURPLE:  Colour = Colour(0x5865F2);
    pub const GREEN:    Colour = Colour(0x57F287);
    pub const RED:      Colour = Colour(0xED4245);
    pub const YELLOW:   Colour = Colour(0xFEE75C);
    pub const GREY:     Colour = Colour(0x99AAB5);
    pub const ORANGE:   Colour = Colour(0xE67E22);
    pub const DARK_RED: Colour = Colour(0xC0392B);

    pub fn from_hex(hex: &str) -> Colour {
        let hex = hex.trim_start_matches('#');
        let val = u32::from_str_radix(hex, 16).unwrap_or(0x5865F2);
        Colour(val)
    }
}

// ── Priority ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Priority { Low, Normal, High, Urgent }

impl Priority {
    pub fn from_str(s: &str) -> Self {
        match s { "low" => Self::Low, "high" => Self::High, "urgent" => Self::Urgent, _ => Self::Normal }
    }
    pub fn as_str(&self) -> &'static str {
        match self { Self::Low => "low", Self::Normal => "normal", Self::High => "high", Self::Urgent => "urgent" }
    }
    pub fn emoji(&self) -> &'static str {
        match self { Self::Low => "🟢", Self::Normal => "🔵", Self::High => "🟠", Self::Urgent => "🔴" }
    }
    pub fn label(&self) -> &'static str {
        match self { Self::Low => "Low", Self::Normal => "Normal", Self::High => "High", Self::Urgent => "Urgent" }
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ── Custom ID vocabulary ──────────────────────────────────────────────────────
// Builders + parsers now live in `crate::ids`. Re-exported here so existing
// `crate::context::cid_*` / `CID_*` call sites keep resolving during the
// feature-first migration; new code should reference `crate::ids` directly.
pub use crate::ids::*;
