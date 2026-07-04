-- Tickets now open in the channel of the panel they were triggered from; the
-- global "ticket parent channel" is gone.
ALTER TABLE guild_config DROP COLUMN ticket_channel_id;

-- One panel per category: dedupe existing links, preferring a published panel
-- (message_id set), then the oldest panel. panel_ticket_types is a rowid table,
-- so rowid addressing works. Panels that lose a category here keep a stale
-- button on their live message until republished — harmless, since the panel
-- button path opens tickets in the interaction's own channel and never reads
-- this table.
DELETE FROM panel_ticket_types
WHERE rowid NOT IN (
    SELECT keep_rowid FROM (
        SELECT ptt.rowid AS keep_rowid,
               ROW_NUMBER() OVER (
                   PARTITION BY ptt.ticket_type_id
                   ORDER BY (p.message_id IS NOT NULL) DESC, p.id ASC
               ) AS rn
        FROM panel_ticket_types ptt
        JOIN panels p ON p.id = ptt.panel_id
    ) WHERE rn = 1
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_panel_types_one_panel_per_type
    ON panel_ticket_types(ticket_type_id);
