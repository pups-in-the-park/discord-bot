-- When auto_add_staff (001) is off, notify_staff governs whether the
-- alert-channel card gets a role ping. Default 1 preserves the old behaviour
-- (alert posts were always preceded by a ping).
ALTER TABLE ticket_types ADD COLUMN notify_staff INTEGER NOT NULL DEFAULT 1;
