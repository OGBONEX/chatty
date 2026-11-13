-- Additional user-controlled profile presentation fields.
ALTER TABLE profiles
    ADD COLUMN banner_url TEXT,
    ADD COLUMN status_text TEXT;
