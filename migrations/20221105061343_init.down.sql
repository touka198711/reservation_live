-- Add down migration script here
DROP EXTENSION btree_gist;
DROP SCHEMA rsvp CASCADE;
-- TODO: consider to create a role for the application