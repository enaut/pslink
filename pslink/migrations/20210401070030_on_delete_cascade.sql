-- Add migration script here
PRAGMA foreign_keys = off;


CREATE TABLE new_clicks (
    id INTEGER PRIMARY KEY NOT NULL,
    link INT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    FOREIGN KEY (link) REFERENCES links (id) ON DELETE CASCADE
);

INSERT INTO
    new_clicks
SELECT
    *
FROM
    clicks;


DROP TABLE clicks;
ALTER TABLE
    new_clicks RENAME TO clicks;
    
PRAGMA foreign_keys = on;