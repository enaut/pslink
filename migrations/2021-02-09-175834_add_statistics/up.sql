-- Your SQL goes here

CREATE TABLE clicks
(
    id INTEGER PRIMARY KEY NOT NULL,
    link INT NOT NULL,
    created_at TIMESTAMP NOT NULL,

    FOREIGN KEY
    (link)
       REFERENCES links
    (id)
);