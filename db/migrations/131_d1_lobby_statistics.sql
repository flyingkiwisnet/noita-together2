ALTER TABLE rooms ADD COLUMN public_stats boolean NOT NULL DEFAULT false;
ALTER TABLE rooms ADD COLUMN stats_locator uuid NOT NULL DEFAULT gen_random_uuid();
CREATE UNIQUE INDEX rooms_stats_locator ON rooms(stats_locator);
CREATE INDEX rooms_stats_discovery ON rooms(stats_locator) WHERE deleted_at IS NULL AND visibility='public';
CREATE TABLE d1_runs(
 run_id uuid PRIMARY KEY REFERENCES runs(id),room_id uuid NOT NULL REFERENCES rooms(id),
 run_key uuid NOT NULL UNIQUE DEFAULT gen_random_uuid(),started_at timestamptz NOT NULL,
 ended_at timestamptz,expires_at timestamptz,purged_at timestamptz,
 revision bigint NOT NULL DEFAULT 1,event_count integer NOT NULL DEFAULT 0,complete boolean NOT NULL DEFAULT true
);
CREATE INDEX d1_runs_latest ON d1_runs(room_id,started_at DESC,run_id);
CREATE INDEX d1_runs_expiry ON d1_runs(expires_at) WHERE expires_at IS NOT NULL;
CREATE TABLE d1_players(
 run_id uuid NOT NULL REFERENCES d1_runs(run_id) ON DELETE CASCADE,
 principal_id uuid NOT NULL REFERENCES accounts(id),row_key uuid NOT NULL DEFAULT gen_random_uuid(),
 display_name text NOT NULL,hearts bigint NOT NULL DEFAULT 0,orbs bigint NOT NULL DEFAULT 0,
 steve_kills bigint NOT NULL DEFAULT 0,deaths bigint NOT NULL DEFAULT 0,wins bigint NOT NULL DEFAULT 0,
 gold_deposited bigint NOT NULL DEFAULT 0,gold_withdrawn bigint NOT NULL DEFAULT 0,
 items_deposited bigint NOT NULL DEFAULT 0,items_claimed bigint NOT NULL DEFAULT 0,
 PRIMARY KEY(run_id,principal_id),UNIQUE(run_id,row_key)
);
CREATE INDEX d1_players_page ON d1_players(run_id,lower(display_name),row_key);
CREATE TABLE d1_events(
 run_id uuid NOT NULL REFERENCES d1_runs(run_id) ON DELETE CASCADE,world_token text NOT NULL,
 event_id text NOT NULL,principal_id uuid NOT NULL,device_id text NOT NULL,kind text NOT NULL,
 event_key text NOT NULL,original jsonb NOT NULL,counted boolean NOT NULL,
 PRIMARY KEY(run_id,world_token,event_id)
);
CREATE UNIQUE INDEX d1_events_original ON d1_events(run_id,world_token,kind,event_key) WHERE counted;
INSERT INTO schema_migrations(version) VALUES(131);
