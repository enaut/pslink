-- Add migration script here

ALTER TABLE users 
ADD COLUMN language Text NOT NULL DEFAULT "en";