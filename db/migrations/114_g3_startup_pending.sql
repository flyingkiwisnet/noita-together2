-- Provisional development startup reservation, never applied to live CF05.
CREATE TABLE startup_pending (
 id uuid PRIMARY KEY,preparation_id uuid NOT NULL UNIQUE REFERENCES preparations(id),
 run_id uuid NOT NULL REFERENCES runs(id),account_id uuid NOT NULL REFERENCES accounts(id),device_id text NOT NULL,
 intent jsonb NOT NULL,frozen_config jsonb NOT NULL,bound_seed text,proof_hash text NOT NULL,proof_ciphertext text NOT NULL,
 desktop_id uuid NOT NULL REFERENCES sessions(id),generation integer NOT NULL DEFAULT 1 CHECK(generation>0),
 offline boolean NOT NULL DEFAULT false,waiting_id uuid REFERENCES waiting_worlds(id),waiting_generation integer,created_at timestamptz NOT NULL
);
CREATE TABLE startup_pending_operations (
 account_id uuid NOT NULL,device_id text NOT NULL,original_session_id uuid NOT NULL,request_id text NOT NULL,
 original jsonb NOT NULL,fingerprint text NOT NULL,pending_id uuid NOT NULL REFERENCES startup_pending(id),
 effect_id text NOT NULL,effect_generation integer NOT NULL,created_at timestamptz NOT NULL,expires_at timestamptz NOT NULL,
 PRIMARY KEY(account_id,device_id,original_session_id,request_id)
);
CREATE INDEX startup_pending_retention ON startup_pending_operations(pending_id,expires_at);
INSERT INTO schema_migrations(version) VALUES(114);
