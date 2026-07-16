-- Overrides the panel channel — lets a staff-only category exist without
-- sitting on any member-facing panel.
ALTER TABLE ticket_types ADD COLUMN ticket_channel_id TEXT;

-- Differs from owner_id when staff open a ticket *for* a member; those tickets
-- are staff-close-only. NULL on old rows (treated as owner-opened).
ALTER TABLE tickets ADD COLUMN opened_by TEXT;
