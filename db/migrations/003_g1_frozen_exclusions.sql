ALTER TABLE runs ADD COLUMN excluded_roster uuid[] NOT NULL DEFAULT '{}';
INSERT INTO schema_migrations(version) VALUES (3);
