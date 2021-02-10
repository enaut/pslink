-- Your SQL goes here
CREATE TABLE user_roles
(
    id INTEGER PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,

    FOREIGN KEY
    (link)
       REFERENCES users
    (id)
);