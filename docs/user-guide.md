# pip — User & Moderator Guide

**pip** is the moderation and support bot for this server. It handles tickets, moderation actions, reports, and anti-raid protection.

---

## For All Members

### Submitting a Report

You can report users or messages to the moderation team. Reports are private — only staff see them.

**Slash commands:**
- `/report user <user>` — report a user, with an optional reason field (a text box opens if you leave it blank)
- `/report message <message link>` — paste a message link (right-click a message → Copy Message Link) to report it

**Right-click menus:**
- Right-click any **message** → Apps → **Report Message**
- Right-click any **user** → Apps → **Report User**

All reports go to the staff reports channel. You will receive a confirmation that your report was received.

---

### Tickets

Tickets are private threads between you and the moderation team. You can open one from a **ticket panel** posted in the server (look for a channel with buttons or a dropdown labelled something like "Open a Ticket").

Once a ticket is open, you can:
- Chat with staff inside the thread
- `/ticket close` — close your own ticket when your issue is resolved
- `/ticket add <user>` — invite another member into your ticket thread
- `/ticket remove <user>` — remove a member you added

---

### Appealing a Moderation Action

If you receive a ban or other action that is marked **appealable**, you will get a DM from pip with an **"Appeal this action"** button. Clicking it opens a form where you can explain your case.

After you submit an appeal:
- Staff will review it in a private thread
- You will receive a DM with the outcome (accepted or denied)
- If accepted on a ban, you'll receive a one-time invite link to rejoin
- If denied, you'll see a **"Report a Concern"** button if you believe the decision was unfair

There is a cooldown between appeals — you cannot resubmit immediately after a denial.

---

## For Moderators

### Moderation Commands

All moderation commands respond ephemerally (only you can see the reply). Actions are logged to the moderation log channel and recorded in the user's infraction history.

| Command | Required Permission | Description |
|---|---|---|
| `/warn <user> <reason>` | Moderate Members | Issue a warning. Optionally DMs the user. |
| `/timeout <user> <duration> <reason>` | Moderate Members | Mute a user for 60s / 5m / 10m / 1h / 1d / 1w. |
| `/untimeout <user> <reason>` | Moderate Members | Remove a timeout early. |
| `/kick <user> <reason>` | Kick Members | Kick a user from the server. |
| `/ban <user> <reason>` | Ban Members | Ban a user. Options: delete past messages, allow/disallow appeals. |
| `/unban <user_id> <reason>` | Ban Members | Unban a user by their ID. |
| `/history <user>` | Moderate Members | View a user's full infraction history. |

**Right-click shortcuts** (right-click a user → Apps):
- **Warn User** — opens a reason modal
- **Timeout User** — opens a reason + duration modal
- **Kick User** — opens a reason modal
- **Ban User** — opens a reason + delete messages + appealable modal
- **View History** — shows infraction history instantly
- **Open Ticket With User** — starts a staff-initiated ticket with the user

**Right-click message shortcuts** (right-click a message → Apps):
- **Delete & Warn** — deletes the message and warns the author in one action
- **Open Ticket From Message** — opens a ticket for the message author
- **Report Message** — same report flow as the user-facing version, but visible to staff

---

### Ticket Management

Inside any ticket thread, staff can use:

| Command | Description |
|---|---|
| `/ticket close [reason]` | Close the ticket. A modal opens if no reason is given. |
| `/ticket claim` | Assign the ticket to yourself. |
| `/ticket unclaim` | Release your claim. |
| `/ticket priority <Low/Normal/High/Urgent>` | Set ticket priority. |
| `/ticket tag add <tag>` | Add a tag to this ticket. |
| `/ticket tag remove <tag>` | Remove a tag from this ticket. |
| `/ticket note <content>` | Add a private staff-only note (not visible to the ticket opener). |
| `/ticket transfer <category>` | Move the ticket to a different category. |
| `/ticket add <user>` | Add someone to the thread. |
| `/ticket remove <user>` | Remove someone from the thread. |
| `/ticket info` | Show ticket metadata (owner, priority, tags, status, claimed by). |
| `/ticket list` | List all open tickets server-wide (up to 20). |

Tickets are **auto-closed after a period of inactivity** — pip checks every 30 minutes and closes stale threads automatically.

---

### Appeals

Inside an appeal thread, use:
- `/appeal accept <response>` — accept the appeal, notify the user, unban if applicable
- `/appeal deny <response>` — deny the appeal, notify the user with a "Report a Concern" option

After resolving, the appeal card in the appeals channel is updated and the thread is archived automatically.

---

### Reports

Reports from members appear as cards in the configured reports channel. Staff can act on them directly from the card using action buttons.

---

### Blocklist

The blocklist prevents specific users from opening tickets.

| Command | Description |
|---|---|
| `/blocklist add <user> <scope> <reason>` | Block a user. Scope: `global` or a specific category name. |
| `/blocklist remove <user> <scope>` | Remove a blocklist entry. |
| `/blocklist list` | View all blocklisted users. |

Requires **Manage Guild** permission.

---

### Role Management

| Command | Description |
|---|---|
| `/role give <user> <role>` | Give a role to a user. |
| `/role remove <user> <role>` | Remove a role from a user. |

Requires **Manage Roles** permission.

---

### Anti-Raid & Auto-Slowmode

pip monitors joins and message rates automatically.

**Raid detection** — when a burst of new (especially young) accounts join, pip raises the join score. If it crosses the configured threshold:
- All text channels are put into slowmode
- An alert is posted in the mod log with a **"Clear Raid Mode"** button
- Raid mode clears automatically once the score decays back down, or you can clear it manually

**Auto-slowmode** — when a regular channel sees a high message rate, pip scales its slowmode (5s → 15s → 30s) and removes it when traffic calms down.

Both systems are configured via `/setup raid` and `/setup slowmode`.

---

## For Administrators

### Initial Setup

Use the `/setup` command group to configure everything. All settings are per-guild.

| Subcommand | What it configures |
|---|---|
| `/setup logs` | Log channels: fallback, moderation, and chat logs |
| `/setup tickets` | Ticket parent channel and reports channel |
| `/setup mod` | Mod staff roles and DM notification toggles (warn/timeout/kick/ban) |
| `/setup appeals` | Appeals channel, concerns channel, and appeal cooldown period |
| `/setup raid` | Raid detection sensitivity (Low / Medium / High) and slowmode strength |
| `/setup slowmode` | Auto-slowmode enable/disable, message rate window, excluded channels |

Settings are saved immediately when you change them in the interactive form.

---

### Ticket Panels & Categories

**Categories** define the types of tickets users can open (e.g. "Support", "Ban Appeal", "Partnership"). Managed via `/ticket category` subcommands (create, configure, delete).

**Panels** are the interactive messages posted in a channel that members use to open tickets. Workflow:

1. `/ticket panel create` — create a panel with a title, description, colour, and layout (Buttons or Select Menu)
2. `/ticket panel publish` — post the panel to a channel, optionally limiting it to specific categories
3. `/ticket panel configure` — edit an existing panel's appearance
4. `/ticket panel list` — see all panels and whether they're published
5. `/ticket panel delete` — remove a panel

Tags for categorising tickets are managed via `/ticket tag list` (requires Manage Guild).
