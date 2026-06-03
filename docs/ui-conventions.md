# UI conventions

How pip builds Discord interfaces, so new features stay consistent. The bot runs on
**serenity `next` + poise `serenity-next`**, which support **components v2** including
`Label`-wrapped inputs and **selects/checkboxes inside modals** (inbound and outbound).

## Pick the right surface

| Surface | Use it for | How |
| --- | --- | --- |
| **CV2 container/card** | Anything shown to users or staff: report cards, ticket cards, DMs, logs. Persistent, rich, can hold buttons/selects, works in DMs. | `ui::Container::new(vec![...])` + `ui::send`/`ui::edit`/`ui::update`. The default. |
| **Modal (components v2)** | A one-shot capture-and-submit form: a reason, an appeal, a close note, a report (checkboxes + reason), a DB-prefilled edit. Can hold `Label`-wrapped text inputs **and** selects/checkbox groups. | `serenity::CreateModal` with `CreateModalComponent::Label(...)`, or the typed `ui::Modal` kit / `#[derive(Cv2Modal)]`. Open with `util::modal_response` (commands) or `ui::open_modal` / `ci.create_response(..Modal(..))` (components). |
| **Ephemeral CV2 form** | Live config where each change saves immediately (`/setup`) — no submit button. | Ephemeral CV2 message whose select/toggle handlers persist on change. Distinct from a modal's single submit. |
| **Context menu / slash / panel button** | Entry points. Context menus for "act on this user/message"; slash for explicit commands; panel buttons/selects for self-service (open ticket). | poise `#[command]` / panel components. |

Rule of thumb: **need free-text or a focused form → modal; need persistent UI or live config → CV2 message.**

## Modals (components v2)

Every modal field is a `Label` wrapping the actual component — serenity `next` only
deserializes that shape. Don't hand-build legacy action-row text inputs.

- **Text input:** `util::modal_input(label, custom_id, paragraph, required, placeholder, value)`
  returns a `CreateModalComponent::Label`. Read back with `ui::read_text` / `util::modal_field`.
- **Checkbox group / select:** `CreateLabel::checkbox_group(label, CreateCheckboxGroup::new(id, options))`
  or `CreateLabel::select_menu(...)`. Read back with `ui::read_checkbox_group` / `ui::read_multi_select`.
- **DB-prefilled edit modals:** `#[derive(Cv2Modal)]` — struct field values become defaults
  outbound; `from_submission` reads edits back. Emits `Label`-wrapped fields.

Inbound modal data is `mi.data.components: FixedArray<ModalComponent>`; always read via the
`ui::read_*` helpers, never by indexing.

## Custom-id vocabulary

All routing goes through `src/ids.rs`. Every builder (`cid_*` / `CID_*`) has a matching
parser (`parse_*`) so encode/decode can't drift. Custom ids are ≤100 chars; encode routing
+ args (`feature:action:id`). Add new ids in pairs and cover them with the round-trip test.

## Colour semantics

Moderation surfaces use a consistent accent (`context::colours`):

| Meaning | Colour |
| --- | --- |
| Ban | `RED` |
| Kick / general action | `ORANGE` |
| Warn / timeout | `YELLOW` |
| Untimeout / unban / success | `GREEN` |
| Info / neutral | `BLURPLE` / `GREY` |

`moderation::view::log_action` derives title + colour from the action `kind`; mirror that
when adding actions.

## Member-facing copy

Action DMs (`moderation::service::send_action_dm` + `ModActionDm`) lead with the outcome in
natural language, name the guild, and only show a reason when one was given. Add new
member-facing notices as `ModActionDm` variants rather than ad-hoc strings.

## Logging volume (future: webhooks)

Mod/event logs currently post as normal bot messages via `send_message`. This is fine at
current volume. If chat/event logging becomes high-volume, switch those streams to a cached
**webhook per log stream** (custom name/avatar, higher throughput) — kept as a deliberate
future upgrade, not built now.
