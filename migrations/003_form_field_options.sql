-- Intake form question types: select (dropdown) and checkbox fields store their
-- choices here as a JSON array of strings. NULL for plain text fields.
ALTER TABLE form_fields ADD COLUMN options TEXT;
