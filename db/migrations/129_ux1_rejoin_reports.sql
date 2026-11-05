ALTER TABLE rejoin_participants ADD COLUMN original_observation jsonb;
INSERT INTO schema_migrations(version) VALUES(129);
