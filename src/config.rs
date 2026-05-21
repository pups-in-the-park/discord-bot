use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Development,
    Production,
}

impl Environment {
    pub fn is_dev(&self) -> bool {
        *self == Environment::Development
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BotConfig {
    pub token: String,
    pub environment: Environment,
    pub log_level: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GuildConfig {
    pub id: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub bot: BotConfig,
    pub database: DatabaseConfig,
    pub guild: GuildConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let text = std::fs::read_to_string("config.toml")
            .context("Failed to read config.toml — copy config.example.toml and fill it in")?;
        toml::from_str(&text).context("Failed to parse config.toml")
    }

    pub fn db_url(&self) -> String {
        format!("sqlite://{}", self.database.path)
    }
}
