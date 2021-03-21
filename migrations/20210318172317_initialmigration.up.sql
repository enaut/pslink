-- Add up migration script here
DROP TABLE IF EXISTS __diesel_schema_migrations;

-- Add migration script here
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY NOT NULL,
    username VARCHAR NOT NULL,
    email VARCHAR NOT NULL,
    password VARCHAR NOT NULL,
    role INTEGER DEFAULT 1 NOT NULL,
    UNIQUE(username),
    UNIQUE(email)
);

CREATE TABLE IF NOT EXISTS links (
    id INTEGER PRIMARY KEY NOT NULL,
    title VARCHAR NOT NULL,
    target VARCHAR NOT NULL,
    code VARCHAR NOT NULL,
    author INT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    FOREIGN KEY (author) REFERENCES users (id),
    UNIQUE (code)
);

CREATE TABLE IF NOT EXISTS clicks (
    id INTEGER PRIMARY KEY NOT NULL,
    link INT NOT NULL,
    created_at TIMESTAMP NOT NULL,
    FOREIGN KEY (link) REFERENCES links (id)
);