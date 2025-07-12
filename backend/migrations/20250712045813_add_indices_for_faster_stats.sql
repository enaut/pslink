-- Add migration script here

CREATE INDEX IF NOT EXISTS idx_clicks_link_created_at ON clicks(link, created_at);
CREATE INDEX IF NOT EXISTS idx_clicks_created_at ON clicks(created_at);