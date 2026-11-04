-- Add comment column to links table
ALTER TABLE links ADD COLUMN comment TEXT NOT NULL DEFAULT '';
