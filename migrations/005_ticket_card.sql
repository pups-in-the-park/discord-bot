-- Message id of the in-thread CV2 ticket card, so claim/unclaim/priority
-- changes can re-render the card. NULL for tickets opened before this column
-- existed (their cards simply aren't refreshed).
ALTER TABLE tickets ADD COLUMN card_message_id TEXT;
