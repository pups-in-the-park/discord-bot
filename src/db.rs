#![allow(dead_code)]
// Several row constructors mirror their table's columns one-to-one, which trips
// the arg-count lint without a cleaner alternative than a params struct per row.
#![allow(clippy::too_many_arguments)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool};
use sqlx::Row;
use std::str::FromStr;

// ── Models ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GuildConfig {
    pub guild_id: String,
    pub log_channel_id: Option<String>,
    pub mod_log_channel_id: Option<String>,
    pub chat_log_channel_id: Option<String>,
    pub ticket_log_channel_id: Option<String>,
    pub reports_channel_id: Option<String>,
    pub appeals_channel_id: Option<String>,
    pub concerns_channel_id: Option<String>,
    pub report_type_id: Option<i64>,
    pub ticket_count: i64,
}

#[derive(Debug, Clone)]
pub struct TicketType {
    pub id: i64,
    pub guild_id: String,
    pub name: String,
    pub label: String,
    pub emoji: Option<String>,
    pub description: Option<String>,
    pub color: String,
    pub thread_name_pattern: String,
    pub max_open_per_user: i64,
    pub auto_close_hours: Option<i64>,
    pub welcome_message: Option<String>,
    pub staff_alert_channel_id: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct FormField {
    pub id: i64,
    pub ticket_type_id: i64,
    pub label: String,
    pub placeholder: Option<String>,
    /// Question type: `short`, `paragraph`, `select` (dropdown), or `checkbox`.
    pub style: String,
    pub required: bool,
    pub min_length: Option<i64>,
    pub max_length: Option<i64>,
    pub position: i64,
    /// Choices for `select`/`checkbox` fields, stored as a JSON array of strings.
    pub options: Option<String>,
}

impl FormField {
    /// Parse the `options` JSON into a vec (empty for text fields or unset).
    pub fn options_vec(&self) -> Vec<String> {
        self.options
            .as_ref()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default()
    }

    /// Whether this field type requires options to be usable.
    pub fn needs_options(&self) -> bool {
        matches!(self.style.as_str(), "select" | "checkbox")
    }
}

#[derive(Debug, Clone)]
pub struct Panel {
    pub id: i64,
    pub guild_id: String,
    pub channel_id: Option<String>,
    pub message_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub color: String,
    pub image_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub footer_text: Option<String>,
    pub layout: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TicketStatus { Open, Closed }

impl From<String> for TicketStatus {
    fn from(s: String) -> Self {
        if s == "closed" { TicketStatus::Closed } else { TicketStatus::Open }
    }
}

impl std::fmt::Display for TicketStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { Self::Open => write!(f, "open"), Self::Closed => write!(f, "closed") }
    }
}

#[derive(Debug, Clone)]
pub struct Ticket {
    pub id: i64,
    pub ticket_number: i64,
    pub guild_id: String,
    pub ticket_type_id: Option<i64>,
    pub thread_id: String,
    pub parent_channel_id: String,
    pub owner_id: String,
    pub claimed_by: Option<String>,
    pub priority: String,
    pub status: TicketStatus,
    pub close_reason: Option<String>,
    pub closed_by: Option<String>,
    pub form_responses: Option<String>,
    pub reported_message_id: Option<String>,
    pub reported_message_url: Option<String>,
    pub reported_message_content: Option<String>,
    pub reported_author_id: Option<String>,
    /// Message id of the in-thread CV2 ticket card, so claim/priority changes can
    /// re-render it. `None` for tickets opened before the column existed (or if
    /// posting the card failed).
    pub card_message_id: Option<String>,
    pub created_at: String,
    pub closed_at: Option<String>,
    pub last_activity_at: String,
}

#[derive(Debug, Clone)]
pub struct TicketNote {
    pub id: i64,
    pub ticket_id: i64,
    pub author_id: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketMessage {
    pub id: i64,
    pub ticket_id: i64,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub attachments: Option<String>,
    pub sent_at: String,
}

#[derive(Debug, Clone)]
pub struct BlocklistEntry {
    pub guild_id: String,
    pub user_id: String,
    pub scope: String,
    pub reason: String,
    pub added_by: String,
    pub added_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TicketTagDefinition {
    pub id: i64,
    pub guild_id: String,
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ModConfig {
    pub guild_id: String,
    pub staff_role_ids: Option<String>,
    pub dm_on_warn: bool,
    pub dm_on_timeout: bool,
    pub dm_on_kick: bool,
    pub dm_on_ban: bool,
    pub appeal_cooldown_days: i64,
}

impl ModConfig {
    pub fn staff_roles(&self) -> Vec<String> {
        self.staff_role_ids
            .as_ref()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub struct Infraction {
    pub id: i64,
    pub guild_id: String,
    pub user_id: String,
    pub moderator_id: String,
    pub kind: String,
    pub reason: String,
    pub duration_secs: Option<i64>,
    pub active: bool,
    pub appealable: bool,
    pub created_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Appeal {
    pub id: i64,
    pub guild_id: String,
    pub infraction_id: i64,
    pub user_id: String,
    pub thread_id: Option<String>,
    pub card_message_id: Option<String>,
    pub reason: String,
    pub status: String,
    pub response: Option<String>,
    pub responded_by: Option<String>,
    pub created_at: String,
    pub responded_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Report {
    pub id: i64,
    pub guild_id: String,
    pub reporter_id: String,
    pub target_user_id: String,
    pub message_id: Option<String>,
    pub message_url: Option<String>,
    pub message_content: Option<String>,
    pub reason: Option<String>,
    pub card_message_id: Option<String>,
    pub thread_id: Option<String>,
    pub status: String,
    pub resolved_by: Option<String>,
    pub concern_raised: bool,
    pub created_at: String,
    pub profile_parts: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Concern {
    pub id: i64,
    pub guild_id: String,
    pub user_id: String,
    pub kind: String,
    pub source_id: i64,
    pub reason: String,
    pub status: String,
    pub reviewed_by: Option<String>,
    pub created_at: String,
    pub target_moderator_id: Option<String>,
    pub reviewed_at: Option<String>,
}

/// Per-moderator concern tally for `/concerns stats`.
#[derive(Debug, Clone)]
pub struct ConcernStat {
    pub moderator_id: Option<String>,
    pub total: i64,
    pub denied_appeals: i64,
    pub dismissed_reports: i64,
}

#[derive(Debug, Clone)]
pub struct RaidConfig {
    pub guild_id: String,
    pub join_threshold: f64,
    pub decay_rate: f64,
    pub slowmode_secs: i64,
    pub raid_active: bool,
    pub raid_triggered_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SlowmodeConfig {
    pub guild_id: String,
    pub enabled: bool,
    pub window_secs: i64,
    pub capacity: i64,
    pub excluded_channels: Option<String>,
}

impl SlowmodeConfig {
    pub fn excluded_channel_ids(&self) -> Vec<String> {
        self.excluded_channels
            .as_ref()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default()
    }
}

// ── Database ──────────────────────────────────────────────────────────────────

pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    pub async fn new(url: &str) -> Result<Self> {
        // Connection-level settings (not schema): WAL for concurrency, foreign keys
        // on for every connection. These belong here, not in a migration — SQLite
        // can't switch journal mode inside the transaction sqlx wraps migrations in.
        let options = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);
        let pool = SqlitePool::connect_with(options).await?;
        Ok(Self { pool })
    }

    /// Apply any pending migrations from `./migrations` (embedded at compile time).
    /// sqlx tracks applied versions in `_sqlx_migrations`, so this auto-resolves
    /// what still needs to run and is safe to call on every startup.
    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        tracing::info!("Migrations applied");
        Ok(())
    }

    // ── Guild config ──────────────────────────────────────────────────────────

    pub async fn get_or_create_guild(&self, guild_id: &str) -> Result<GuildConfig> {
        sqlx::query("INSERT OR IGNORE INTO guild_config (guild_id) VALUES (?)")
            .bind(guild_id)
            .execute(&self.pool)
            .await?;

        let row = sqlx::query(
            "SELECT guild_id,log_channel_id,mod_log_channel_id,chat_log_channel_id,
             ticket_log_channel_id,reports_channel_id,appeals_channel_id,
             concerns_channel_id,report_type_id,ticket_count
             FROM guild_config WHERE guild_id=?",
        )
        .bind(guild_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(GuildConfig {
            guild_id: row.get("guild_id"),
            log_channel_id: row.get("log_channel_id"),
            mod_log_channel_id: row.get("mod_log_channel_id"),
            chat_log_channel_id: row.get("chat_log_channel_id"),
            ticket_log_channel_id: row.get("ticket_log_channel_id"),
            reports_channel_id: row.get("reports_channel_id"),
            appeals_channel_id: row.get("appeals_channel_id"),
            concerns_channel_id: row.get("concerns_channel_id"),
            report_type_id: row.get("report_type_id"),
            ticket_count: row.get("ticket_count"),
        })
    }

    pub async fn update_guild_channels(
        &self,
        guild_id: &str,
        log: Option<&str>,
        mod_log: Option<&str>,
        chat_log: Option<&str>,
        ticket_log: Option<&str>,
    ) -> Result<()> {
        self.get_or_create_guild(guild_id).await?;
        sqlx::query(
            "UPDATE guild_config SET
             log_channel_id = COALESCE(?, log_channel_id),
             mod_log_channel_id = COALESCE(?, mod_log_channel_id),
             chat_log_channel_id = COALESCE(?, chat_log_channel_id),
             ticket_log_channel_id = COALESCE(?, ticket_log_channel_id)
             WHERE guild_id=?",
        )
        .bind(log)
        .bind(mod_log)
        .bind(chat_log)
        .bind(ticket_log)
        .bind(guild_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_reports_channel(&self, guild_id: &str, channel_id: Option<&str>) -> Result<()> {
        self.get_or_create_guild(guild_id).await?;
        sqlx::query("UPDATE guild_config SET reports_channel_id=? WHERE guild_id=?")
            .bind(channel_id)
            .bind(guild_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_appeals_channel(&self, guild_id: &str, channel_id: Option<&str>) -> Result<()> {
        self.get_or_create_guild(guild_id).await?;
        sqlx::query("UPDATE guild_config SET appeals_channel_id=? WHERE guild_id=?")
            .bind(channel_id)
            .bind(guild_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_concerns_channel(&self, guild_id: &str, channel_id: Option<&str>) -> Result<()> {
        self.get_or_create_guild(guild_id).await?;
        sqlx::query("UPDATE guild_config SET concerns_channel_id=? WHERE guild_id=?")
            .bind(channel_id)
            .bind(guild_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_log_fallback_channel(&self, guild_id: &str, channel_id: Option<&str>) -> Result<()> {
        self.get_or_create_guild(guild_id).await?;
        sqlx::query("UPDATE guild_config SET log_channel_id=? WHERE guild_id=?")
            .bind(channel_id).bind(guild_id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn set_mod_log_channel(&self, guild_id: &str, channel_id: Option<&str>) -> Result<()> {
        self.get_or_create_guild(guild_id).await?;
        sqlx::query("UPDATE guild_config SET mod_log_channel_id=? WHERE guild_id=?")
            .bind(channel_id).bind(guild_id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn set_chat_log_channel(&self, guild_id: &str, channel_id: Option<&str>) -> Result<()> {
        self.get_or_create_guild(guild_id).await?;
        sqlx::query("UPDATE guild_config SET chat_log_channel_id=? WHERE guild_id=?")
            .bind(channel_id).bind(guild_id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn set_ticket_log_channel(&self, guild_id: &str, channel_id: Option<&str>) -> Result<()> {
        self.get_or_create_guild(guild_id).await?;
        sqlx::query("UPDATE guild_config SET ticket_log_channel_id=? WHERE guild_id=?")
            .bind(channel_id).bind(guild_id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn set_report_type(&self, guild_id: &str, type_id: Option<i64>) -> Result<()> {
        self.get_or_create_guild(guild_id).await?;
        sqlx::query("UPDATE guild_config SET report_type_id=? WHERE guild_id=?")
            .bind(type_id)
            .bind(guild_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn next_ticket_number(&self, guild_id: &str) -> Result<i64> {
        self.get_or_create_guild(guild_id).await?;
        // Increment and read back in one atomic statement — a separate UPDATE then
        // SELECT lets two concurrent opens read the same counter and collide.
        let n: i64 = sqlx::query_scalar(
            "UPDATE guild_config SET ticket_count=ticket_count+1 WHERE guild_id=? RETURNING ticket_count",
        )
        .bind(guild_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(n)
    }

    // ── Ticket types ──────────────────────────────────────────────────────────

    fn map_ticket_type(r: &sqlx::sqlite::SqliteRow) -> TicketType {
        TicketType {
            id: r.get("id"),
            guild_id: r.get("guild_id"),
            name: r.get("name"),
            label: r.get("label"),
            emoji: r.get("emoji"),
            description: r.get("description"),
            color: r.get("color"),
            thread_name_pattern: r.get("thread_name_pattern"),
            max_open_per_user: r.get("max_open_per_user"),
            auto_close_hours: r.get("auto_close_hours"),
            welcome_message: r.get("welcome_message"),
            staff_alert_channel_id: r.get("staff_alert_channel_id"),
            active: r.get::<i64, _>("active") != 0,
        }
    }

    // The `has_form`, `ping_roles`, and `auto_add_staff` columns still exist in the
    // schema but are dormant: staff roles are always mentioned in new threads, and
    // the intake form is active exactly when the category has questions.
    const TICKET_TYPE_COLS: &'static str =
        "id,guild_id,name,label,emoji,description,color,thread_name_pattern,
         max_open_per_user,auto_close_hours,
         welcome_message,staff_alert_channel_id,active";

    pub async fn create_ticket_type(
        &self,
        guild_id: &str,
        name: &str,
        label: &str,
        emoji: Option<&str>,
        description: Option<&str>,
        color: &str,
        welcome_message: Option<&str>,
    ) -> Result<TicketType> {
        let row = sqlx::query(
            "INSERT INTO ticket_types (guild_id,name,label,emoji,description,color,welcome_message)
             VALUES (?,?,?,?,?,?,?)
             RETURNING id,guild_id,name,label,emoji,description,color,thread_name_pattern,
                       max_open_per_user,auto_close_hours,
                       welcome_message,staff_alert_channel_id,active",
        )
        .bind(guild_id)
        .bind(name)
        .bind(label)
        .bind(emoji)
        .bind(description)
        .bind(color)
        .bind(welcome_message)
        .fetch_one(&self.pool)
        .await?;
        Ok(Self::map_ticket_type(&row))
    }

    pub async fn get_ticket_type_by_id(&self, id: i64) -> Result<Option<TicketType>> {
        let row = sqlx::query(
            "SELECT id,guild_id,name,label,emoji,description,color,thread_name_pattern,
             max_open_per_user,auto_close_hours,
             welcome_message,staff_alert_channel_id,active
             FROM ticket_types WHERE id=?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.as_ref().map(Self::map_ticket_type))
    }

    pub async fn get_ticket_type_by_name(&self, guild_id: &str, name: &str) -> Result<Option<TicketType>> {
        let row = sqlx::query(
            "SELECT id,guild_id,name,label,emoji,description,color,thread_name_pattern,
             max_open_per_user,auto_close_hours,
             welcome_message,staff_alert_channel_id,active
             FROM ticket_types WHERE guild_id=? AND name=?",
        )
        .bind(guild_id)
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.as_ref().map(Self::map_ticket_type))
    }

    pub async fn get_ticket_types(&self, guild_id: &str) -> Result<Vec<TicketType>> {
        let rows = sqlx::query(
            "SELECT id,guild_id,name,label,emoji,description,color,thread_name_pattern,
             max_open_per_user,auto_close_hours,
             welcome_message,staff_alert_channel_id,active
             FROM ticket_types WHERE guild_id=? AND active=1 ORDER BY id ASC",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.iter().map(Self::map_ticket_type).collect())
    }

    /// Every internal name in the guild, **including deactivated categories** —
    /// the `UNIQUE(guild_id, name)` constraint spans inactive rows too, so name
    /// dedup must check them all.
    pub async fn get_all_ticket_type_names(&self, guild_id: &str) -> Result<Vec<String>> {
        let names: Vec<String> =
            sqlx::query_scalar("SELECT name FROM ticket_types WHERE guild_id=?")
                .bind(guild_id)
                .fetch_all(&self.pool)
                .await?;
        Ok(names)
    }

    pub async fn update_ticket_type(
        &self,
        id: i64,
        label: &str,
        emoji: Option<&str>,
        description: Option<&str>,
        color: &str,
        welcome_message: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE ticket_types SET label=?,emoji=?,description=?,color=?,welcome_message=? WHERE id=?",
        )
        .bind(label)
        .bind(emoji)
        .bind(description)
        .bind(color)
        .bind(welcome_message)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn set_ticket_type_max_open(&self, id: i64, max: i64) -> Result<()> {
        sqlx::query("UPDATE ticket_types SET max_open_per_user=? WHERE id=?")
            .bind(max).bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn set_ticket_type_auto_close(&self, id: i64, hours: Option<i64>) -> Result<()> {
        sqlx::query("UPDATE ticket_types SET auto_close_hours=? WHERE id=?")
            .bind(hours).bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn set_staff_alert_channel(&self, id: i64, channel_id: Option<&str>) -> Result<()> {
        sqlx::query("UPDATE ticket_types SET staff_alert_channel_id=? WHERE id=?")
            .bind(channel_id).bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn set_ticket_type_thread_pattern(&self, id: i64, pattern: &str) -> Result<()> {
        sqlx::query("UPDATE ticket_types SET thread_name_pattern=? WHERE id=?")
            .bind(pattern).bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn deactivate_ticket_type(&self, id: i64) -> Result<()> {
        sqlx::query("UPDATE ticket_types SET active=0 WHERE id=?")
            .bind(id).execute(&self.pool).await?;
        Ok(())
    }

    // ── Ticket type roles ─────────────────────────────────────────────────────

    pub async fn replace_type_roles(&self, type_id: i64, role_ids: &[String]) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM ticket_type_roles WHERE ticket_type_id=?")
            .bind(type_id).execute(&mut *tx).await?;
        for role_id in role_ids {
            sqlx::query("INSERT INTO ticket_type_roles (ticket_type_id,role_id) VALUES (?,?)")
                .bind(type_id).bind(role_id).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_type_roles(&self, type_id: i64) -> Result<Vec<String>> {
        let rows: Vec<String> =
            sqlx::query_scalar("SELECT role_id FROM ticket_type_roles WHERE ticket_type_id=?")
                .bind(type_id).fetch_all(&self.pool).await?;
        Ok(rows)
    }

    // ── Form fields ───────────────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub async fn add_form_field(
        &self,
        type_id: i64,
        label: &str,
        placeholder: Option<&str>,
        style: &str,
        required: bool,
        min_length: Option<i64>,
        max_length: Option<i64>,
        options: Option<&str>,
    ) -> Result<FormField> {
        let pos: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(position)+1,0) FROM form_fields WHERE ticket_type_id=?",
        )
        .bind(type_id)
        .fetch_one(&self.pool)
        .await?;

        let row = sqlx::query(
            "INSERT INTO form_fields (ticket_type_id,label,placeholder,style,required,min_length,max_length,position,options)
             VALUES (?,?,?,?,?,?,?,?,?)
             RETURNING id,ticket_type_id,label,placeholder,style,required,min_length,max_length,position,options",
        )
        .bind(type_id).bind(label).bind(placeholder).bind(style)
        .bind(required as i64).bind(min_length).bind(max_length).bind(pos).bind(options)
        .fetch_one(&self.pool).await?;

        Ok(Self::map_form_field(&row))
    }

    fn map_form_field(r: &sqlx::sqlite::SqliteRow) -> FormField {
        FormField {
            id: r.get("id"),
            ticket_type_id: r.get("ticket_type_id"),
            label: r.get("label"),
            placeholder: r.get("placeholder"),
            style: r.get("style"),
            required: r.get::<i64, _>("required") != 0,
            min_length: r.get("min_length"),
            max_length: r.get("max_length"),
            position: r.get("position"),
            options: r.get("options"),
        }
    }

    pub async fn get_form_fields(&self, type_id: i64) -> Result<Vec<FormField>> {
        let rows = sqlx::query(
            "SELECT id,ticket_type_id,label,placeholder,style,required,min_length,max_length,position,options
             FROM form_fields WHERE ticket_type_id=? ORDER BY position ASC",
        )
        .bind(type_id).fetch_all(&self.pool).await?;
        Ok(rows.iter().map(Self::map_form_field).collect())
    }

    /// Question counts for every category in a guild (`ticket_type_id → count`),
    /// for the hub's per-category question badge. Types with no questions are absent.
    pub async fn count_form_fields_by_type(
        &self,
        guild_id: &str,
    ) -> Result<std::collections::HashMap<i64, i64>> {
        let rows: Vec<(i64, i64)> = sqlx::query_as(
            "SELECT ff.ticket_type_id, COUNT(*) FROM form_fields ff
             JOIN ticket_types tt ON tt.id = ff.ticket_type_id
             WHERE tt.guild_id=? GROUP BY ff.ticket_type_id",
        )
        .bind(guild_id).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().collect())
    }

    /// How many intake questions the category has (cheaper than fetching them all).
    pub async fn count_form_fields(&self, type_id: i64) -> Result<i64> {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM form_fields WHERE ticket_type_id=?")
            .bind(type_id).fetch_one(&self.pool).await?;
        Ok(n)
    }

    pub async fn get_form_field(&self, field_id: i64) -> Result<Option<FormField>> {
        let row = sqlx::query(
            "SELECT id,ticket_type_id,label,placeholder,style,required,min_length,max_length,position,options
             FROM form_fields WHERE id=?",
        )
        .bind(field_id).fetch_optional(&self.pool).await?;
        Ok(row.as_ref().map(Self::map_form_field))
    }

    /// Update an intake question's editable fields (the style is fixed at creation).
    pub async fn update_form_field(
        &self,
        field_id: i64,
        label: &str,
        required: bool,
        options: Option<&str>,
        placeholder: Option<&str>,
    ) -> Result<()> {
        sqlx::query("UPDATE form_fields SET label=?, required=?, options=?, placeholder=? WHERE id=?")
            .bind(label).bind(required as i64).bind(options).bind(placeholder).bind(field_id)
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn remove_form_field(&self, type_id: i64, field_id: i64) -> Result<bool> {
        let res = sqlx::query("DELETE FROM form_fields WHERE id=? AND ticket_type_id=?")
            .bind(field_id).bind(type_id).execute(&self.pool).await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn clear_form_fields(&self, type_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM form_fields WHERE ticket_type_id=?")
            .bind(type_id).execute(&self.pool).await?;
        Ok(())
    }

    // ── Panels ────────────────────────────────────────────────────────────────

    fn map_panel(r: &sqlx::sqlite::SqliteRow) -> Panel {
        Panel {
            id: r.get("id"),
            guild_id: r.get("guild_id"),
            channel_id: r.get("channel_id"),
            message_id: r.get("message_id"),
            title: r.get("title"),
            description: r.get("description"),
            color: r.get("color"),
            image_url: r.get("image_url"),
            thumbnail_url: r.get("thumbnail_url"),
            footer_text: r.get("footer_text"),
            layout: r.get("layout"),
        }
    }

    pub async fn create_panel(
        &self,
        guild_id: &str,
        title: &str,
        description: Option<&str>,
        color: &str,
        layout: &str,
    ) -> Result<Panel> {
        let row = sqlx::query(
            "INSERT INTO panels (guild_id,title,description,color,layout)
             VALUES (?,?,?,?,?)
             RETURNING id,guild_id,channel_id,message_id,title,description,color,
                       image_url,thumbnail_url,footer_text,layout",
        )
        .bind(guild_id).bind(title).bind(description).bind(color).bind(layout)
        .fetch_one(&self.pool).await?;
        Ok(Self::map_panel(&row))
    }

    pub async fn get_panel(&self, id: i64) -> Result<Option<Panel>> {
        let row = sqlx::query(
            "SELECT id,guild_id,channel_id,message_id,title,description,color,
             image_url,thumbnail_url,footer_text,layout FROM panels WHERE id=?",
        )
        .bind(id).fetch_optional(&self.pool).await?;
        Ok(row.as_ref().map(Self::map_panel))
    }

    pub async fn get_panels(&self, guild_id: &str) -> Result<Vec<Panel>> {
        let rows = sqlx::query(
            "SELECT id,guild_id,channel_id,message_id,title,description,color,
             image_url,thumbnail_url,footer_text,layout
             FROM panels WHERE guild_id=? ORDER BY id",
        )
        .bind(guild_id).fetch_all(&self.pool).await?;
        Ok(rows.iter().map(Self::map_panel).collect())
    }

    pub async fn update_panel(
        &self,
        panel_id: i64,
        title: &str,
        description: Option<&str>,
        color: &str,
        layout: &str,
    ) -> Result<()> {
        sqlx::query("UPDATE panels SET title=?,description=?,color=?,layout=? WHERE id=?")
            .bind(title).bind(description).bind(color).bind(layout).bind(panel_id)
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn update_panel_message(&self, panel_id: i64, channel_id: &str, message_id: &str) -> Result<()> {
        sqlx::query("UPDATE panels SET channel_id=?,message_id=? WHERE id=?")
            .bind(channel_id).bind(message_id).bind(panel_id)
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn delete_panel(&self, panel_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM panels WHERE id=?")
            .bind(panel_id).execute(&self.pool).await?;
        Ok(())
    }

    /// Replace a panel's category links with exactly `type_ids`, in order.
    pub async fn set_panel_types(&self, panel_id: i64, type_ids: &[i64]) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM panel_ticket_types WHERE panel_id=?")
            .bind(panel_id).execute(&mut *tx).await?;
        for (pos, tid) in type_ids.iter().enumerate() {
            sqlx::query("INSERT INTO panel_ticket_types (panel_id,ticket_type_id,position) VALUES (?,?,?)")
                .bind(panel_id).bind(tid).bind(pos as i64).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// The panel carrying the given ticket type, if any. A unique index on
    /// panel_ticket_types(ticket_type_id) guarantees at most one.
    pub async fn get_panel_for_type(&self, type_id: i64) -> Result<Option<Panel>> {
        let row = sqlx::query(
            "SELECT p.id,p.guild_id,p.channel_id,p.message_id,p.title,p.description,p.color,
             p.image_url,p.thumbnail_url,p.footer_text,p.layout
             FROM panels p JOIN panel_ticket_types ptt ON ptt.panel_id=p.id
             WHERE ptt.ticket_type_id=?",
        )
        .bind(type_id).fetch_optional(&self.pool).await?;
        Ok(row.as_ref().map(Self::map_panel))
    }

    /// (category label, other panel title) for every id in `type_ids` that is
    /// already linked to a panel other than `panel_id`.
    pub async fn find_panel_type_conflicts(
        &self,
        panel_id: i64,
        type_ids: &[i64],
    ) -> Result<Vec<(String, String)>> {
        if type_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; type_ids.len()].join(",");
        let sql = format!(
            "SELECT tt.label, p.title
             FROM panel_ticket_types ptt
             JOIN ticket_types tt ON tt.id = ptt.ticket_type_id
             JOIN panels p ON p.id = ptt.panel_id
             WHERE ptt.panel_id != ? AND ptt.ticket_type_id IN ({placeholders})",
        );
        let mut q = sqlx::query(&sql).bind(panel_id);
        for tid in type_ids {
            q = q.bind(tid);
        }
        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows.iter().map(|r| (r.get("label"), r.get("title"))).collect())
    }

    /// Ticket type ids linked to any panel other than `panel_id` in this guild.
    pub async fn get_type_ids_on_other_panels(
        &self,
        guild_id: &str,
        panel_id: i64,
    ) -> Result<Vec<i64>> {
        let ids: Vec<i64> = sqlx::query_scalar(
            "SELECT ptt.ticket_type_id
             FROM panel_ticket_types ptt JOIN panels p ON p.id = ptt.panel_id
             WHERE p.guild_id=? AND ptt.panel_id != ?",
        )
        .bind(guild_id).bind(panel_id).fetch_all(&self.pool).await?;
        Ok(ids)
    }

    pub async fn get_panel_types(&self, panel_id: i64) -> Result<Vec<TicketType>> {
        let rows = sqlx::query(
            "SELECT tt.id,tt.guild_id,tt.name,tt.label,tt.emoji,tt.description,tt.color,
             tt.thread_name_pattern,tt.max_open_per_user,tt.auto_close_hours,
             tt.welcome_message,tt.staff_alert_channel_id,tt.active
             FROM ticket_types tt
             JOIN panel_ticket_types ptt ON ptt.ticket_type_id=tt.id
             WHERE ptt.panel_id=? AND tt.active=1
             ORDER BY ptt.position ASC",
        )
        .bind(panel_id).fetch_all(&self.pool).await?;
        Ok(rows.iter().map(Self::map_ticket_type).collect())
    }

    // ── Tickets ───────────────────────────────────────────────────────────────

    fn map_ticket(r: &sqlx::sqlite::SqliteRow) -> Ticket {
        Ticket {
            id: r.get("id"),
            ticket_number: r.get("ticket_number"),
            guild_id: r.get("guild_id"),
            ticket_type_id: r.get("ticket_type_id"),
            thread_id: r.get("thread_id"),
            parent_channel_id: r.get("parent_channel_id"),
            owner_id: r.get("owner_id"),
            claimed_by: r.get("claimed_by"),
            priority: r.get("priority"),
            status: TicketStatus::from(r.get::<String, _>("status")),
            close_reason: r.get("close_reason"),
            closed_by: r.get("closed_by"),
            form_responses: r.get("form_responses"),
            reported_message_id: r.get("reported_message_id"),
            reported_message_url: r.get("reported_message_url"),
            reported_message_content: r.get("reported_message_content"),
            reported_author_id: r.get("reported_author_id"),
            card_message_id: r.get("card_message_id"),
            created_at: r.get("created_at"),
            closed_at: r.get("closed_at"),
            last_activity_at: r.get("last_activity_at"),
        }
    }

    const TICKET_SELECT: &'static str =
        "id,ticket_number,guild_id,ticket_type_id,thread_id,parent_channel_id,
         owner_id,claimed_by,priority,status,close_reason,closed_by,form_responses,
         reported_message_id,reported_message_url,reported_message_content,reported_author_id,
         created_at,closed_at,last_activity_at";

    pub async fn create_ticket(
        &self,
        number: i64,
        guild_id: &str,
        ticket_type_id: Option<i64>,
        thread_id: &str,
        parent_channel_id: &str,
        owner_id: &str,
        form_responses: Option<&str>,
        reported_message_id: Option<&str>,
        reported_message_url: Option<&str>,
        reported_message_content: Option<&str>,
        reported_author_id: Option<&str>,
    ) -> Result<Ticket> {
        let row = sqlx::query(
            "INSERT INTO tickets (ticket_number,guild_id,ticket_type_id,thread_id,parent_channel_id,
             owner_id,form_responses,reported_message_id,reported_message_url,
             reported_message_content,reported_author_id)
             VALUES (?,?,?,?,?,?,?,?,?,?,?)
             RETURNING id,ticket_number,guild_id,ticket_type_id,thread_id,parent_channel_id,
                       owner_id,claimed_by,priority,status,close_reason,closed_by,form_responses,
                       reported_message_id,reported_message_url,reported_message_content,
                       reported_author_id,card_message_id,created_at,closed_at,last_activity_at",
        )
        .bind(number).bind(guild_id).bind(ticket_type_id).bind(thread_id)
        .bind(parent_channel_id).bind(owner_id).bind(form_responses)
        .bind(reported_message_id).bind(reported_message_url)
        .bind(reported_message_content).bind(reported_author_id)
        .fetch_one(&self.pool).await?;
        Ok(Self::map_ticket(&row))
    }

    pub async fn get_ticket_by_id(&self, id: i64) -> Result<Option<Ticket>> {
        let row = sqlx::query(
            "SELECT id,ticket_number,guild_id,ticket_type_id,thread_id,parent_channel_id,
             owner_id,claimed_by,priority,status,close_reason,closed_by,form_responses,
             reported_message_id,reported_message_url,reported_message_content,reported_author_id,
             card_message_id,created_at,closed_at,last_activity_at FROM tickets WHERE id=?",
        )
        .bind(id).fetch_optional(&self.pool).await?;
        Ok(row.as_ref().map(Self::map_ticket))
    }

    pub async fn get_ticket_by_thread(&self, thread_id: &str) -> Result<Option<Ticket>> {
        let row = sqlx::query(
            "SELECT id,ticket_number,guild_id,ticket_type_id,thread_id,parent_channel_id,
             owner_id,claimed_by,priority,status,close_reason,closed_by,form_responses,
             reported_message_id,reported_message_url,reported_message_content,reported_author_id,
             card_message_id,created_at,closed_at,last_activity_at FROM tickets WHERE thread_id=?",
        )
        .bind(thread_id).fetch_optional(&self.pool).await?;
        Ok(row.as_ref().map(Self::map_ticket))
    }

    pub async fn count_open_tickets_for_user(
        &self,
        guild_id: &str,
        owner_id: &str,
        ticket_type_id: i64,
    ) -> Result<i64> {
        let n: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tickets
             WHERE guild_id=? AND owner_id=? AND ticket_type_id=? AND status='open'",
        )
        .bind(guild_id).bind(owner_id).bind(ticket_type_id)
        .fetch_one(&self.pool).await?;
        Ok(n)
    }

    pub async fn get_open_tickets(&self, guild_id: &str) -> Result<Vec<Ticket>> {
        let rows = sqlx::query(
            "SELECT id,ticket_number,guild_id,ticket_type_id,thread_id,parent_channel_id,
             owner_id,claimed_by,priority,status,close_reason,closed_by,form_responses,
             reported_message_id,reported_message_url,reported_message_content,reported_author_id,
             card_message_id,created_at,closed_at,last_activity_at
             FROM tickets WHERE guild_id=? AND status='open' ORDER BY ticket_number ASC",
        )
        .bind(guild_id).fetch_all(&self.pool).await?;
        Ok(rows.iter().map(Self::map_ticket).collect())
    }

    pub async fn get_stale_tickets(&self) -> Result<Vec<Ticket>> {
        let rows = sqlx::query(
            "SELECT t.id,t.ticket_number,t.guild_id,t.ticket_type_id,t.thread_id,t.parent_channel_id,
             t.owner_id,t.claimed_by,t.priority,t.status,t.close_reason,t.closed_by,t.form_responses,
             t.reported_message_id,t.reported_message_url,t.reported_message_content,t.reported_author_id,
             t.card_message_id,t.created_at,t.closed_at,t.last_activity_at
             FROM tickets t
             JOIN ticket_types tt ON tt.id=t.ticket_type_id
             WHERE t.status='open'
               AND tt.auto_close_hours IS NOT NULL
               AND (julianday('now') - julianday(t.last_activity_at)) * 24 >= tt.auto_close_hours",
        )
        .fetch_all(&self.pool).await?;
        Ok(rows.iter().map(Self::map_ticket).collect())
    }

    pub async fn close_ticket(&self, id: i64, closed_by: &str, reason: Option<&str>) -> Result<()> {
        sqlx::query(
            "UPDATE tickets SET status='closed',closed_by=?,close_reason=?,
             closed_at=datetime('now') WHERE id=?",
        )
        .bind(closed_by).bind(reason).bind(id)
        .execute(&self.pool).await?;
        Ok(())
    }

    /// Remember the in-thread CV2 ticket card's message id so later claim /
    /// priority changes can re-render it.
    pub async fn set_ticket_card_message(&self, id: i64, message_id: &str) -> Result<()> {
        sqlx::query("UPDATE tickets SET card_message_id=? WHERE id=?")
            .bind(message_id).bind(id).execute(&self.pool).await?;
        Ok(())
    }

    /// Atomically claim a ticket only if it is currently unclaimed. Returns `true`
    /// if this call took the claim, `false` if it was already claimed (the guard
    /// closes the check-then-act race between two near-simultaneous claimers).
    pub async fn claim_ticket(&self, id: i64, user_id: &str) -> Result<bool> {
        let res = sqlx::query("UPDATE tickets SET claimed_by=? WHERE id=? AND claimed_by IS NULL")
            .bind(user_id).bind(id).execute(&self.pool).await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn unclaim_ticket(&self, id: i64) -> Result<()> {
        sqlx::query("UPDATE tickets SET claimed_by=NULL WHERE id=?")
            .bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn set_priority(&self, id: i64, priority: &str) -> Result<()> {
        sqlx::query("UPDATE tickets SET priority=? WHERE id=?")
            .bind(priority).bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn touch_ticket(&self, thread_id: &str) -> Result<()> {
        sqlx::query("UPDATE tickets SET last_activity_at=datetime('now') WHERE thread_id=?")
            .bind(thread_id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn transfer_ticket(&self, id: i64, new_type_id: i64) -> Result<()> {
        sqlx::query("UPDATE tickets SET ticket_type_id=? WHERE id=?")
            .bind(new_type_id).bind(id).execute(&self.pool).await?;
        Ok(())
    }

    // ── Ticket members ────────────────────────────────────────────────────────

    pub async fn add_member(&self, ticket_id: i64, user_id: &str, added_by: &str) -> Result<bool> {
        let res = sqlx::query(
            "INSERT OR IGNORE INTO ticket_members (ticket_id,user_id,added_by) VALUES (?,?,?)",
        )
        .bind(ticket_id).bind(user_id).bind(added_by).execute(&self.pool).await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn remove_member(&self, ticket_id: i64, user_id: &str) -> Result<bool> {
        let res = sqlx::query("DELETE FROM ticket_members WHERE ticket_id=? AND user_id=?")
            .bind(ticket_id).bind(user_id).execute(&self.pool).await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn get_members(&self, ticket_id: i64) -> Result<Vec<String>> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT user_id FROM ticket_members WHERE ticket_id=? ORDER BY added_at",
        )
        .bind(ticket_id).fetch_all(&self.pool).await?;
        Ok(rows)
    }

    // ── Staff notes ───────────────────────────────────────────────────────────

    pub async fn add_note(&self, ticket_id: i64, author_id: &str, content: &str) -> Result<TicketNote> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO ticket_notes (ticket_id,author_id,content) VALUES (?,?,?) RETURNING id",
        )
        .bind(ticket_id).bind(author_id).bind(content)
        .fetch_one(&self.pool).await?;

        let row = sqlx::query(
            "SELECT id,ticket_id,author_id,content,created_at FROM ticket_notes WHERE id=?",
        )
        .bind(id).fetch_one(&self.pool).await?;

        Ok(TicketNote {
            id: row.get("id"),
            ticket_id: row.get("ticket_id"),
            author_id: row.get("author_id"),
            content: row.get("content"),
            created_at: row.get("created_at"),
        })
    }

    pub async fn get_notes(&self, ticket_id: i64) -> Result<Vec<TicketNote>> {
        let rows = sqlx::query(
            "SELECT id,ticket_id,author_id,content,created_at FROM ticket_notes
             WHERE ticket_id=? ORDER BY created_at",
        )
        .bind(ticket_id).fetch_all(&self.pool).await?;
        Ok(rows.iter().map(|r| TicketNote {
            id: r.get("id"),
            ticket_id: r.get("ticket_id"),
            author_id: r.get("author_id"),
            content: r.get("content"),
            created_at: r.get("created_at"),
        }).collect())
    }

    // ── Ticket tags ───────────────────────────────────────────────────────────

    pub async fn add_tag(&self, ticket_id: i64, tag: &str, added_by: &str) -> Result<bool> {
        let res = sqlx::query(
            "INSERT OR IGNORE INTO ticket_tags (ticket_id,tag,added_by) VALUES (?,?,?)",
        )
        .bind(ticket_id).bind(tag).bind(added_by).execute(&self.pool).await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn remove_tag(&self, ticket_id: i64, tag: &str) -> Result<bool> {
        let res = sqlx::query("DELETE FROM ticket_tags WHERE ticket_id=? AND tag=?")
            .bind(ticket_id).bind(tag).execute(&self.pool).await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn get_tags(&self, ticket_id: i64) -> Result<Vec<String>> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT tag FROM ticket_tags WHERE ticket_id=? ORDER BY added_at",
        )
        .bind(ticket_id).fetch_all(&self.pool).await?;
        Ok(rows)
    }

    // ── Tag definitions ───────────────────────────────────────────────────────

    pub async fn create_tag_definition(&self, guild_id: &str, name: &str, color: Option<&str>) -> Result<bool> {
        let res = sqlx::query(
            "INSERT OR IGNORE INTO ticket_tag_definitions (guild_id,name,color) VALUES (?,?,?)",
        )
        .bind(guild_id).bind(name).bind(color).execute(&self.pool).await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn delete_tag_definition(&self, guild_id: &str, name: &str) -> Result<bool> {
        let res = sqlx::query("DELETE FROM ticket_tag_definitions WHERE guild_id=? AND name=?")
            .bind(guild_id).bind(name).execute(&self.pool).await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn get_tag_definitions(&self, guild_id: &str) -> Result<Vec<TicketTagDefinition>> {
        let rows = sqlx::query(
            "SELECT id,guild_id,name,color FROM ticket_tag_definitions WHERE guild_id=? ORDER BY name",
        )
        .bind(guild_id).fetch_all(&self.pool).await?;
        Ok(rows.iter().map(|r| TicketTagDefinition {
            id: r.get("id"),
            guild_id: r.get("guild_id"),
            name: r.get("name"),
            color: r.get("color"),
        }).collect())
    }

    // ── Messages ──────────────────────────────────────────────────────────────

    pub async fn save_message(
        &self,
        ticket_id: i64,
        message_id: Option<&str>,
        author_id: &str,
        author_name: &str,
        content: &str,
        attachments: Option<&str>,
        sent_at: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO ticket_messages (ticket_id,message_id,author_id,author_name,content,attachments,sent_at)
             VALUES (?,?,?,?,?,?,?)",
        )
        .bind(ticket_id).bind(message_id).bind(author_id).bind(author_name)
        .bind(content).bind(attachments).bind(sent_at)
        .execute(&self.pool).await?;
        Ok(())
    }

    // ── Blocklist ─────────────────────────────────────────────────────────────

    pub async fn blocklist_add(
        &self,
        guild_id: &str,
        user_id: &str,
        scope: &str,
        reason: &str,
        added_by: &str,
        expires_at: Option<&str>,
    ) -> Result<bool> {
        // REPLACE so re-blocking the same scope updates the reason/expiry.
        let res = sqlx::query(
            "INSERT OR REPLACE INTO blocklist (guild_id,user_id,scope,reason,added_by,expires_at)
             VALUES (?,?,?,?,?,?)",
        )
        .bind(guild_id).bind(user_id).bind(scope).bind(reason).bind(added_by).bind(expires_at)
        .execute(&self.pool).await?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn blocklist_remove(&self, guild_id: &str, user_id: &str, scope: &str) -> Result<bool> {
        let res = sqlx::query(
            "DELETE FROM blocklist WHERE guild_id=? AND user_id=? AND scope=?",
        )
        .bind(guild_id).bind(user_id).bind(scope).execute(&self.pool).await?;
        Ok(res.rows_affected() > 0)
    }

    /// The active (non-expired) blocklist entry covering `scope`, if any. A
    /// `ticket:{name}` scope is also covered by the `tickets` umbrella entry.
    pub async fn get_active_block(
        &self,
        guild_id: &str,
        user_id: &str,
        scope: &str,
    ) -> Result<Option<BlocklistEntry>> {
        let umbrella = if scope.starts_with("ticket:") { "tickets" } else { scope };
        let row = sqlx::query(
            "SELECT guild_id,user_id,scope,reason,added_by,added_at,expires_at FROM blocklist
             WHERE guild_id=? AND user_id=? AND (scope=? OR scope=?)
               AND (expires_at IS NULL OR expires_at > datetime('now'))
             ORDER BY (expires_at IS NULL) DESC, added_at DESC LIMIT 1",
        )
        .bind(guild_id).bind(user_id).bind(scope).bind(umbrella)
        .fetch_optional(&self.pool).await?;
        Ok(row.as_ref().map(Self::map_blocklist))
    }

    pub async fn is_blocklisted(&self, guild_id: &str, user_id: &str, scope: &str) -> Result<bool> {
        Ok(self.get_active_block(guild_id, user_id, scope).await?.is_some())
    }

    pub async fn get_blocklist(&self, guild_id: &str) -> Result<Vec<BlocklistEntry>> {
        let rows = sqlx::query(
            "SELECT guild_id,user_id,scope,reason,added_by,added_at,expires_at FROM blocklist
             WHERE guild_id=? ORDER BY added_at DESC",
        )
        .bind(guild_id).fetch_all(&self.pool).await?;
        Ok(rows.iter().map(Self::map_blocklist).collect())
    }

    fn map_blocklist(r: &sqlx::sqlite::SqliteRow) -> BlocklistEntry {
        BlocklistEntry {
            guild_id: r.get("guild_id"),
            user_id: r.get("user_id"),
            scope: r.get("scope"),
            reason: r.get("reason"),
            added_by: r.get("added_by"),
            added_at: r.get("added_at"),
            expires_at: r.get("expires_at"),
        }
    }

    // ── Moderation config ─────────────────────────────────────────────────────

    pub async fn get_or_create_mod_config(&self, guild_id: &str) -> Result<ModConfig> {
        sqlx::query("INSERT OR IGNORE INTO mod_config (guild_id) VALUES (?)")
            .bind(guild_id).execute(&self.pool).await?;
        let row = sqlx::query(
            "SELECT guild_id,staff_role_ids,dm_on_warn,dm_on_timeout,dm_on_kick,dm_on_ban,
             appeal_cooldown_days FROM mod_config WHERE guild_id=?",
        )
        .bind(guild_id).fetch_one(&self.pool).await?;
        Ok(ModConfig {
            guild_id: row.get("guild_id"),
            staff_role_ids: row.get("staff_role_ids"),
            dm_on_warn: row.get::<i64, _>("dm_on_warn") != 0,
            dm_on_timeout: row.get::<i64, _>("dm_on_timeout") != 0,
            dm_on_kick: row.get::<i64, _>("dm_on_kick") != 0,
            dm_on_ban: row.get::<i64, _>("dm_on_ban") != 0,
            appeal_cooldown_days: row.get("appeal_cooldown_days"),
        })
    }

    pub async fn update_mod_config(
        &self,
        guild_id: &str,
        staff_role_ids: Option<&str>,
        dm_on_warn: bool,
        dm_on_timeout: bool,
        dm_on_kick: bool,
        dm_on_ban: bool,
        appeal_cooldown_days: i64,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO mod_config (guild_id,staff_role_ids,dm_on_warn,dm_on_timeout,dm_on_kick,dm_on_ban,appeal_cooldown_days)
             VALUES (?,?,?,?,?,?,?)
             ON CONFLICT(guild_id) DO UPDATE SET
               staff_role_ids=excluded.staff_role_ids,
               dm_on_warn=excluded.dm_on_warn,
               dm_on_timeout=excluded.dm_on_timeout,
               dm_on_kick=excluded.dm_on_kick,
               dm_on_ban=excluded.dm_on_ban,
               appeal_cooldown_days=excluded.appeal_cooldown_days",
        )
        .bind(guild_id).bind(staff_role_ids)
        .bind(dm_on_warn as i64).bind(dm_on_timeout as i64)
        .bind(dm_on_kick as i64).bind(dm_on_ban as i64)
        .bind(appeal_cooldown_days)
        .execute(&self.pool).await?;
        Ok(())
    }

    // ── Infractions ───────────────────────────────────────────────────────────

    fn map_infraction(r: &sqlx::sqlite::SqliteRow) -> Infraction {
        Infraction {
            id: r.get("id"),
            guild_id: r.get("guild_id"),
            user_id: r.get("user_id"),
            moderator_id: r.get("moderator_id"),
            kind: r.get("kind"),
            reason: r.get("reason"),
            duration_secs: r.get("duration_secs"),
            active: r.get::<i64, _>("active") != 0,
            appealable: r.get::<i64, _>("appealable") != 0,
            created_at: r.get("created_at"),
            expires_at: r.get("expires_at"),
        }
    }

    pub async fn create_infraction(
        &self,
        guild_id: &str,
        user_id: &str,
        moderator_id: &str,
        kind: &str,
        reason: &str,
        duration_secs: Option<i64>,
        appealable: bool,
        expires_at: Option<&str>,
    ) -> Result<Infraction> {
        let row = sqlx::query(
            "INSERT INTO infractions (guild_id,user_id,moderator_id,kind,reason,duration_secs,appealable,expires_at)
             VALUES (?,?,?,?,?,?,?,?)
             RETURNING id,guild_id,user_id,moderator_id,kind,reason,duration_secs,active,appealable,created_at,expires_at",
        )
        .bind(guild_id).bind(user_id).bind(moderator_id).bind(kind).bind(reason)
        .bind(duration_secs).bind(appealable as i64).bind(expires_at)
        .fetch_one(&self.pool).await?;
        Ok(Self::map_infraction(&row))
    }

    pub async fn get_infractions(&self, guild_id: &str, user_id: &str) -> Result<Vec<Infraction>> {
        let rows = sqlx::query(
            "SELECT id,guild_id,user_id,moderator_id,kind,reason,duration_secs,active,appealable,created_at,expires_at
             FROM infractions WHERE guild_id=? AND user_id=? ORDER BY created_at DESC",
        )
        .bind(guild_id).bind(user_id).fetch_all(&self.pool).await?;
        Ok(rows.iter().map(Self::map_infraction).collect())
    }

    pub async fn get_infraction_by_id(&self, id: i64) -> Result<Option<Infraction>> {
        let row = sqlx::query(
            "SELECT id,guild_id,user_id,moderator_id,kind,reason,duration_secs,active,appealable,created_at,expires_at
             FROM infractions WHERE id=?",
        )
        .bind(id).fetch_optional(&self.pool).await?;
        Ok(row.as_ref().map(Self::map_infraction))
    }

    pub async fn deactivate_infraction(&self, id: i64) -> Result<()> {
        sqlx::query("UPDATE infractions SET active=0 WHERE id=?")
            .bind(id).execute(&self.pool).await?;
        Ok(())
    }

    /// Active, time-limited bans (temporary bans). The expiry task filters these by
    /// comparing `expires_at` against the current time in Rust (avoids SQLite
    /// datetime-format pitfalls with RFC3339 strings).
    pub async fn get_active_temp_bans(&self) -> Result<Vec<Infraction>> {
        let rows = sqlx::query(
            "SELECT id,guild_id,user_id,moderator_id,kind,reason,duration_secs,active,appealable,created_at,expires_at
             FROM infractions WHERE kind='ban' AND active=1 AND expires_at IS NOT NULL",
        )
        .fetch_all(&self.pool).await?;
        Ok(rows.iter().map(Self::map_infraction).collect())
    }

    /// Mark a user's still-active ban infractions inactive (e.g. after a manual unban
    /// or accepted appeal), so the temp-ban expiry task won't try to lift them again.
    pub async fn deactivate_active_bans(&self, guild_id: &str, user_id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE infractions SET active=0 WHERE kind='ban' AND active=1 AND guild_id=? AND user_id=?",
        )
        .bind(guild_id).bind(user_id).execute(&self.pool).await?;
        Ok(())
    }

    // ── Appeals ───────────────────────────────────────────────────────────────

    fn map_appeal(r: &sqlx::sqlite::SqliteRow) -> Appeal {
        Appeal {
            id: r.get("id"),
            guild_id: r.get("guild_id"),
            infraction_id: r.get("infraction_id"),
            user_id: r.get("user_id"),
            thread_id: r.get("thread_id"),
            card_message_id: r.get("card_message_id"),
            reason: r.get("reason"),
            status: r.get("status"),
            response: r.get("response"),
            responded_by: r.get("responded_by"),
            created_at: r.get("created_at"),
            responded_at: r.get("responded_at"),
        }
    }

    pub async fn create_appeal(
        &self,
        guild_id: &str,
        infraction_id: i64,
        user_id: &str,
        reason: &str,
    ) -> Result<Appeal> {
        let row = sqlx::query(
            "INSERT INTO appeals (guild_id,infraction_id,user_id,reason)
             VALUES (?,?,?,?)
             RETURNING id,guild_id,infraction_id,user_id,thread_id,card_message_id,reason,
                       status,response,responded_by,created_at,responded_at",
        )
        .bind(guild_id).bind(infraction_id).bind(user_id).bind(reason)
        .fetch_one(&self.pool).await?;
        Ok(Self::map_appeal(&row))
    }

    pub async fn set_appeal_thread(&self, appeal_id: i64, thread_id: &str, card_message_id: &str) -> Result<()> {
        sqlx::query("UPDATE appeals SET thread_id=?,card_message_id=? WHERE id=?")
            .bind(thread_id).bind(card_message_id).bind(appeal_id)
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn resolve_appeal(
        &self,
        appeal_id: i64,
        status: &str,
        response: &str,
        responded_by: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE appeals SET status=?,response=?,responded_by=?,responded_at=datetime('now') WHERE id=?",
        )
        .bind(status).bind(response).bind(responded_by).bind(appeal_id)
        .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn get_appeal_by_thread(&self, thread_id: &str) -> Result<Option<Appeal>> {
        let row = sqlx::query(
            "SELECT id,guild_id,infraction_id,user_id,thread_id,card_message_id,reason,
             status,response,responded_by,created_at,responded_at
             FROM appeals WHERE thread_id=?",
        )
        .bind(thread_id).fetch_optional(&self.pool).await?;
        Ok(row.as_ref().map(Self::map_appeal))
    }

    pub async fn get_appeal_by_id(&self, id: i64) -> Result<Option<Appeal>> {
        let row = sqlx::query(
            "SELECT id,guild_id,infraction_id,user_id,thread_id,card_message_id,reason,
             status,response,responded_by,created_at,responded_at
             FROM appeals WHERE id=?",
        )
        .bind(id).fetch_optional(&self.pool).await?;
        Ok(row.as_ref().map(Self::map_appeal))
    }

    pub async fn get_appeal_for_infraction(&self, infraction_id: i64) -> Result<Option<Appeal>> {
        let row = sqlx::query(
            "SELECT id,guild_id,infraction_id,user_id,thread_id,card_message_id,reason,
             status,response,responded_by,created_at,responded_at
             FROM appeals WHERE infraction_id=?",
        )
        .bind(infraction_id).fetch_optional(&self.pool).await?;
        Ok(row.as_ref().map(Self::map_appeal))
    }

    /// Returns the most recent ban appeal for a user in a guild (for cooldown check)
    pub async fn get_latest_ban_appeal(&self, guild_id: &str, user_id: &str) -> Result<Option<Appeal>> {
        let row = sqlx::query(
            "SELECT a.id,a.guild_id,a.infraction_id,a.user_id,a.thread_id,a.card_message_id,a.reason,
             a.status,a.response,a.responded_by,a.created_at,a.responded_at
             FROM appeals a
             JOIN infractions i ON i.id=a.infraction_id
             WHERE a.guild_id=? AND a.user_id=? AND i.kind='ban'
             ORDER BY a.created_at DESC LIMIT 1",
        )
        .bind(guild_id).bind(user_id).fetch_optional(&self.pool).await?;
        Ok(row.as_ref().map(Self::map_appeal))
    }

    // ── Reports ───────────────────────────────────────────────────────────────

    fn map_report(r: &sqlx::sqlite::SqliteRow) -> Report {
        Report {
            id: r.get("id"),
            guild_id: r.get("guild_id"),
            reporter_id: r.get("reporter_id"),
            target_user_id: r.get("target_user_id"),
            message_id: r.get("message_id"),
            message_url: r.get("message_url"),
            message_content: r.get("message_content"),
            reason: r.get("reason"),
            card_message_id: r.get("card_message_id"),
            thread_id: r.get("thread_id"),
            status: r.get("status"),
            resolved_by: r.get("resolved_by"),
            concern_raised: r.get::<i64, _>("concern_raised") != 0,
            created_at: r.get("created_at"),
            profile_parts: r.get("profile_parts"),
        }
    }

    pub async fn create_report(
        &self,
        guild_id: &str,
        reporter_id: &str,
        target_user_id: &str,
        message_id: Option<&str>,
        message_url: Option<&str>,
        message_content: Option<&str>,
        reason: Option<&str>,
        profile_parts: Option<&str>,
    ) -> Result<Report> {
        let row = sqlx::query(
            "INSERT INTO reports (guild_id,reporter_id,target_user_id,message_id,message_url,message_content,reason,profile_parts)
             VALUES (?,?,?,?,?,?,?,?)
             RETURNING id,guild_id,reporter_id,target_user_id,message_id,message_url,message_content,
                       reason,card_message_id,thread_id,status,resolved_by,concern_raised,created_at,profile_parts",
        )
        .bind(guild_id).bind(reporter_id).bind(target_user_id)
        .bind(message_id).bind(message_url).bind(message_content).bind(reason).bind(profile_parts)
        .fetch_one(&self.pool).await?;
        Ok(Self::map_report(&row))
    }

    /// True if `reporter_id` already has an unresolved (`status='open'`) report
    /// against `target_user_id` in this guild — used to dedup spammy re-reports.
    pub async fn has_open_report(
        &self,
        guild_id: &str,
        reporter_id: &str,
        target_user_id: &str,
    ) -> Result<bool> {
        let row = sqlx::query(
            "SELECT 1 FROM reports
             WHERE guild_id=? AND reporter_id=? AND target_user_id=? AND status='open' LIMIT 1",
        )
        .bind(guild_id)
        .bind(reporter_id)
        .bind(target_user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.is_some())
    }

    pub async fn set_report_card(&self, report_id: i64, card_message_id: &str) -> Result<()> {
        sqlx::query("UPDATE reports SET card_message_id=? WHERE id=?")
            .bind(card_message_id).bind(report_id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn set_report_thread(&self, report_id: i64, thread_id: &str) -> Result<()> {
        sqlx::query("UPDATE reports SET thread_id=? WHERE id=?")
            .bind(thread_id).bind(report_id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn resolve_report(&self, report_id: i64, status: &str, resolved_by: &str) -> Result<()> {
        sqlx::query("UPDATE reports SET status=?,resolved_by=? WHERE id=?")
            .bind(status).bind(resolved_by).bind(report_id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn set_report_concern_raised(&self, report_id: i64) -> Result<()> {
        sqlx::query("UPDATE reports SET concern_raised=1 WHERE id=?")
            .bind(report_id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn get_report_by_id(&self, id: i64) -> Result<Option<Report>> {
        let row = sqlx::query(
            "SELECT id,guild_id,reporter_id,target_user_id,message_id,message_url,message_content,
             reason,card_message_id,thread_id,status,resolved_by,concern_raised,created_at,profile_parts
             FROM reports WHERE id=?",
        )
        .bind(id).fetch_optional(&self.pool).await?;
        Ok(row.as_ref().map(Self::map_report))
    }

    pub async fn get_report_by_thread(&self, thread_id: &str) -> Result<Option<Report>> {
        let row = sqlx::query(
            "SELECT id,guild_id,reporter_id,target_user_id,message_id,message_url,message_content,
             reason,card_message_id,thread_id,status,resolved_by,concern_raised,created_at,profile_parts
             FROM reports WHERE thread_id=?",
        )
        .bind(thread_id).fetch_optional(&self.pool).await?;
        Ok(row.as_ref().map(Self::map_report))
    }

    // ── Concerns ──────────────────────────────────────────────────────────────

    fn map_concern(r: &sqlx::sqlite::SqliteRow) -> Concern {
        Concern {
            id: r.get("id"),
            guild_id: r.get("guild_id"),
            user_id: r.get("user_id"),
            kind: r.get("kind"),
            source_id: r.get("source_id"),
            reason: r.get("reason"),
            status: r.get("status"),
            reviewed_by: r.get("reviewed_by"),
            created_at: r.get("created_at"),
            target_moderator_id: r.get("target_moderator_id"),
            reviewed_at: r.get("reviewed_at"),
        }
    }

    pub async fn create_concern(
        &self,
        guild_id: &str,
        user_id: &str,
        kind: &str,
        source_id: i64,
        reason: &str,
        target_moderator_id: Option<&str>,
    ) -> Result<Concern> {
        let row = sqlx::query(
            "INSERT INTO concerns (guild_id,user_id,kind,source_id,reason,target_moderator_id)
             VALUES (?,?,?,?,?,?)
             RETURNING id,guild_id,user_id,kind,source_id,reason,status,reviewed_by,created_at,
                       target_moderator_id,reviewed_at",
        )
        .bind(guild_id).bind(user_id).bind(kind).bind(source_id).bind(reason).bind(target_moderator_id)
        .fetch_one(&self.pool).await?;
        Ok(Self::map_concern(&row))
    }

    pub async fn mark_concern_reviewed(&self, concern_id: i64, reviewed_by: &str) -> Result<()> {
        sqlx::query("UPDATE concerns SET status='reviewed',reviewed_by=?,reviewed_at=datetime('now') WHERE id=?")
            .bind(reviewed_by).bind(concern_id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn get_concern_by_id(&self, id: i64) -> Result<Option<Concern>> {
        let row = sqlx::query(
            "SELECT id,guild_id,user_id,kind,source_id,reason,status,reviewed_by,created_at,
             target_moderator_id,reviewed_at
             FROM concerns WHERE id=?",
        )
        .bind(id).fetch_optional(&self.pool).await?;
        Ok(row.as_ref().map(Self::map_concern))
    }

    /// Concern counts grouped by the moderator whose decision was challenged,
    /// most-challenged first.
    pub async fn concern_stats(&self, guild_id: &str) -> Result<Vec<ConcernStat>> {
        let rows = sqlx::query(
            "SELECT target_moderator_id,
                    COUNT(*) AS total,
                    SUM(CASE WHEN kind='appeal' THEN 1 ELSE 0 END) AS denied_appeals,
                    SUM(CASE WHEN kind='report' THEN 1 ELSE 0 END) AS dismissed_reports
             FROM concerns WHERE guild_id=?
             GROUP BY target_moderator_id
             ORDER BY total DESC",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| ConcernStat {
                moderator_id: r.get("target_moderator_id"),
                total: r.get("total"),
                denied_appeals: r.get("denied_appeals"),
                dismissed_reports: r.get("dismissed_reports"),
            })
            .collect())
    }

    /// Pending vs reviewed concern counts for a guild.
    pub async fn concern_status_counts(&self, guild_id: &str) -> Result<(i64, i64)> {
        let row = sqlx::query(
            "SELECT
                SUM(CASE WHEN status='pending' THEN 1 ELSE 0 END) AS pending,
                SUM(CASE WHEN status='reviewed' THEN 1 ELSE 0 END) AS reviewed
             FROM concerns WHERE guild_id=?",
        )
        .bind(guild_id)
        .fetch_one(&self.pool)
        .await?;
        Ok((
            row.get::<Option<i64>, _>("pending").unwrap_or(0),
            row.get::<Option<i64>, _>("reviewed").unwrap_or(0),
        ))
    }

    // ── Raid config ───────────────────────────────────────────────────────────

    pub async fn get_or_create_raid_config(&self, guild_id: &str) -> Result<RaidConfig> {
        sqlx::query("INSERT OR IGNORE INTO raid_config (guild_id) VALUES (?)")
            .bind(guild_id).execute(&self.pool).await?;
        let row = sqlx::query(
            "SELECT guild_id,join_threshold,decay_rate,slowmode_secs,raid_active,raid_triggered_at
             FROM raid_config WHERE guild_id=?",
        )
        .bind(guild_id).fetch_one(&self.pool).await?;
        Ok(RaidConfig {
            guild_id: row.get("guild_id"),
            join_threshold: row.get("join_threshold"),
            decay_rate: row.get("decay_rate"),
            slowmode_secs: row.get("slowmode_secs"),
            raid_active: row.get::<i64, _>("raid_active") != 0,
            raid_triggered_at: row.get("raid_triggered_at"),
        })
    }

    pub async fn set_raid_active(&self, guild_id: &str, active: bool) -> Result<()> {
        // `datetime('now')` must be evaluated by SQLite, not bound as a string —
        // binding it stored the literal text `datetime('now')` in the column.
        let sql = if active {
            "UPDATE raid_config SET raid_active=1,raid_triggered_at=datetime('now') WHERE guild_id=?"
        } else {
            "UPDATE raid_config SET raid_active=0,raid_triggered_at=NULL WHERE guild_id=?"
        };
        sqlx::query(sql).bind(guild_id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn update_raid_config(
        &self,
        guild_id: &str,
        join_threshold: f64,
        decay_rate: f64,
        slowmode_secs: i64,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO raid_config (guild_id,join_threshold,decay_rate,slowmode_secs)
             VALUES (?,?,?,?)
             ON CONFLICT(guild_id) DO UPDATE SET
               join_threshold=excluded.join_threshold,
               decay_rate=excluded.decay_rate,
               slowmode_secs=excluded.slowmode_secs",
        )
        .bind(guild_id).bind(join_threshold).bind(decay_rate).bind(slowmode_secs)
        .execute(&self.pool).await?;
        Ok(())
    }

    // ── Slowmode config ───────────────────────────────────────────────────────

    pub async fn get_or_create_slowmode_config(&self, guild_id: &str) -> Result<SlowmodeConfig> {
        sqlx::query("INSERT OR IGNORE INTO slowmode_config (guild_id) VALUES (?)")
            .bind(guild_id).execute(&self.pool).await?;
        let row = sqlx::query(
            "SELECT guild_id,enabled,window_secs,capacity,excluded_channels
             FROM slowmode_config WHERE guild_id=?",
        )
        .bind(guild_id).fetch_one(&self.pool).await?;
        Ok(SlowmodeConfig {
            guild_id: row.get("guild_id"),
            enabled: row.get::<i64, _>("enabled") != 0,
            window_secs: row.get("window_secs"),
            capacity: row.get("capacity"),
            excluded_channels: row.get("excluded_channels"),
        })
    }

    pub async fn update_slowmode_config(
        &self,
        guild_id: &str,
        enabled: bool,
        window_secs: i64,
        capacity: i64,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO slowmode_config (guild_id,enabled,window_secs,capacity)
             VALUES (?,?,?,?)
             ON CONFLICT(guild_id) DO UPDATE SET
               enabled=excluded.enabled,
               window_secs=excluded.window_secs,
               capacity=excluded.capacity",
        )
        .bind(guild_id).bind(enabled as i64).bind(window_secs).bind(capacity)
        .execute(&self.pool).await?;
        Ok(())
    }
}
