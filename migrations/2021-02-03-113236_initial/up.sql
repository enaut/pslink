-- Your SQL goes here
CREATE TABLE users
(
    id INTEGER PRIMARY KEY NOT NULL,
    username VARCHAR NOT NULL,
    email VARCHAR NOT NULL,
    password VARCHAR NOT NULL,

    UNIQUE(username, email)
);

CREATE TABLE links
(
    id INTEGER PRIMARY KEY NOT NULL,
    title VARCHAR NOT NULL,
    target VARCHAR NOT NULL,
    code VARCHAR NOT NULL,
    author INT NOT NULL,
    created_at TIMESTAMP NOT NULL,


    FOREIGN KEY
    (author)
       REFERENCES users
    (id),

    UNIQUE
    (code)
);