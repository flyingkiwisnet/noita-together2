CREATE TABLE c2_room_state (
 room_id uuid PRIMARY KEY REFERENCES rooms(id) ON DELETE CASCADE,
 sequence bigint NOT NULL DEFAULT 0 CHECK(sequence>=0), rules_revision bigint NOT NULL DEFAULT 1
);
CREATE TABLE c2_rules (
 room_id uuid NOT NULL REFERENCES c2_room_state(room_id) ON DELETE CASCADE,
 revision bigint NOT NULL, sequence bigint NOT NULL, rules jsonb NOT NULL, PRIMARY KEY(room_id,revision)
);
CREATE TABLE c2_encounters (
 run_id uuid NOT NULL REFERENCES runs(id) ON DELETE CASCADE, progression bigint NOT NULL CHECK(progression BETWEEN 0 AND 4294967295),
 encounter_id uuid NOT NULL UNIQUE,stage text NOT NULL DEFAULT 'locked' CHECK(stage IN ('locked','unlocked','battle')),
 revision bigint NOT NULL DEFAULT 0,projection_digest text NOT NULL DEFAULT '',PRIMARY KEY(run_id,progression)
);
CREATE TABLE c2_arena_worlds (
 encounter_id uuid NOT NULL REFERENCES c2_encounters(encounter_id) ON DELETE CASCADE,
 principal_id uuid NOT NULL, world_token text NOT NULL,arena_token uuid NOT NULL,near boolean NOT NULL DEFAULT false,needs_fresh boolean NOT NULL DEFAULT false,
 service_epoch uuid NOT NULL,session_id uuid NOT NULL,session_epoch uuid NOT NULL,attachment_id uuid NOT NULL,owner_generation bigint NOT NULL,
 near_until timestamptz NOT NULL,readiness_epoch uuid NOT NULL,picked_up boolean NOT NULL DEFAULT false,orb_count bigint NOT NULL DEFAULT 0 CHECK(orb_count BETWEEN 0 AND 4294967295),
 PRIMARY KEY(encounter_id,principal_id,world_token)
);
CREATE TABLE c2_events (
 run_id uuid NOT NULL REFERENCES runs(id) ON DELETE CASCADE,event_id uuid NOT NULL,
 room_id uuid NOT NULL REFERENCES c2_room_state(room_id),source_principal uuid NOT NULL,source_world text NOT NULL,source_device text NOT NULL,
 sequence bigint NOT NULL,rules_revision bigint NOT NULL,digest text NOT NULL,body jsonb NOT NULL,source_state text NOT NULL,
 source_reason text NOT NULL DEFAULT '',created_at timestamptz NOT NULL DEFAULT now(),PRIMARY KEY(run_id,event_id),UNIQUE(room_id,sequence)
);
CREATE TABLE c2_deliveries (
 run_id uuid NOT NULL,event_id uuid NOT NULL,recipient_principal uuid NOT NULL,recipient_world text NOT NULL,sequence bigint NOT NULL,
 state text NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','applied','skipped','uncertain','terminal')),
 recipient_progression bigint NOT NULL,reason text NOT NULL DEFAULT '',issued boolean NOT NULL DEFAULT false,PRIMARY KEY(run_id,event_id,recipient_principal,recipient_world),
 FOREIGN KEY(run_id,event_id) REFERENCES c2_events(run_id,event_id) ON DELETE CASCADE
);
CREATE INDEX c2_pending ON c2_deliveries(run_id,recipient_principal,recipient_world,sequence) WHERE state IN ('pending','uncertain');
CREATE TABLE c2_completions (
 run_id uuid NOT NULL REFERENCES runs(id) ON DELETE CASCADE,principal_id uuid NOT NULL,world_token text NOT NULL,progression bigint NOT NULL,
 event_id uuid NOT NULL,PRIMARY KEY(run_id,principal_id,world_token,progression)
);
CREATE TABLE c2_world_progress (run_id uuid NOT NULL REFERENCES runs(id) ON DELETE CASCADE,principal_id uuid NOT NULL,world_token text NOT NULL,progression bigint NOT NULL CHECK(progression BETWEEN 0 AND 4294967295),PRIMARY KEY(run_id,principal_id,world_token));
CREATE TABLE c2_world_progress_history (run_id uuid NOT NULL REFERENCES runs(id) ON DELETE CASCADE,principal_id uuid NOT NULL,world_token text NOT NULL,progression bigint NOT NULL CHECK(progression BETWEEN 0 AND 4294967295),PRIMARY KEY(run_id,principal_id,world_token,progression));
CREATE TABLE c2_completion_gaps(run_id uuid NOT NULL REFERENCES runs(id) ON DELETE CASCADE,principal_id uuid NOT NULL,world_token text NOT NULL,event_id uuid NOT NULL,reason text NOT NULL,PRIMARY KEY(run_id,principal_id,world_token));
CREATE TABLE c2_steve (
 run_id uuid NOT NULL REFERENCES runs(id) ON DELETE CASCADE,principal_id uuid NOT NULL,world_token text NOT NULL,progression bigint NOT NULL,
 event_id uuid NOT NULL,PRIMARY KEY(run_id,principal_id,world_token,progression)
);
ALTER TABLE g3_hourglass_activations DROP CONSTRAINT g3_hourglass_activations_sequence_check;
ALTER TABLE g3_hourglass_activations ALTER COLUMN sequence TYPE bigint;
ALTER TABLE g3_hourglass_activations ADD CHECK(sequence BETWEEN 1 AND 4294967295);
ALTER TABLE g3_hourglass_activations ADD COLUMN c2_body jsonb,ADD COLUMN c2_digest text,ADD COLUMN c2_device text,
 ADD COLUMN c2_sequence bigint,ADD COLUMN c2_source_state text,ADD COLUMN c2_source_reason text NOT NULL DEFAULT '',
 ADD COLUMN c2_work jsonb NOT NULL DEFAULT '{}'::jsonb;
CREATE INDEX c2_hourglass_sequence ON g3_hourglass_activations(run_id,c2_sequence) WHERE c2_body IS NOT NULL;
CREATE INDEX hourglass_record_work ON g3_hourglass_activations USING gin(record jsonb_path_ops);
CREATE INDEX c2_hourglass_work ON g3_hourglass_activations USING gin(c2_work jsonb_path_ops);
CREATE OR REPLACE FUNCTION g3_hourglass_immutable_original() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE previous jsonb; current jsonb; i integer;
BEGIN
 IF TG_OP='DELETE' THEN IF EXISTS(SELECT 1 FROM runs WHERE id=OLD.run_id AND state NOT IN ('running','staging') AND coalesce(ended_at,created_at)<now()-interval '30 days') THEN RETURN OLD; END IF; RAISE EXCEPTION 'Retained hourglass original'; END IF;
 IF NEW.c2_body IS DISTINCT FROM OLD.c2_body OR NEW.c2_digest IS DISTINCT FROM OLD.c2_digest OR NEW.c2_device IS DISTINCT FROM OLD.c2_device OR NEW.c2_sequence IS DISTINCT FROM OLD.c2_sequence OR NEW.run_id<>OLD.run_id OR NEW.activation_id<>OLD.activation_id OR NEW.sequence<>OLD.sequence OR NEW.created_at<>OLD.created_at
 OR NEW.record->'worldTokenEncoding' IS DISTINCT FROM OLD.record->'worldTokenEncoding' OR NEW.record->'plan' IS DISTINCT FROM OLD.record->'plan' OR NEW.record->'digest' IS DISTINCT FROM OLD.record->'digest' THEN
  RAISE EXCEPTION 'Immutable hourglass original';
 END IF;
 IF jsonb_array_length(NEW.record->'deliveries')<>jsonb_array_length(OLD.record->'deliveries') THEN RAISE EXCEPTION 'Immutable recipients'; END IF;
 FOR i IN 0..jsonb_array_length(OLD.record->'deliveries')-1 LOOP
  previous=OLD.record->'deliveries'->i;current=NEW.record->'deliveries'->i;
  IF (previous-'outcome'-'receiptId') IS DISTINCT FROM (current-'outcome'-'receiptId') OR
   (previous->>'outcome'<>'pending' AND previous IS DISTINCT FROM current) THEN RAISE EXCEPTION 'Immutable delivery'; END IF;
 END LOOP;
 RETURN NEW;
END $$;


INSERT INTO schema_migrations(version) VALUES(126);
