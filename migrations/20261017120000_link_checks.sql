-- History of health checks for link targets.
CREATE TABLE link_checks (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    link        INTEGER NOT NULL REFERENCES links(id) ON DELETE CASCADE,
    checked_at  DATETIME NOT NULL,
    status      TEXT NOT NULL,           -- 'ok' | 'warning' | 'failed'
    http_status INTEGER,                 -- final status code, NULL on network error
    error       TEXT,                    -- e.g. "timeout", "connection failed"
    final_url   TEXT,                    -- after redirects
    duration_ms INTEGER NOT NULL,
    embeddable  BOOLEAN                  -- NULL if unknown
);

CREATE INDEX idx_link_checks_link_checked ON link_checks(link, checked_at DESC);
