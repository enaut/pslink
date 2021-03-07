-- Your SQL goes here

ALTER TABLE users ADD COLUMN role INTEGER DEFAULT 1 NOT NULL;

UPDATE users SET role=2 where id is 1;

