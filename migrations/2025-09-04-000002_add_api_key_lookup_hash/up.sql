-- Add deterministic lookup hash column for API keys
ALTER TABLE api_keys
    ADD COLUMN api_key_lookup_hash VARCHAR NOT NULL DEFAULT '';

-- Backfill existing rows (compute hash from nothing -> leave '')
-- (If raw keys not stored, cannot backfill; admin can force regenerate.)

-- Add index for fast lookup
CREATE INDEX IF NOT EXISTS idx_api_keys_lookup_hash ON api_keys (api_key_lookup_hash);

-- Optional: enforce uniqueness if we consider extremely low collision risk
-- ALTER TABLE api_keys ADD CONSTRAINT uq_api_keys_lookup_hash UNIQUE (api_key_lookup_hash);
