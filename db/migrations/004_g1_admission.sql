-- Old draft rooms have no recorded publication consent; do not publish them.
ALTER TABLE rooms ADD COLUMN visibility text NOT NULL DEFAULT 'private' CHECK(visibility IN ('public','private'));
CREATE TABLE room_admissions (room_id uuid NOT NULL REFERENCES rooms(id), account_id uuid NOT NULL REFERENCES accounts(id), PRIMARY KEY(room_id,account_id));
INSERT INTO schema_migrations(version) VALUES(4);
