CREATE TABLE c1_room_state (
 room_id uuid PRIMARY KEY REFERENCES rooms(id) ON DELETE CASCADE,
 sequence bigint NOT NULL DEFAULT 0 CHECK(sequence>=0),
 rules_revision bigint NOT NULL DEFAULT 1 CHECK(rules_revision BETWEEN 1 AND 4294967295)
);
CREATE TABLE c1_rules (
 room_id uuid NOT NULL REFERENCES c1_room_state(room_id) ON DELETE CASCADE,
 revision bigint NOT NULL CHECK(revision BETWEEN 1 AND 4294967295),
 sequence bigint NOT NULL CHECK(sequence>=0), rules jsonb NOT NULL,
 PRIMARY KEY(room_id,revision)
);
CREATE TABLE c1_events (
 run_id uuid NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
 event_id uuid NOT NULL, room_id uuid NOT NULL REFERENCES c1_room_state(room_id) ON DELETE CASCADE,
 source_principal uuid NOT NULL REFERENCES accounts(id), source_world text NOT NULL,
 sequence bigint NOT NULL CHECK(sequence>0), rules_revision bigint NOT NULL,
 digest text NOT NULL, body jsonb NOT NULL, source_state text NOT NULL CHECK(source_state IN ('pending','applied','skipped','uncertain')),
 created_at timestamptz NOT NULL DEFAULT now(), PRIMARY KEY(run_id,event_id), UNIQUE(room_id,sequence),
 FOREIGN KEY(room_id,rules_revision) REFERENCES c1_rules(room_id,revision)
);
CREATE TABLE c1_deliveries (
 run_id uuid NOT NULL,event_id uuid NOT NULL,recipient_principal uuid NOT NULL REFERENCES accounts(id),recipient_world text NOT NULL,
 sequence bigint NOT NULL CHECK(sequence>0),state text NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','applied','skipped','uncertain','terminal')),
 reason text NOT NULL DEFAULT '', issued boolean NOT NULL DEFAULT false, PRIMARY KEY(run_id,event_id,recipient_principal,recipient_world),
 FOREIGN KEY(run_id,event_id) REFERENCES c1_events(run_id,event_id) ON DELETE CASCADE
);
CREATE INDEX c1_recipient_work ON c1_deliveries(run_id,recipient_principal,recipient_world,sequence) WHERE state IN ('pending','uncertain');
CREATE TABLE c1_life (
 run_id uuid NOT NULL REFERENCES runs(id) ON DELETE CASCADE,principal_id uuid NOT NULL REFERENCES accounts(id),world_token text NOT NULL,
 state text NOT NULL CHECK(state IN ('playing','respawning','terminal','won')),death_event uuid NOT NULL,rules_revision bigint NOT NULL,
 PRIMARY KEY(run_id,principal_id,world_token),FOREIGN KEY(run_id,death_event) REFERENCES c1_events(run_id,event_id)
);
INSERT INTO schema_migrations(version) VALUES(124);
