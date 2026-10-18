-- Migration to aggregate clicks into daily buckets and add total_clicks to links

-- Clean up any orphaned clicks for links deleted before foreign keys were enforced
DELETE FROM clicks WHERE link NOT IN (SELECT id FROM links);

ALTER TABLE links ADD COLUMN total_clicks INTEGER NOT NULL DEFAULT 0;

UPDATE links SET total_clicks = COALESCE(
    (SELECT COUNT(*) FROM clicks WHERE clicks.link = links.id), 0
);

CREATE TABLE clicks_daily (
    link INTEGER NOT NULL,
    date DATE NOT NULL,
    count INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (link, date),
    FOREIGN KEY (link) REFERENCES links (id) ON DELETE CASCADE
);

INSERT INTO clicks_daily (link, date, count)
SELECT link, date(created_at), COUNT(*)
FROM clicks
GROUP BY link, date(created_at);

DROP INDEX IF EXISTS idx_clicks_link_created_at;
DROP INDEX IF EXISTS idx_clicks_created_at;

DROP TABLE clicks;

ALTER TABLE clicks_daily RENAME TO clicks;
