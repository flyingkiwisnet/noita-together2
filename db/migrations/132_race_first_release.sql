ALTER TABLE g3_runs DROP CONSTRAINT g3_runs_mode_check;
ALTER TABLE g3_runs ADD CONSTRAINT g3_runs_mode_check CHECK(mode IN ('core','race'));
CREATE TABLE race_runs(
 run_id uuid PRIMARY KEY REFERENCES runs(id),room_id uuid NOT NULL REFERENCES rooms(id),
 run_key uuid NOT NULL UNIQUE DEFAULT gen_random_uuid(),config jsonb NOT NULL,
 registry_revision text NOT NULL,started_at timestamptz NOT NULL,deadline_at timestamptz,
 ended_at timestamptz,end_reason text NOT NULL DEFAULT '',expires_at timestamptz,purged_at timestamptz,
 revision bigint NOT NULL DEFAULT 1,event_count integer NOT NULL DEFAULT 0
);
CREATE INDEX race_runs_latest ON race_runs(room_id,started_at DESC,run_id);
CREATE INDEX race_runs_deadline ON race_runs(deadline_at) WHERE ended_at IS NULL;
CREATE INDEX race_runs_expiry ON race_runs(expires_at) WHERE purged_at IS NULL;
CREATE TABLE race_entrants(
 run_id uuid NOT NULL REFERENCES race_runs(run_id),account_id uuid NOT NULL REFERENCES accounts(id),
 world_token text NOT NULL,device_id text NOT NULL,row_key uuid NOT NULL DEFAULT gen_random_uuid(),
 display_name text NOT NULL,status text NOT NULL DEFAULT 'racing' CHECK(status IN ('racing','finished','dead','dnf')),
 evaluation jsonb NOT NULL,last_sequence bigint NOT NULL DEFAULT 0,finish_order integer,
 PRIMARY KEY(run_id,account_id),UNIQUE(run_id,world_token),UNIQUE(run_id,row_key),UNIQUE(run_id,finish_order)
);
CREATE TABLE race_receipts(
 run_id uuid NOT NULL REFERENCES race_runs(run_id),world_token text NOT NULL,event_id uuid NOT NULL,
 sequence bigint NOT NULL,original_hash text NOT NULL,result jsonb NOT NULL,
 PRIMARY KEY(run_id,world_token,event_id),UNIQUE(run_id,world_token,sequence)
);
INSERT INTO schema_migrations(version) VALUES(132);
