-- Candidate successor upgrade. Accepted G1/G2 migration runners do not load this.
ALTER TABLE runs ADD COLUMN review_minutes integer NOT NULL DEFAULT 30 CHECK(review_minutes BETWEEN 5 AND 60);
UPDATE runs SET review_until=created_at+interval '30 minutes';
ALTER TABLE runs ALTER COLUMN review_until SET NOT NULL;
ALTER TABLE inspected_releases ADD COLUMN authorization_uuid uuid NOT NULL DEFAULT gen_random_uuid() UNIQUE;
CREATE TABLE preparations (
 id uuid PRIMARY KEY, run_id uuid NOT NULL REFERENCES runs(id), desktop_id uuid NOT NULL REFERENCES sessions(id),
 desktop_epoch uuid NOT NULL, service_epoch uuid NOT NULL, run_epoch uuid NOT NULL, config_revision integer NOT NULL,
 state text NOT NULL CHECK(state IN ('issued','consumed','loaded','cancelled','expired')),
 issued_at timestamptz NOT NULL, expires_at timestamptz NOT NULL, CHECK(expires_at>issued_at)
);
CREATE UNIQUE INDEX live_preparation ON preparations(run_id,desktop_id) WHERE state IN ('issued','consumed','loaded');
CREATE INDEX preparation_expiry ON preparations(expires_at) WHERE state IN ('issued','consumed');
ALTER TABLE attachments DROP CONSTRAINT attachments_run_id_desktop_id_key;
CREATE UNIQUE INDEX live_attachment ON attachments(run_id,desktop_id) WHERE revoked_at IS NULL;
ALTER TABLE attachments ADD COLUMN preparation_id uuid UNIQUE REFERENCES preparations(id);
INSERT INTO schema_migrations(version) VALUES(8);
ALTER TABLE inspected_releases ADD COLUMN start_fingerprint text CHECK(length(start_fingerprint)=64);
