# pip — Developer Guide

## Overview

**pip** is a single-guild Discord moderation bot written in Rust. It uses [Serenity](https://github.com/serenity-rs/serenity) (the `next` branch) as the Discord API client and [Poise](https://github.com/serenity-rs/poise) (`serenity-next` branch) as the slash-command framework — pinned as git dependencies, requires rustc ≥ 1.95. This stack supports **components v2**, including selects and checkbox groups inside modals. Persistence is SQLite via [sqlx](https://github.com/launchbadr/sqlx) with versioned migrations under `migrations/`. Slash commands are dispatched by poise; component and modal routing is custom (see each feature's `router.rs`).

> **Building UI?** See [ui-conventions.md](ui-conventions.md) for when to use cards vs modals vs ephemeral forms, the modal-v2 helpers, custom-id rules, and colour semantics.

---

## Project Structure

```
src/
├── main.rs              # Entry point, framework setup, background tasks
├── config.rs            # Config struct, loaded from config.toml
├── context.rs           # Shared types: BotData, BotError, component ID helpers
├── db.rs                # All database access (sqlx, SQLite)
├── events.rs            # Gateway event handler (logs, raid, auto-slowmode)
├── raid.rs              # In-memory raid scoring and per-channel token bucket
├── util.rs              # Shared helpers (CV2 builders, close logic, modals, etc.)
├── commands/
│   ├── mod.rs           # Re-exports
│   ├── general.rs       # /ping, /help
│   ├── setup.rs         # /setup subcommands
│   ├── mod_cmd.rs       # /warn, /timeout, /untimeout, /kick, /ban, /unban, /history, /appeal
│   ├── blocklist.rs     # /blocklist add/remove/list
│   ├── role_cmd.rs      # /role give/remove
│   ├── report_cmd.rs    # /report user/message
│   └── tickets/
│       ├── mod.rs       # Re-exports, /ticket command group
│       ├── ticket.rs    # Ticket subcommands (close, claim, priority, tag, note, transfer, add, remove, info, list)
│       ├── panel.rs     # /ticket panel subcommands
│       └── category.rs  # /ticket category subcommands
└── handlers/
    ├── mod.rs
    ├── context_menu.rs  # Right-click message/user context menu commands
    ├── components.rs    # Button/select-menu interaction dispatcher
    └── modals.rs        # Modal submission dispatcher
migrations/              # SQLx migration SQL files
config.example.toml      # Config template
```

---

## Configuration

Copy `config.example.toml` to `config.toml` and fill in:

```toml
[bot]
token = "YOUR_BOT_TOKEN_HERE"
environment = "development"   # "development" | "production"
log_level = "info"            # tracing filter string

[database]
path = "./pip.db"

[guild]
id = 123456789012345678       # The single guild this bot serves
```

The bot is single-guild by design — commands are registered in the configured guild only (`poise::builtins::register_in_guild`).

**Environment behaviour:** In `development`, unhandled errors are printed verbatim in Discord. In `production`, a short reference code is shown to the user and the full error is logged server-side with `tracing::error!`.

---

## Running

```
cargo run
```

SQLx migrations run automatically on startup via `db.migrate()`. The database file is created if it does not exist.

**Required bot intents:**
- `GUILDS`, `GUILD_MESSAGES`, `MESSAGE_CONTENT`, `GUILD_MEMBERS`, `GUILD_MODERATION`, `DIRECT_MESSAGES`

---

## Core Abstractions

### `BotData` (`src/context.rs`)

The single shared state object, wrapped in `Arc<BotData>` and available via `ctx.data()` in every command.

```rust
pub struct BotData {
    pub db: Database,
    pub config: AppConfig,
    pub raid: RaidState,
}
```

### `BotError` (`src/context.rs`)

The unified error type. `BotError::User(msg)` means a human-readable error that is shown directly to the user. All other variants produce a reference code in production.

```rust
impl BotError {
    pub fn user(msg: impl Into<String>) -> Self { ... }
}
```

### Component ID scheme (`src/context.rs`)

All interactive component IDs (buttons, selects, modals) are constructed with helper functions that encode their parameters into a structured string, e.g.:

```
m:appeal:<infraction_id>:<guild_id>
m:close_modal:<ticket_id>
m:rep_user:<user_id>
```

`handlers/components.rs` and `handlers/modals.rs` parse these prefixes to dispatch interactions to the correct handler. The `cid_*` functions in `context.rs` are the single source of truth for these IDs.

---

## Interaction Dispatch

Poise handles slash commands automatically. The two interaction types it ignores — component clicks and modal submissions — are dispatched manually in `main.rs`:

```rust
serenity::Interaction::Component(ci) => handlers::components::dispatch(ctx, data, ci).await,
serenity::Interaction::Modal(mi)     => handlers::modals::dispatch(ctx, data, mi).await,
```

`components::dispatch` and `modals::dispatch` match on the `custom_id` prefix and call the appropriate handler.

---

## Discord Components V2 (CV2)

pip uses Discord's Components V2 API throughout for rich embedded layouts. The CV2 builder helpers live in `util.rs`:

| Helper | Wraps |
|---|---|
| `cv2_container(children, color)` | A coloured container block |
| `cv2_text(content)` | A text display component |
| `cv2_separator(divider, spacing)` | A visual divider |
| `cv2_action_row(buttons)` | A row of buttons |
| `cv2_button(id, label, style, emoji)` | An individual button |
| `send_cv2(http, channel, components)` | Sends a CV2 message |
| `edit_cv2(http, channel, message, components)` | Edits an existing CV2 message |

CV2 messages are sent with `flags: 32768` (IS_COMPONENTS_V2) — Serenity does not expose this natively so it is applied via raw JSON manipulation in `send_cv2`.

---

## Database (`src/db.rs`)

All queries go through the `Database` struct. The underlying pool is `sqlx::SqlitePool`. Tables include:

- `guild_config` — per-guild channel and flag settings
- `mod_config` — per-guild moderation settings (DM toggles, staff roles, appeal cooldown)
- `infractions` — all moderation actions (warn, timeout, kick, ban, unban, blocklist, etc.)
- `tickets`, `ticket_types`, `ticket_members`, `ticket_messages`, `ticket_notes`, `ticket_tags`, `tag_definitions` — full ticket system state
- `panels`, `panel_ticket_types` — ticket panel configuration
- `appeals` — ban/action appeals with status tracking
- `reports` — member-submitted reports
- `blocklist` — per-scope ticket blocklist
- `raid_config`, `slowmode_config` — anti-raid and auto-slowmode settings

Migrations are plain SQL files in `migrations/` managed by sqlx.

---

## Anti-Raid (`src/raid.rs`)

`RaidState` is held in `BotData` and is purely in-memory (not persisted between restarts). It has two independent subsystems:

### Join Score (Raid Detection)

An exponential-decay score is incremented on every `GuildMemberAddition` event. New accounts (< 1 day old) add 2.0, accounts under a week add 1.5, older accounts add 1.0. The score decays continuously between joins.

When the score crosses the configured threshold:
1. `raid_active` is set in the DB
2. All text/news channels get slowmode applied
3. A CV2 alert card is posted to the mod log with a "Clear Raid Mode" button

Auto-recovery: on every subsequent join while raid is active, the score is re-evaluated — if it decays below 20% of the threshold, raid mode clears automatically.

Sensitivity presets configure `decay_rate` and `threshold`:
- Low: decay 0.05, threshold 8.0
- Medium: decay 0.15, threshold 5.0
- High: decay 0.30, threshold 3.0

### Channel Pressure (Auto-Slowmode)

Each text channel has an independent token bucket (`ChannelPressure`). On each message, a token is consumed and refilled over the configured window. Pressure (0.0–1.0) maps to slowmode tiers:

| Pressure | Slowmode |
|---|---|
| < 0.50 | 0s (off) |
| 0.50–0.74 | 5s |
| 0.75–0.89 | 15s |
| ≥ 0.90 | 30s |

A 5-second cooldown prevents spam-editing the channel. Excluded channel IDs are stored in `slowmode_config`.

---

## Ticket System

### Lifecycle

1. A member interacts with a panel button/select → `components::dispatch` handles it
2. A form modal opens (fields configured per category in `ticket_types`)
3. On submission → `util::execute_open` creates the ticket row, spins up a private thread, and posts the opening CV2 card
4. The thread is the ticket — all messages are saved to `ticket_messages` via the `Message` event handler in `events.rs` (calls `db.touch_ticket` and `db.save_message`)
5. Staff manage the ticket via `/ticket` subcommands or buttons on the card
6. On close → `util::execute_close` marks the ticket closed, archives the thread, and logs the closure
7. The auto-close background task (30-minute poll, `main.rs`) calls `db.get_stale_tickets()` and runs `execute_close` for each

### Panels

Panels are CV2 messages in a configured channel. `util::build_panel_cv2` constructs the message from a `Panel` and a list of `TicketType`s. When the panel button/select is interacted with, the component handler opens the relevant form modal.

---

## Background Tasks

One background task is spawned at startup:

**`auto_close_task`** — polls every 30 minutes, fetches stale tickets from the DB, and closes them with the reason "Auto-closed due to inactivity". The stale threshold is configured per guild.

---

## Error Handling

- `BotError::User(msg)` — shown verbatim to the user as an ephemeral message
- Other errors — in dev: printed with `{:?}`; in prod: an 8-char UUID reference code is generated, logged with `tracing::error!`, and the code is shown to the user
- Event handler errors are logged with `tracing::warn!` but do not crash the bot
- DM send failures are silently ignored (users may have DMs closed)

---

## Key `util.rs` Functions

| Function | Purpose |
|---|---|
| `execute_close` | Close a ticket: update DB, archive thread, log |
| `execute_open` | Open a ticket: create DB row, create thread, post card |
| `validate_target` | Ensure the target user isn't the bot, the command author, or a higher role |
| `is_mod_staff` | Check if a user has a configured mod staff role |
| `require_mod_staff` | `is_mod_staff` as a guard that returns `BotError::User` on failure |
| `format_infraction_history` | Format a list of infractions into display lines |
| `format_duration` | Format seconds into a human-readable string (e.g. "1 hour 30 minutes") |
| `modal_response` | Respond to an interaction by showing a modal |
| `build_panel_cv2` | Build the CV2 component tree for a ticket panel |
| `build_open_modal` | Build the modal for opening a ticket with its category's form fields |
| `build_setup_*_form` | Build the CV2 setup forms for each `/setup` subcommand |
