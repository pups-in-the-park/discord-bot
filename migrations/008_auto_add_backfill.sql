-- auto_add_staff (001, DEFAULT 0) was dormant but now gates the in-thread
-- staff ping, which used to be unconditional. Backfill to 1 so existing
-- categories don't silently stop notifying staff.
UPDATE ticket_types SET auto_add_staff = 1;
