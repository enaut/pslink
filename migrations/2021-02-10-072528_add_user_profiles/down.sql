-- This file should undo anything in `up.sql`

CREATE TABLE usersold
(
    id INTEGER PRIMARY KEY NOT NULL,
    username VARCHAR NOT NULL,
    email VARCHAR NOT NULL,
    password VARCHAR NOT NULL,

    UNIQUE(username, email)
);

INSERT INTO usersold
SELECT id,username,email,password
FROM users;
DROP TABLE users;

ALTER TABLE usersold
RENAME TO users;

