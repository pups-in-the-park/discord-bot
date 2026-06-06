//! Custom-id vocabulary for components and modals.
//!
//! Discord round-trips an interaction's `custom_id` (≤100 chars) back to us on
//! click/submit, so we encode routing + arguments into it. Every id has a
//! **builder** (`cid_*` / `CID_*`) used when emitting a component and a matching
//! **parser** (`parse_*`) used when one comes back, so handlers never hand-split
//! `id.split(':')` and the encode/decode shapes can't drift apart.
#![allow(dead_code)]

// ── Generic tail parsers ───────────────────────────────────────────────────────

/// `prefix` + one trailing `i64`, e.g. `t:claim:42` → `42`.
fn tail_i64(id: &str, prefix: &str) -> Option<i64> {
    id.strip_prefix(prefix)?.parse().ok()
}

/// `prefix` + one trailing `u64`, e.g. `m:warn:123` → `123`.
fn tail_u64(id: &str, prefix: &str) -> Option<u64> {
    id.strip_prefix(prefix)?.parse().ok()
}

// ── Ticket / panel IDs ──────────────────────────────────────────────────────────

pub fn cid_panel_btn(ticket_type_id: i64) -> String {
    format!("p:btn:{ticket_type_id}")
}
pub fn parse_panel_btn(id: &str) -> Option<i64> {
    tail_i64(id, "p:btn:")
}

pub const CID_PANEL_SELECT: &str = "p:sel";

pub fn cid_ctx_type_select(target_user_id: u64) -> String {
    format!("ctx:sel:{target_user_id}")
}
pub fn parse_ctx_type_select(id: &str) -> Option<u64> {
    tail_u64(id, "ctx:sel:")
}

pub fn cid_open_modal(ticket_type_id: i64) -> String {
    format!("m:open:{ticket_type_id}")
}
pub fn parse_open_modal(id: &str) -> Option<i64> {
    tail_i64(id, "m:open:")
}

pub fn cid_ctx_open_modal(ticket_type_id: i64, target_user_id: u64) -> String {
    format!("m:ctx:{ticket_type_id}:{target_user_id}")
}
/// → `(ticket_type_id, target_user_id)`
pub fn parse_ctx_open_modal(id: &str) -> Option<(i64, u64)> {
    let rest = id.strip_prefix("m:ctx:")?;
    let (a, b) = rest.split_once(':')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

pub const CID_CLOSE_BTN: &str = "t:close";

pub fn cid_claim_btn(ticket_id: i64) -> String {
    format!("t:claim:{ticket_id}")
}
pub fn parse_claim_btn(id: &str) -> Option<i64> {
    tail_i64(id, "t:claim:")
}

pub fn cid_confirm_close(ticket_id: i64) -> String {
    format!("t:cfm:{ticket_id}")
}
pub fn parse_confirm_close(id: &str) -> Option<i64> {
    tail_i64(id, "t:cfm:")
}

pub const CID_CANCEL_CLOSE: &str = "t:can";

pub fn cid_priority_select(ticket_id: i64) -> String {
    format!("t:pri:{ticket_id}")
}
pub fn parse_priority_select(id: &str) -> Option<i64> {
    tail_i64(id, "t:pri:")
}

pub fn cid_close_modal(ticket_id: i64) -> String {
    format!("m:close:{ticket_id}")
}
pub fn parse_close_modal(id: &str) -> Option<i64> {
    tail_i64(id, "m:close:")
}

pub fn cid_form_field_add_modal(type_id: i64, style: &str, required: bool) -> String {
    format!("m:ff:add:{type_id}:{style}:{required}")
}
/// → `(type_id, style, required)`
pub fn parse_form_field_add_modal(id: &str) -> Option<(i64, String, bool)> {
    let rest = id.strip_prefix("m:ff:add:")?;
    let mut parts = rest.splitn(3, ':');
    let type_id = parts.next()?.parse().ok()?;
    let style = parts.next()?.to_string();
    let required = parts.next()?.parse().ok()?;
    Some((type_id, style, required))
}

// ── Setup IDs ───────────────────────────────────────────────────────────────────

pub const CID_SETUP_LOG_CHANNELS: &str = "setup:log";
pub const CID_SETUP_TICKET: &str = "setup:ticket";
pub const CID_SETUP_MOD_STAFF: &str = "setup:mod";
pub const CID_SETUP_APPEALS: &str = "setup:appeals";
pub const CID_SETUP_RAID: &str = "setup:raid";
pub const CID_SETUP_SLOWMODE: &str = "setup:slowmode";

// ── Moderation IDs ──────────────────────────────────────────────────────────────

pub fn cid_mod_warn_modal(target_id: u64) -> String {
    format!("m:warn:{target_id}")
}
pub fn parse_mod_warn_modal(id: &str) -> Option<u64> {
    tail_u64(id, "m:warn:")
}

pub fn cid_mod_timeout_modal(target_id: u64) -> String {
    format!("m:timeout:{target_id}")
}
pub fn parse_mod_timeout_modal(id: &str) -> Option<u64> {
    tail_u64(id, "m:timeout:")
}

pub fn cid_mod_kick_modal(target_id: u64) -> String {
    format!("m:kick:{target_id}")
}
pub fn parse_mod_kick_modal(id: &str) -> Option<u64> {
    tail_u64(id, "m:kick:")
}

pub fn cid_mod_ban_modal(target_id: u64) -> String {
    format!("m:ban:{target_id}")
}
pub fn parse_mod_ban_modal(id: &str) -> Option<u64> {
    tail_u64(id, "m:ban:")
}

pub fn cid_mod_dw_modal(msg_id: u64, chan_id: u64, author_id: u64) -> String {
    format!("m:dw:{msg_id}:{chan_id}:{author_id}")
}
/// → `(msg_id, chan_id, author_id)`
pub fn parse_mod_dw_modal(id: &str) -> Option<(u64, u64, u64)> {
    parse_triple_u64(id, "m:dw:")
}

// ── Appeal IDs ──────────────────────────────────────────────────────────────────

/// Button in DM to open the appeal modal; encodes infraction_id and guild_id.
pub fn cid_appeal_btn(infraction_id: i64, guild_id: u64) -> String {
    format!("ap:btn:{infraction_id}:{guild_id}")
}
/// → `(infraction_id, guild_id)`
pub fn parse_appeal_btn(id: &str) -> Option<(i64, u64)> {
    let rest = id.strip_prefix("ap:btn:")?;
    let (a, b) = rest.split_once(':')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

/// Appeal modal itself.
pub fn cid_appeal_modal(infraction_id: i64, guild_id: u64) -> String {
    format!("m:ap:{infraction_id}:{guild_id}")
}
/// → `(infraction_id, guild_id)`
pub fn parse_appeal_modal(id: &str) -> Option<(i64, u64)> {
    let rest = id.strip_prefix("m:ap:")?;
    let (a, b) = rest.split_once(':')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

/// Accept/Deny button on a staff appeal card or thread intro.
pub fn cid_appeal_resolve(appeal_id: i64, accept: bool) -> String {
    format!("ap:res:{}:{appeal_id}", if accept { "a" } else { "d" })
}
/// → `(appeal_id, accept)`
pub fn parse_appeal_resolve(id: &str) -> Option<(i64, bool)> {
    let rest = id.strip_prefix("ap:res:")?;
    let (a, b) = rest.split_once(':')?;
    let accept = match a {
        "a" => true,
        "d" => false,
        _ => return None,
    };
    Some((b.parse().ok()?, accept))
}

/// Modal collecting the moderator's response when resolving an appeal via button.
pub fn cid_appeal_resolve_modal(appeal_id: i64, accept: bool) -> String {
    format!("m:apr:{}:{appeal_id}", if accept { "a" } else { "d" })
}
/// → `(appeal_id, accept)`
pub fn parse_appeal_resolve_modal(id: &str) -> Option<(i64, bool)> {
    let rest = id.strip_prefix("m:apr:")?;
    let (a, b) = rest.split_once(':')?;
    let accept = match a {
        "a" => true,
        "d" => false,
        _ => return None,
    };
    Some((b.parse().ok()?, accept))
}

// ── Concern IDs ─────────────────────────────────────────────────────────────────

/// "Report a Concern" button; `kind`: "appeal" or "report"; `source_id`: appeal_id or report_id.
pub fn cid_concern_btn(kind: &str, source_id: i64, guild_id: u64) -> String {
    format!("con:btn:{kind}:{source_id}:{guild_id}")
}
/// → `(kind, source_id, guild_id)`
pub fn parse_concern_btn(id: &str) -> Option<(String, i64, u64)> {
    parse_concern(id, "con:btn:")
}

pub fn cid_concern_modal(kind: &str, source_id: i64, guild_id: u64) -> String {
    format!("m:con:{kind}:{source_id}:{guild_id}")
}
/// → `(kind, source_id, guild_id)`
pub fn parse_concern_modal(id: &str) -> Option<(String, i64, u64)> {
    parse_concern(id, "m:con:")
}

pub fn cid_concern_reviewed(concern_id: i64) -> String {
    format!("con:rev:{concern_id}")
}
pub fn parse_concern_reviewed(id: &str) -> Option<i64> {
    tail_i64(id, "con:rev:")
}

// ── Report IDs ──────────────────────────────────────────────────────────────────

/// Report message context menu modal.
pub fn cid_report_msg_modal(msg_id: u64, chan_id: u64, author_id: u64) -> String {
    format!("m:rep:{msg_id}:{chan_id}:{author_id}")
}
/// → `(msg_id, chan_id, author_id)`
pub fn parse_report_msg_modal(id: &str) -> Option<(u64, u64, u64)> {
    parse_triple_u64(id, "m:rep:")
}

/// Report user modal.
pub fn cid_report_user_modal(target_id: u64) -> String {
    format!("m:rusr:{target_id}")
}
pub fn parse_report_user_modal(id: &str) -> Option<u64> {
    tail_u64(id, "m:rusr:")
}

/// "Investigate" button on a report card.
pub fn cid_report_investigate(report_id: i64) -> String {
    format!("rep:inv:{report_id}")
}
pub fn parse_report_investigate(id: &str) -> Option<i64> {
    tail_i64(id, "rep:inv:")
}

/// "Dismiss" button on a report card.
pub fn cid_report_dismiss(report_id: i64) -> String {
    format!("rep:dis:{report_id}")
}
pub fn parse_report_dismiss(id: &str) -> Option<i64> {
    tail_i64(id, "rep:dis:")
}

/// "Take Action" select inside a report investigation thread.
pub fn cid_report_action_select(report_id: i64) -> String {
    format!("rep:act:{report_id}")
}
pub fn parse_report_action_select(id: &str) -> Option<i64> {
    tail_i64(id, "rep:act:")
}

/// "Dismiss" button inside a report investigation thread.
pub fn cid_report_thread_dismiss(report_id: i64) -> String {
    format!("rep:tdis:{report_id}")
}
pub fn parse_report_thread_dismiss(id: &str) -> Option<i64> {
    tail_i64(id, "rep:tdis:")
}

/// Ephemeral "which action" select shown after clicking Take Action.
pub fn cid_report_action_chosen(report_id: i64) -> String {
    format!("rep:action_chosen:{report_id}")
}
pub fn parse_report_action_chosen(id: &str) -> Option<i64> {
    tail_i64(id, "rep:action_chosen:")
}

/// Modal collecting the reason/params for a chosen report action.
pub fn cid_report_action_modal(action: &str, report_id: i64, target_id: u64) -> String {
    format!("m:ract:{action}:{report_id}:{target_id}")
}
/// → `(action, report_id, target_id)`
pub fn parse_report_action_modal(id: &str) -> Option<(String, i64, u64)> {
    let rest = id.strip_prefix("m:ract:")?;
    let mut p = rest.splitn(3, ':');
    let action = p.next()?.to_string();
    let report_id = p.next()?.parse().ok()?;
    let target_id = p.next()?.parse().ok()?;
    Some((action, report_id, target_id))
}

// ── Raid IDs ────────────────────────────────────────────────────────────────────

pub const CID_RAID_CLEAR: &str = "raid:clear";

// ── Form field constants ────────────────────────────────────────────────────────

pub const FORM_FIELD_BASE: &str = "ff_";
pub const CLOSE_REASON_FIELD: &str = "close_reason";
pub const REPORT_REASON_FIELD: &str = "report_reason";
pub const REPORT_PARTS_FIELD: &str = "report_parts";
pub const APPEAL_REASON_FIELD: &str = "appeal_reason";
pub const APPEAL_RESPONSE_FIELD: &str = "appeal_response";
pub const CONCERN_REASON_FIELD: &str = "concern_reason";

// ── Shared multi-arg parsers ────────────────────────────────────────────────────

fn parse_triple_u64(id: &str, prefix: &str) -> Option<(u64, u64, u64)> {
    let rest = id.strip_prefix(prefix)?;
    let mut p = rest.splitn(3, ':');
    Some((
        p.next()?.parse().ok()?,
        p.next()?.parse().ok()?,
        p.next()?.parse().ok()?,
    ))
}

/// `prefix` + `kind:source_id:guild_id` (concern button & modal share this shape).
fn parse_concern(id: &str, prefix: &str) -> Option<(String, i64, u64)> {
    let rest = id.strip_prefix(prefix)?;
    let mut p = rest.splitn(3, ':');
    let kind = p.next()?.to_string();
    let source_id = p.next()?.parse().ok()?;
    let guild_id = p.next()?.parse().ok()?;
    Some((kind, source_id, guild_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builders_round_trip_through_parsers() {
        assert_eq!(parse_panel_btn(&cid_panel_btn(7)), Some(7));
        assert_eq!(parse_claim_btn(&cid_claim_btn(7)), Some(7));
        assert_eq!(parse_open_modal(&cid_open_modal(7)), Some(7));
        assert_eq!(parse_ctx_open_modal(&cid_ctx_open_modal(7, 99)), Some((7, 99)));
        assert_eq!(parse_appeal_btn(&cid_appeal_btn(3, 50)), Some((3, 50)));
        assert_eq!(parse_appeal_modal(&cid_appeal_modal(3, 50)), Some((3, 50)));
        assert_eq!(parse_appeal_resolve(&cid_appeal_resolve(9, true)), Some((9, true)));
        assert_eq!(parse_appeal_resolve(&cid_appeal_resolve(9, false)), Some((9, false)));
        assert_eq!(
            parse_appeal_resolve_modal(&cid_appeal_resolve_modal(9, true)),
            Some((9, true))
        );
        assert_eq!(parse_mod_warn_modal(&cid_mod_warn_modal(5)), Some(5));
        assert_eq!(
            parse_mod_dw_modal(&cid_mod_dw_modal(1, 2, 3)),
            Some((1, 2, 3))
        );
        assert_eq!(
            parse_report_msg_modal(&cid_report_msg_modal(1, 2, 3)),
            Some((1, 2, 3))
        );
        assert_eq!(
            parse_concern_btn(&cid_concern_btn("appeal", 8, 60)),
            Some(("appeal".to_string(), 8, 60))
        );
        assert_eq!(
            parse_concern_modal(&cid_concern_modal("report", 8, 60)),
            Some(("report".to_string(), 8, 60))
        );
        assert_eq!(
            parse_form_field_add_modal(&cid_form_field_add_modal(4, "short", true)),
            Some((4, "short".to_string(), true))
        );
    }

    #[test]
    fn parsers_reject_wrong_prefix() {
        assert_eq!(parse_claim_btn("t:pri:1"), None);
        assert_eq!(parse_appeal_btn("ap:btn:notanumber:5"), None);
    }
}
