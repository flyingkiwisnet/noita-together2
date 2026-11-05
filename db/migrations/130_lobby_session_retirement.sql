-- Keep saved-world join consent while removing superseded sessions from the lobby.
ALTER TABLE memberships ADD COLUMN roster_retired boolean NOT NULL DEFAULT false;
CREATE INDEX memberships_current_roster ON memberships(room_id,desktop_id) WHERE NOT roster_retired;
INSERT INTO schema_migrations(version) VALUES(130);
