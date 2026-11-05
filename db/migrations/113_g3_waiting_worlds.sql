-- Development rejoin namespaces only; no live/frozen migration.
ALTER TABLE preparations ADD COLUMN consumed_operation jsonb;
CREATE TABLE waiting_worlds (
 id uuid PRIMARY KEY, preparation_id uuid NOT NULL UNIQUE REFERENCES preparations(id),
 run_id uuid NOT NULL REFERENCES runs(id),account_id uuid NOT NULL REFERENCES accounts(id),device_id text NOT NULL,
 intent jsonb NOT NULL,frozen_config jsonb NOT NULL,proof_hash text NOT NULL,proof_ciphertext text NOT NULL,
 attachment_id uuid NOT NULL REFERENCES attachments(id),desktop_id uuid NOT NULL REFERENCES sessions(id),
 generation integer NOT NULL DEFAULT 1 CHECK(generation>0),context_id text NOT NULL,
 offline boolean NOT NULL DEFAULT false,created_at timestamptz NOT NULL,
 UNIQUE(run_id,account_id,device_id)
);
CREATE TABLE waiting_operations (
 account_id uuid NOT NULL,device_id text NOT NULL,original_session_id uuid NOT NULL,request_id text NOT NULL,
 kind text NOT NULL CHECK(kind IN ('enroll','recover','offline')),original jsonb NOT NULL,fingerprint text NOT NULL,
 waiting_id uuid NOT NULL REFERENCES waiting_worlds(id),effect_id text NOT NULL,effect_generation integer NOT NULL,
 created_at timestamptz NOT NULL,expires_at timestamptz NOT NULL,
 PRIMARY KEY(account_id,device_id,original_session_id,request_id)
);
CREATE INDEX waiting_operations_retention ON waiting_operations(waiting_id,expires_at);
INSERT INTO schema_migrations(version) VALUES(113);
