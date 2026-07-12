# UI conventions

How pip builds Discord interfaces, so new features stay consistent. The bot runs on
**serenity `next` + poise `serenity-next`**, which support both classic **embeds**
and **components v2 (CV2)** — including `Label`-wrapped inputs and selects/checkboxes
inside modals.

## The one hard rule

**A single message is either embeds or CV2 — never both.** Setting the
`IS_COMPONENTS_V2` flag (`ui::CV2_FLAG`) on a message forbids `content` and
`embeds`; conversely an embed message cannot hold CV2 containers, sections, or
inline buttons. So every surface commits to one paradigm before you build it.

## Pick the paradigm: interactivity first, then weight

Ask two questions, in order:

1. **Does the surface have controls the user acts on (buttons / selects), or does
   it re-render in place in response to an interaction?** If yes → **CV2 container**.
2. If no (read-only), **does structured chrome actually earn its keep** — discrete
   labelled facts (field grid), an author line, a thumbnail/avatar, an
   auto-localized timestamp, an accent colour that carries meaning? If yes →
   **embed**. If it's just a sentence or two of prose → **plain markdown text**.

| Answer | Paradigm | Why |
| --- | --- | --- |
| **Has controls / edits in place** | **CV2 container** | Inline buttons/selects, sections with accessories, separators, galleries, edit-in-place. Tickets, reports, appeals, panels, **config (`/setup`)**. |
| **Read-only + structured chrome pays off** | **Embed** | Colour bar, field grid, author, footer, auto-localized timestamp, thumbnail/image for free. Mod/event logs, member DMs, history & list views, blocklist/mod-action receipts. |
| **Read-only + just a line or two of prose** | **Plain markdown** | A normal message `content` — no embed, no container. Command confirmations & acks (“Ticket unclaimed”, “Priority set to High”, role given), `/ping`, `/help`, `/concern stats`. Don't wrap a single sentence in chrome. |
| **Free-text / numeric capture** | **Modal** | A focused form that submits once. Always *launched from* something else — an ✏️ Edit button on a CV2 card, or a command / context menu. Never a standalone surface. |

**The rule of thumb:** controls → container; a labelled/structured read-only record
→ embed; a plain confirmation → plain text. When in doubt between embed and plain
text, ask whether you'd fill in more than a `description` — if not, it's plain text.

> **Kit caveat.** Every sender in `src/ui/respond.rs` (`send`, `edit`,
> `respond_ephemeral`, `update`, `slash_respond`, …) hard-codes `CV2_FLAG`, so the
> `ui::` kit *only* emits CV2 containers. Plain-text surfaces go through poise
> (`ctx.send(CreateReply::default().content(..))`) or a raw `content` message —
> **not** through `ui::`. Never route a no-control surface through `ui::` just
> because it's convenient; that's what produced the old text-in-a-container cards.

Config (`/setup`) is **not** a separate paradigm — it's a CV2 surface (usually
ephemeral). How its values are entered is covered under "Data entry on CV2
surfaces" below.

Corollaries of the binary rule:
- **An embed never carries buttons.** Need a button anywhere on the card → it's
  CV2. Use **markdown links** (not link buttons) for jump-to-message / external
  links inside an embed.
- **A read-only surface is never a CV2 container.** Don't reach for a container
  just to draw a coloured box around static text — it's an embed if structured
  chrome pays off, otherwise plain markdown. (The terminal states of an
  interactive card — a resolved appeal, a dismissed report — are the one
  exception: once a message is sent as CV2, Discord forbids editing it into a
  plain/embed message, so an edit-in-place outcome stays a container.)
- **A member notice that the member must act on** (e.g. an "Appeal" button) is no
  longer read-only → it's a CV2 card. If the action lives elsewhere (a slash
  command, a context menu), the notice stays a read-only embed.

## Embeds (read-only)

Built with `serenity::CreateEmbed`, sent via
`serenity::CreateMessage::new().embed(embed)` (or `.embeds(..)`, up to 10). Accent
colour comes from `context::colours` (see below).

Use each element for its purpose — don't overload one field with everything:

| Element | Use it for |
| --- | --- |
| `.colour(..)` | Semantic accent (see colour table). Always set it. |
| `.title(..)` | The one-line "what happened" — `🔨 Member Banned · Case #42`. |
| `.url(..)` | Make the title a link (rare; e.g. link to a source message). |
| `.description(..)` | Prose / the main body. Markdown + mentions + markdown links. Up to 4096 chars. |
| `.field(name, value, inline)` | Discrete labelled facts (User, Moderator, Reason, Duration). `inline = true` lays fields out in up-to-3 columns; `inline = false` is a full-width row. Use inline for short scalars, full-width for prose like a reason. |
| `.author(..)` | The actor/subject as a small header line with icon (e.g. the member). |
| `.thumbnail(url)` | Small top-right image — typically the target user's avatar (`user.face()`). |
| `.image(url)` | Large bottom image — attachments / evidence being reported. |
| `.footer(..)` | Secondary metadata: case id, source, bot tag. |
| `.timestamp(..)` | When it happened. Discord renders it in each viewer's locale — prefer this over writing a time into a field. |

Guidance:
- **Fields are for scalars, description is for prose.** A long reason goes in a
  full-width field or the description, not split across inline fields.
- **One embed = one event.** Don't pack multiple unrelated events into one embed's
  fields; send separate embeds (or messages).
- **No interactivity.** The moment you want a button, switch to CV2.

## CV2 containers (interactive)

Built with the typed `ui::` kit (`src/ui/`) and put on the wire with `ui::send` /
`ui::edit` / `ui::update` / `ui::respond_ephemeral` / `ui::slash_respond`. The kit
serializes to the exact CV2 JSON; don't hand-write component JSON at call sites.

| Element | Builder | Use it for |
| --- | --- | --- |
| **Container** | `ui::Container::new(vec![..]).accent(colour.0)` | The outer card. `.accent` draws the coloured side bar (mirror the embed colour table). |
| **Text** | `ui::text("..")` | A block of markdown. The CV2 equivalent of an embed description. |
| **Section** | `ui::Section::new(vec![text..], accessory)` | Text paired with **one** accessory — a button or a thumbnail — sitting to its right. Use when a control acts on the text beside it (a row's claim/close button). |
| **Separator** | `ui::separator(divider, Spacing::Small\|Large)` | Visual break between groups. `divider = true` draws a line. CV2 has no field grid — separators do the structural work. |
| **Action row** | `ui::action_row(vec![Button/select..])` | A row of buttons or a single select, when the controls aren't tied to one specific line of text. |
| **Button** | `ui::Button::new(custom_id, label, style)` / `ui::Button::link(url, label)` | Actions. Styles: Primary/Secondary/Success/Danger/Link. `custom_id` routes through `src/ids.rs`. |
| **Selects** | `ui::StringSelect` / `RoleSelect` / `ChannelSelect` / `UserSelect` / `MentionableSelect` | Choosing among options inline (config, filters, assignment). |

Guidance:
- **Build the body from `ui::text` + separators**, not by abusing fields — CV2 has
  no automatic field columns.
- **Edit in place.** Interactive cards re-render via `ui::edit` / `ui::update`
  rather than posting a fresh message, so state lives in one card.
- **There is no footer/timestamp chrome.** If a CV2 card needs a timestamp, write
  a Discord `<t:unix:R>` markdown timestamp into a `ui::text` line.

### Hubs & status dashboards

Multi-step admin areas use a **hub**: an ephemeral CV2 card that lists what exists
— one `Section` per item with an inline **Configure button** accessory (falling back
to a `StringSelect` when the list is too big for the component cap) — plus a Create
button, and re-renders in place (`ui::update`) as you drill in and back out.
`/setup overview`, `/ticket category manage`, and `/ticket panel manage` are the
references. Conventions:

- A **status dashboard** (`/setup overview`) is a hub-of-hubs: one `Section` per area
  with a ✅/⚠️ status line and a quick-link **button** accessory that opens that area.
  Surface the one setting that blocks everything (a published panel) explicitly.
- **Open vs back vs drill-in.** A button launched from *another* card opens the hub as
  a **new** ephemeral (`ui::respond_ephemeral`); a Back/select button *inside* the hub
  updates **in place** (`ui::update`). Give them separate custom ids (e.g. `…:open`
  vs `…:back`) so they don't fight over which behaviour applies.
- Keep a card under ~38 components (Discord's limit is 40) — fold related controls
  into one form rather than spreading across several.
- A config card with many settings splits into **tabs**: a row of buttons under the
  header (active tab `Primary` + disabled, others `Secondary`) that swap the body in
  place, with each setting as a `Section` row — current value as text, its edit
  control as the accessory. The category config panel (`cat:cfg:{id}:tab:{tab}`) is
  the reference.
- **A modal submit can never open another modal** (Discord rejects it). Collect a
  flow's choice *first* with an inline select/button, then open one modal that
  gathers everything else — don't design two-modal wizards. The add-question select
  and the single category create modal are the references.

### Data entry on CV2 surfaces

A CV2 card is the persistent display of state; how a value is *edited* depends on
its kind. Keep this split consistent across config and any other editable card:

| Value kind | How it's edited | Builder |
| --- | --- | --- |
| **Standalone choice** — channel / role / user / string select, or an on/off toggle that applies on its own | **Inline**, persists immediately on change. Pre-fill with the current value so state is visible when the card opens. | `ui::ChannelSelect` / `RoleSelect` / `StringSelect` with `.default(..)` / `.defaults(..)`; or a toggle `ui::Button` that flips style/label. |
| **Free text / number** | **Never typed inline** (CV2 has no inline text field). Show the current value as a `ui::text` line and put an **✏️ Edit button (`ButtonStyle::Secondary`) beside it that opens a modal**. Re-render the card on submit. | `ui::Button::new(cid, "Change …", ButtonStyle::Secondary).emoji("✏️")` → `ui::open_modal` → `ui::update`. |
| **Choice that's part of a multi-field form** — a category picked *together with* a required reason, etc. | **In a modal**, alongside the text it belongs with, committed atomically on submit. Not inline. | `Label`-wrapped select inside the same `ui::Modal` as the text inputs. See "Inline select vs select-in-a-modal". |

Rule of thumb: **a choice is a select/toggle; a value you type is an Edit button +
modal.** Don't invent a "type into the card" flow — text always routes through a
modal. (`/setup` is the reference implementation of this split.)

### Inline select vs select-in-a-modal

Both are select menus; the question is *where the choice lives*. The deciding fact:
**a modal is frozen between open and submit — its fields can't re-render and are
read only once, on submit. An inline select fires an interaction on every change,
so the card can update live.**

Use an **inline (edit-in-place) select** when:
- The choice is a **standalone setting that applies immediately** — it's valid on
  its own, no other input needed (`/setup` channel/role pickers, raid sensitivity).
- The choice **drives what else is shown** — picking it should reveal or change
  other controls live. Only inline can do this; a modal can't re-render.
- You want the change to take effect with **one fewer click** (no dialog), and to
  stay visible/persistent on the card.

Use a **select inside a modal** when:
- The choice is **one field of a form submitted together** — it only makes sense
  alongside other inputs (a category + a required reason) and should commit
  **atomically on submit**, not piecemeal.
- The form **already opens a modal for free text**, and the choice belongs with it
  — gather both in one dialog instead of splitting across a card and a popup.
- The user should be able to **confirm or cancel the whole set at once** (all-or-
  nothing), rather than each pick persisting the instant it's made.

## Modals (components v2)

A modal is the only way to capture free text. It is never a standalone surface —
it's launched from an ✏️ Edit button on a CV2 card, or from a command / context
menu. Every modal field is a `Label` wrapping the actual component — serenity
`next` only deserializes that shape. Don't hand-build legacy action-row text inputs.

A modal holds a small number of fields and submits once, so it's for **focused
capture**, not a sprawling form. The field types:

| Field | Use it for | Builder / read-back |
| --- | --- | --- |
| **Text input** | Free text or numbers — a reason, a note, a numeric threshold. `Short` for one line, `Paragraph` for prose. | `util::modal_input(label, id, paragraph, required, placeholder, value)` / `ui::TextInput`. Read: `ui::read_text` / `util::modal_field`. |
| **Select menu (single)** | Pick **one** from a list — a category, a severity. Best when there are many options or each needs a description/emoji. `min=1, max=1` (the `StringSelect` default). | `CreateLabel::select_menu(..)` / `ui::StringSelect` (or an entity select) in a `Label`. Read: `ui::read_multi_select` (one value). |
| **Select menu (multi)** | Pick **several** from a longer list. Set `max_values > 1`. | same as above with `.max_values(n)`. Read: `ui::read_multi_select`. |
| **Checkbox group** | A handful of independent on/off flags — "select all that apply". Constrain with min/max selected. | `CreateLabel::checkbox_group(label, CreateCheckboxGroup::new(id, options))`. Read: `ui::read_checkbox_group`. |

There is **no native "radio" component.** A single mutually-exclusive choice is
either a single-value select menu (`max=1`) or a checkbox group capped at one
(`max_values = 1`). Prefer a **select** when the list is long or options need
descriptions; prefer a **checkbox group capped at one** when there are only a few
short options you want shown all at once.

- **DB-prefilled edit modals:** `#[derive(Cv2Modal)]` — struct field values become
  defaults outbound; `from_submission` reads edits back. Emits `Label`-wrapped fields.
- Open with `util::modal_response` (commands) or `ui::open_modal` /
  `ci.create_response(..Modal(..))` (components).

Inbound modal data is `mi.data.components: FixedArray<ModalComponent>`; always read
via the `ui::read_*` helpers, never by indexing.

## Colour semantics (shared)

Both embeds (`.colour`) and CV2 containers (`.accent`) use the same palette from
`context::colours`. Same meaning whichever paradigm you're in:

| Meaning | Colour |
| --- | --- |
| Ban | `RED` |
| Kick / general action | `ORANGE` |
| Warn / timeout | `YELLOW` |
| Untimeout / unban / success | `GREEN` |
| Info / neutral | `BLURPLE` / `GREY` |

Derive title + colour from the action `kind` in one place and mirror it when adding
actions, so the same action always looks the same.

## Custom-id vocabulary

All routing goes through `src/ids.rs`. Every builder (`cid_*` / `CID_*`) has a
matching parser (`parse_*`) so encode/decode can't drift. Custom ids are ≤100
chars; encode routing + args (`feature:action:id`). Add new ids in pairs and cover
them with the round-trip test.

## Member-facing copy

Action DMs (`moderation::service::send_action_dm` + `ModActionDm`) lead with the
outcome in natural language, name the guild, and only show a reason when one was
given. They are read-only notices → **embeds**. Add new member-facing notices as
`ModActionDm` variants rather than ad-hoc strings.

## Logging volume (future: webhooks)

Mod/event logs currently post as normal bot messages. This is fine at current
volume. If chat/event logging becomes high-volume, switch those streams to a cached
**webhook per log stream** (custom name/avatar, higher throughput) — a deliberate
future upgrade, not built now.
