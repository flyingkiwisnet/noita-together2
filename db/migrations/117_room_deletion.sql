-- Retain room/run/statistics identity; deletion only withdraws lobby access.
ALTER TABLE rooms ADD COLUMN deleted_at timestamptz;
CREATE INDEX rooms_active_host ON rooms(host_account_id) WHERE deleted_at IS NULL;
INSERT INTO schema_migrations(version) VALUES (117);
