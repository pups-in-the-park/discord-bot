-- Backlog features: richer reports, accountable concerns, blocklist expiry/scopes.

-- Reports: which profile parts a user/profile report flags (comma-joined codes).
ALTER TABLE reports ADD COLUMN profile_parts TEXT;

-- Concerns: attribute each concern to the moderator whose decision it challenges,
-- and record when an admin reviewed it.
ALTER TABLE concerns ADD COLUMN target_moderator_id TEXT;
ALTER TABLE concerns ADD COLUMN reviewed_at TEXT;

-- Blocklist: per-entry expiry (NULL = permanent).
ALTER TABLE blocklist ADD COLUMN expires_at TEXT;

-- Blocklist scope vocabulary: legacy `global`/`{category}` -> typed scopes
-- (`tickets`, `ticket:{name}`, plus the new `reports`/`concerns`/`appeals`).
UPDATE blocklist SET scope = 'tickets' WHERE scope = 'global';
UPDATE blocklist SET scope = 'ticket:' || scope
    WHERE scope NOT IN ('tickets', 'reports', 'concerns', 'appeals')
      AND scope NOT LIKE 'ticket:%';
