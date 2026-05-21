PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

-- ── Guild configuration ───────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS guild_config (
    guild_id                TEXT NOT NULL PRIMARY KEY,
    log_channel_id          TEXT,
    mod_log_channel_id      TEXT,
    chat_log_channel_id     TEXT,
    ticket_log_channel_id   TEXT,
    ticket_channel_id       TEXT,          -- parent text channel for private threads
    reports_channel_id      TEXT,          -- where report cards are posted (staff-only)
    appeals_channel_id      TEXT,          -- where appeal threads are created
    concerns_channel_id     TEXT,          -- admin-only accountability channel
    report_type_id          INTEGER,       -- ticket_types.id used for "Report message"
    ticket_count            INTEGER NOT NULL DEFAULT 0,
    created_at              TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ── Ticket types ──────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS ticket_types (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id             TEXT    NOT NULL,
    name                 TEXT    NOT NULL,
    label                TEXT    NOT NULL,
    emoji                TEXT,
    description          TEXT,
    color                TEXT    NOT NULL DEFAULT '5865F2',
    thread_name_pattern  TEXT    NOT NULL DEFAULT 'ticket-{number}-{username}',
    has_form             INTEGER NOT NULL DEFAULT 0,
    max_open_per_user    INTEGER NOT NULL DEFAULT 1,
    auto_close_hours     INTEGER,
    ping_roles           TEXT,             -- JSON array of role IDs
    welcome_message      TEXT,
    auto_add_staff       INTEGER NOT NULL DEFAULT 0,
    staff_alert_channel_id TEXT,
    active               INTEGER NOT NULL DEFAULT 1,
    created_at           TEXT    NOT NULL DEFAULT (datetime('now')),
    UNIQUE(guild_id, name)
);

-- ── Staff roles per ticket type (many-to-many) ────────────────────────────────
CREATE TABLE IF NOT EXISTS ticket_type_roles (
    ticket_type_id INTEGER NOT NULL REFERENCES ticket_types(id) ON DELETE CASCADE,
    role_id        TEXT    NOT NULL,
    PRIMARY KEY (ticket_type_id, role_id)
);

-- ── Form fields (max 5 per type, Discord modal limit) ─────────────────────────
CREATE TABLE IF NOT EXISTS form_fields (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    ticket_type_id INTEGER NOT NULL REFERENCES ticket_types(id) ON DELETE CASCADE,
    label          TEXT    NOT NULL,
    placeholder    TEXT,
    style          TEXT    NOT NULL DEFAULT 'short',  -- short | paragraph
    required       INTEGER NOT NULL DEFAULT 1,
    min_length     INTEGER,
    max_length     INTEGER,
    position       INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_form_fields_type ON form_fields(ticket_type_id, position);

-- ── Panels ────────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS panels (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id       TEXT    NOT NULL,
    channel_id     TEXT,
    message_id     TEXT,
    title          TEXT    NOT NULL DEFAULT 'Support',
    description    TEXT,
    color          TEXT    NOT NULL DEFAULT '5865F2',
    image_url      TEXT,
    thumbnail_url  TEXT,
    footer_text    TEXT,
    layout         TEXT    NOT NULL DEFAULT 'buttons',  -- buttons | select
    created_at     TEXT    NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_panels_guild ON panels(guild_id);

-- ── Panel ↔ ticket type mapping ───────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS panel_ticket_types (
    panel_id       INTEGER NOT NULL REFERENCES panels(id) ON DELETE CASCADE,
    ticket_type_id INTEGER NOT NULL REFERENCES ticket_types(id) ON DELETE CASCADE,
    position       INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (panel_id, ticket_type_id)
);

-- ── Tickets (thread-based) ────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS tickets (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    ticket_number           INTEGER NOT NULL,
    guild_id                TEXT    NOT NULL,
    ticket_type_id          INTEGER REFERENCES ticket_types(id),
    thread_id               TEXT    NOT NULL UNIQUE,
    parent_channel_id       TEXT    NOT NULL,
    owner_id                TEXT    NOT NULL,
    claimed_by              TEXT,
    priority                TEXT    NOT NULL DEFAULT 'normal',  -- low|normal|high|urgent
    status                  TEXT    NOT NULL DEFAULT 'open',    -- open|closed
    close_reason            TEXT,
    closed_by               TEXT,
    form_responses          TEXT,   -- JSON: {"field label": "answer"}
    reported_message_id     TEXT,
    reported_message_url    TEXT,
    reported_message_content TEXT,
    reported_author_id      TEXT,
    created_at              TEXT    NOT NULL DEFAULT (datetime('now')),
    closed_at               TEXT,
    last_activity_at        TEXT    NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_tickets_guild    ON tickets(guild_id);
CREATE INDEX IF NOT EXISTS idx_tickets_thread   ON tickets(thread_id);
CREATE INDEX IF NOT EXISTS idx_tickets_owner    ON tickets(guild_id, owner_id, status);
CREATE INDEX IF NOT EXISTS idx_tickets_status   ON tickets(guild_id, status);
CREATE INDEX IF NOT EXISTS idx_tickets_activity ON tickets(last_activity_at) WHERE status = 'open';

-- ── Ticket members ────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS ticket_members (
    ticket_id INTEGER NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    user_id   TEXT    NOT NULL,
    added_by  TEXT    NOT NULL,
    added_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (ticket_id, user_id)
);

-- ── Private staff notes ───────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS ticket_notes (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    ticket_id  INTEGER NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    author_id  TEXT    NOT NULL,
    content    TEXT    NOT NULL,
    created_at TEXT    NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_notes_ticket ON ticket_notes(ticket_id);

-- ── Ticket tag assignments ────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS ticket_tags (
    ticket_id INTEGER NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    tag       TEXT    NOT NULL,
    added_by  TEXT    NOT NULL,
    added_at  TEXT    NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (ticket_id, tag)
);

-- ── Guild-configurable tag definitions ───────────────────────────────────────
CREATE TABLE IF NOT EXISTS ticket_tag_definitions (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id  TEXT NOT NULL,
    name      TEXT NOT NULL,
    color     TEXT,   -- optional hex
    UNIQUE(guild_id, name)
);

-- ── Message log for transcripts ───────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS ticket_messages (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    ticket_id   INTEGER NOT NULL REFERENCES tickets(id) ON DELETE CASCADE,
    message_id  TEXT,
    author_id   TEXT    NOT NULL,
    author_name TEXT    NOT NULL,
    content     TEXT    NOT NULL,
    attachments TEXT,   -- JSON array of URLs
    sent_at     TEXT    NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_messages_ticket ON ticket_messages(ticket_id, sent_at);

-- ── Blocklist (replaces blacklist, supports scope) ────────────────────────────
CREATE TABLE IF NOT EXISTS blocklist (
    guild_id TEXT    NOT NULL,
    user_id  TEXT    NOT NULL,
    scope    TEXT    NOT NULL DEFAULT 'global',  -- 'global' or ticket_type name
    reason   TEXT    NOT NULL,
    added_by TEXT    NOT NULL,
    added_at TEXT    NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (guild_id, user_id, scope)
);

-- ── Moderation config ─────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS mod_config (
    guild_id             TEXT    NOT NULL PRIMARY KEY,
    staff_role_ids       TEXT,          -- JSON array of role ID strings
    dm_on_warn           INTEGER NOT NULL DEFAULT 1,
    dm_on_timeout        INTEGER NOT NULL DEFAULT 1,
    dm_on_kick           INTEGER NOT NULL DEFAULT 1,
    dm_on_ban            INTEGER NOT NULL DEFAULT 1,
    appeal_cooldown_days INTEGER NOT NULL DEFAULT 7
);

-- ── Infraction log ────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS infractions (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id      TEXT    NOT NULL,
    user_id       TEXT    NOT NULL,
    moderator_id  TEXT    NOT NULL,
    kind          TEXT    NOT NULL,  -- warn|timeout|untimeout|kick|ban|unban|blocklist|unblocklist
    reason        TEXT    NOT NULL,
    duration_secs INTEGER,           -- NULL = permanent (used by timeout)
    active        INTEGER NOT NULL DEFAULT 1,
    appealable    INTEGER NOT NULL DEFAULT 1,
    created_at    TEXT    NOT NULL DEFAULT (datetime('now')),
    expires_at    TEXT
);
CREATE INDEX IF NOT EXISTS idx_infractions_guild_user ON infractions(guild_id, user_id);
CREATE INDEX IF NOT EXISTS idx_infractions_active
    ON infractions(guild_id, active, expires_at) WHERE active = 1;

-- ── DM-based appeals ──────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS appeals (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id      TEXT    NOT NULL,
    infraction_id INTEGER NOT NULL REFERENCES infractions(id),
    user_id       TEXT    NOT NULL,
    thread_id     TEXT    UNIQUE,       -- appeal thread in the appeals channel
    card_message_id TEXT,               -- message ID of the appeal card (for edits)
    reason        TEXT    NOT NULL,
    status        TEXT    NOT NULL DEFAULT 'pending',  -- pending|accepted|denied
    response      TEXT,
    responded_by  TEXT,
    created_at    TEXT    NOT NULL DEFAULT (datetime('now')),
    responded_at  TEXT
);
CREATE INDEX IF NOT EXISTS idx_appeals_guild ON appeals(guild_id, status);
CREATE INDEX IF NOT EXISTS idx_appeals_user  ON appeals(user_id, status);
CREATE INDEX IF NOT EXISTS idx_appeals_thread ON appeals(thread_id);

-- ── Reports ───────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS reports (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id        TEXT NOT NULL,
    reporter_id     TEXT NOT NULL,
    target_user_id  TEXT NOT NULL,
    message_id      TEXT,
    message_url     TEXT,
    message_content TEXT,
    reason          TEXT,
    card_message_id TEXT,               -- message ID of the card in #reports (for editing)
    thread_id       TEXT UNIQUE,        -- investigation thread, NULL until Investigate clicked
    status          TEXT NOT NULL DEFAULT 'open',  -- open|action_taken|no_action
    resolved_by     TEXT,
    concern_raised  INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_reports_guild  ON reports(guild_id, status);
CREATE INDEX IF NOT EXISTS idx_reports_thread ON reports(thread_id);

-- ── Accountability concerns (admin-only) ──────────────────────────────────────
CREATE TABLE IF NOT EXISTS concerns (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id    TEXT NOT NULL,
    user_id     TEXT NOT NULL,
    kind        TEXT NOT NULL,    -- 'denied_appeal' | 'no_action_report'
    source_id   INTEGER NOT NULL, -- appeal_id or report_id
    reason      TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'pending',  -- pending|reviewed
    reviewed_by TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_concerns_guild ON concerns(guild_id, status);

-- ── Anti-raid config + runtime state ─────────────────────────────────────────
CREATE TABLE IF NOT EXISTS raid_config (
    guild_id          TEXT    NOT NULL PRIMARY KEY,
    join_window_secs  INTEGER NOT NULL DEFAULT 10,
    join_threshold    REAL    NOT NULL DEFAULT 5.0,
    decay_rate        REAL    NOT NULL DEFAULT 0.15,
    slowmode_secs     INTEGER NOT NULL DEFAULT 30,
    raid_active       INTEGER NOT NULL DEFAULT 0,
    raid_triggered_at TEXT
);

-- ── Auto-slowmode config ──────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS slowmode_config (
    guild_id         TEXT    NOT NULL PRIMARY KEY,
    enabled          INTEGER NOT NULL DEFAULT 1,
    window_secs      INTEGER NOT NULL DEFAULT 10,
    capacity         INTEGER NOT NULL DEFAULT 20,
    excluded_channels TEXT   -- JSON array of channel ID strings
);
