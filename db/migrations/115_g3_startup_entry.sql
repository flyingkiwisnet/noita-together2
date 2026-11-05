-- Development rejoin composition only. Rows represent actual mutations, never offers.
CREATE TABLE startup_entry_effects (
 pending_id uuid PRIMARY KEY,preparation_id uuid NOT NULL UNIQUE,run_id uuid NOT NULL REFERENCES runs(id),
 account_id uuid NOT NULL REFERENCES accounts(id),device_id text NOT NULL,world_token text NOT NULL,
 scope jsonb NOT NULL,identity jsonb NOT NULL,proof_hash text NOT NULL,
 state text NOT NULL CHECK(state IN ('activated','cancelled')),attachment_id uuid,
 pending_generation integer,waiting_generation integer,created_at timestamptz NOT NULL,
 UNIQUE(run_id,world_token)
);
CREATE TABLE startup_entry_operations (
 account_id uuid NOT NULL,device_id text NOT NULL,run_id uuid NOT NULL REFERENCES runs(id),
 original_session_id uuid NOT NULL,request_id text NOT NULL,kind text NOT NULL CHECK(kind IN ('entryActivate','entryCancel')),
 original jsonb NOT NULL,fingerprint text NOT NULL,pending_id uuid NOT NULL REFERENCES startup_entry_effects(pending_id),
 effect_id text NOT NULL,created_at timestamptz NOT NULL,expires_at timestamptz NOT NULL,
 PRIMARY KEY(account_id,device_id,original_session_id,request_id)
);
CREATE INDEX startup_entry_operation_scope ON startup_entry_operations(account_id,device_id,run_id,expires_at);
ALTER TABLE preparations ADD COLUMN retired_by uuid REFERENCES startup_entry_effects(pending_id);
ALTER TABLE waiting_worlds ADD COLUMN retired_by uuid REFERENCES startup_entry_effects(pending_id);
ALTER TABLE waiting_worlds DROP CONSTRAINT waiting_worlds_run_id_account_id_device_id_key;
CREATE UNIQUE INDEX waiting_current_lineage ON waiting_worlds(run_id,account_id,device_id) WHERE retired_by IS NULL AND NOT offline;
INSERT INTO schema_migrations(version) VALUES(115);
