-- Isolated G3 only. Stable discovery identity and durable collection metadata.
ALTER TABLE g3_runs ADD COLUMN discovery_sequence bigserial;
ALTER TABLE g3_runs ADD COLUMN room_name text NOT NULL DEFAULT '' CHECK(octet_length(room_name)<=80);
ALTER TABLE g3_runs ADD COLUMN created_at timestamptz NOT NULL DEFAULT now();
ALTER TABLE g3_runs ADD COLUMN statistics_expires_at timestamptz;
ALTER TABLE g3_runs ADD COLUMN statistics_deleted_at timestamptz;
CREATE UNIQUE INDEX g3_run_discovery ON g3_runs(discovery_sequence);
CREATE UNIQUE INDEX g3_run_discovery_pair ON g3_runs(id,discovery_sequence);
ALTER TABLE g3_access ADD COLUMN discovery_sequence bigint;
UPDATE g3_access a SET discovery_sequence=r.discovery_sequence FROM g3_runs r WHERE r.id=a.run_id;
ALTER TABLE g3_access ALTER COLUMN discovery_sequence SET NOT NULL;
ALTER TABLE g3_access ADD FOREIGN KEY(run_id,discovery_sequence) REFERENCES g3_runs(id,discovery_sequence);
CREATE UNIQUE INDEX g3_access_principal ON g3_access(principal_id,discovery_sequence);
CREATE FUNCTION g3_access_discovery_identity() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN SELECT discovery_sequence INTO STRICT NEW.discovery_sequence FROM g3_runs WHERE id=NEW.run_id; RETURN NEW; END $$;
CREATE TRIGGER g3_access_discovery_identity BEFORE INSERT OR UPDATE ON g3_access FOR EACH ROW EXECUTE FUNCTION g3_access_discovery_identity();
UPDATE g3_runs g SET room_name=r.name,created_at=n.created_at FROM rooms r,runs n WHERE r.id=g.room_id AND n.id=g.id;
UPDATE g3_runs g SET statistics_expires_at=s.expires FROM (SELECT run_id,max(expires_at) AS expires FROM g3_statistics GROUP BY run_id) s WHERE s.run_id=g.id;
INSERT INTO schema_migrations(version) VALUES(105);
