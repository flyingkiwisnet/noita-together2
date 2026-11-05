-- LC1 stable identity and bounded high-water fencing. No transfer accounting.
CREATE TABLE world_lineages (
 run_id uuid NOT NULL REFERENCES runs(id), account_id uuid NOT NULL REFERENCES accounts(id),
 device_id text NOT NULL, revision integer NOT NULL DEFAULT 0 CHECK(revision>=0),
 current_world_id uuid, PRIMARY KEY(run_id,account_id,device_id)
);
CREATE TABLE world_creation_intents (
 world_id uuid PRIMARY KEY, run_id uuid NOT NULL REFERENCES runs(id),
 account_id uuid NOT NULL REFERENCES accounts(id), device_id text NOT NULL,
 association jsonb NOT NULL, proof_hash text NOT NULL, desktop_id uuid NOT NULL REFERENCES sessions(id),
 base_world_id uuid, base_revision integer NOT NULL, created_at timestamptz NOT NULL
);
CREATE INDEX world_creation_owner ON world_creation_intents(run_id,account_id,device_id);
CREATE TABLE world_associations (
 world_id uuid PRIMARY KEY, run_id uuid NOT NULL REFERENCES runs(id),
 preparation_id uuid NOT NULL UNIQUE,
 account_id uuid NOT NULL REFERENCES accounts(id), device_id text NOT NULL,
 association jsonb NOT NULL, proof_hash text NOT NULL,
 high_water integer NOT NULL CHECK(high_water>0), attempt_key text NOT NULL,
 generation integer NOT NULL DEFAULT 0 CHECK(generation>=0), owner_attempt integer,
 attachment_id uuid REFERENCES attachments(id), desktop_id uuid NOT NULL REFERENCES sessions(id),
 connection_id text NOT NULL, supersede_after timestamptz,
 actual_seed text, terminal_reason text, created_at timestamptz NOT NULL,
 UNIQUE(run_id,world_id)
);
CREATE INDEX world_associations_run ON world_associations(run_id,account_id,device_id);
INSERT INTO schema_migrations(version) VALUES(116);
