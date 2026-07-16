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
- `/ticket close` — close your own ticket when your issue is resolved. (If a
  staff member opened the ticket for you, only staff can close it.)
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

The **ticket card** at the top of each thread has the quick controls: **Claim**
(shows "Claimed by @you" and disables until `/ticket unclaim`), **Close**, and a
**priority dropdown** (staff only). If the category has a staff-alert channel, a
card is posted there for each new ticket with a **🎟️ Join Ticket** button — it
adds you to the thread, or just takes you there if you're already in — and a
**Jump to Ticket** link for staff who can already see the thread.

Inside any ticket thread, staff can also use:

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

When a member appeals, a card with **Accept** / **Deny** buttons is posted to the
appeals channel and to the private review thread. Staff resolve the appeal by
clicking either button, which opens a short modal to enter the response sent to the
member:
- **Accept** — accept the appeal, notify the user, unban if applicable.
- **Deny** — deny the appeal, notify the user with a "Report a Concern" option.

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

**Start with `/setup overview`.** It opens a dashboard showing every configuration
area with a ✅ (ready) or ⚠️ (needs attention) marker, and a button to jump straight
into each one — you never have to memorise command names. The dashboard also flags
when no panel is **published** yet, which the ticket system can't work without —
ticket threads open in the channel each panel is published to.

All settings are per-guild and save immediately when you change them in the form.

| Area (button on the dashboard) | What it configures |
|---|---|
| **Categories** (`/ticket category manage`) | The ticket types members can open |
| **Panels** (`/ticket panel manage`) | The messages members click to open a ticket — tickets open in the panel's channel, **required for tickets** |
| **Reports channel** (`/setup tickets`) | Where reported messages and users appear for staff review |
| **Mod staff** (`/setup mod`) | Mod staff roles and DM notification toggles (warn/timeout/kick/ban) |
| **Appeals & concerns** (`/setup appeals`) | Appeals channel, concerns channel, and the appeal waiting period |
| **Anti-raid** (`/setup raid`) | Raid detection sensitivity (Low / Medium / High) and slowmode strength |
| **Auto-slowmode** (`/setup slowmode`) | Auto-slowmode enable/disable, message-rate thresholds |
| **Log channels** (`/setup logs`) | Fallback, moderation, chat, and ticket logs |

---

### Ticket Categories

Run **`/ticket category manage`** (or the **Categories** button on the dashboard) to
open the category hub:

- **Create Category** — one modal capturing the name, emoji, description, and accent
  colour (a **dropdown**). Submitting lands you straight on the category's config
  panel, where everything else lives.
- Each category in the list has an inline **Configure** button (with very long lists
  this collapses into a dropdown).

The config panel is split into three tabs, swapped in place:

- **Overview** — basic info (label, emoji, colour, description — the same modal as
  Create, prefilled), the welcome message, the thread-name pattern, and **Delete**
  (with confirmation). Each row shows the current value next to its **Edit** button.
- **Behaviour** — **Staff Roles** plus two toggles that decide how they hear
  about new tickets: **Auto-Add Staff** (ping the roles inside each new thread —
  the mention also pulls them into the private thread), and **Notify Staff**
  (when Auto-Add is off, ping the roles alongside the alert-channel card; with
  both off, staff aren't pinged at all). Also the staff-alert channel, a
  **Ticket Channel** override (where this category's tickets open — leave unset
  to use its panel's channel; set it for **staff-only categories** that aren't
  on any panel and are opened purely via the context menus), and limits (max
  open per user — set **0 for unlimited** — and auto-close).

  Tickets opened *for* a member by staff (via the context menus) can only be
  closed by staff — the member can't dismiss a verification, conduct, or
  welfare thread themselves.
- **Questions** — the intake-form questions (up to 5). There's no on/off switch:
  the form shows whenever the category has questions, and is skipped when it has
  none.

**Intake form questions** support four types, chosen from the **Add a question**
dropdown on the Questions tab:

- **Short answer** / **Paragraph** — free text.
- **Dropdown** — the member picks one of your choices.
- **Checkboxes** — the member picks any number of your choices.

Picking a type opens a single modal with everything the question needs — its label,
whether it's required, an optional placeholder hint, and (for dropdown/checkbox
questions) the choices, one per line. Every question has an inline **Edit** button to
reopen the same modal prefilled.

### Ticket Panels

Run **`/ticket panel manage`** (or the **Panels** button on the dashboard) to open the
panel hub:

1. **Create Panel** — give it a title (description and colour optional).
2. In the panel's form: edit basics, choose the **layout** (buttons up to 5, or a
   dropdown for any number), and tick which **categories** appear using the
   multi-select.
3. **Publish** by selecting a channel — pip posts the panel there. Changing categories
   or basics afterwards updates the live message automatically. Re-publishing to the
   same channel edits the existing message in place.

Tags for categorising tickets are managed via `/ticket tag list` (requires Manage Guild).
