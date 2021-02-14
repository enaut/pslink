-- Your SQL goes here

ALTER TABLE users ADD COLUMN role INTEGER DEFAULT 1 NOT NULL;
CREATE TABLE usersnew
(
    id INTEGER PRIMARY KEY NOT NULL,
    username VARCHAR NOT NULL,
    email VARCHAR NOT NULL,
    password VARCHAR NOT NULL,
    role INTEGER DEFAULT 1 NOT NULL,

    UNIQUE(username, email),
    FOREIGN KEY
        (role) REFERENCES user_roles
        (id)
);

INSERT INTO usersnew
SELECT *
FROM users;
DROP TABLE users;

ALTER TABLE usersnew
RENAME TO users;

UPDATE users SET role=2 where id is 1;

