-- Selected RJ candidate only; never applied by the published launcher.
CREATE TABLE rejoin_runs (
 run_id uuid PRIMARY KEY REFERENCES runs(id), room_id uuid NOT NULL REFERENCES rooms(id),
 original_start_id text NOT NULL, frozen_config jsonb NOT NULL, config_revision integer NOT NULL,
 terminal_reason text, created_at timestamptz NOT NULL
);
CREATE TABLE rejoin_participants (
 id uuid PRIMARY KEY, run_id uuid NOT NULL REFERENCES rejoin_runs(run_id),
 account_id uuid NOT NULL REFERENCES accounts(id), device_id text NOT NULL,
 world_token text NOT NULL, proof_hash text NOT NULL, proof_ciphertext text NOT NULL,
 original_report jsonb NOT NULL, accepted_digest text NOT NULL,
 generation integer NOT NULL DEFAULT 1 CHECK(generation>0),
 desktop_id uuid NOT NULL REFERENCES sessions(id), attachment_id uuid NOT NULL REFERENCES attachments(id),
 state text NOT NULL CHECK(state IN ('active','reconnecting','inspecting','review','offline','removed','ended')),
 reason text NOT NULL DEFAULT '', terminal boolean NOT NULL DEFAULT false,
 authorization_id uuid NOT NULL, authorized_at timestamptz NOT NULL,
 pending_report jsonb, created_at timestamptz NOT NULL,
 UNIQUE(run_id,world_token), UNIQUE(run_id,account_id,device_id)
);
CREATE TABLE rejoin_requests (
 participant_id uuid NOT NULL REFERENCES rejoin_participants(id), account_id uuid NOT NULL,
 request_id text NOT NULL, fingerprint text NOT NULL, generation integer NOT NULL,
 desktop_id uuid NOT NULL, service_epoch uuid NOT NULL, result_ciphertext text NOT NULL,
 PRIMARY KEY(participant_id,account_id,request_id)
);
-- Reuse migration008's live_attachment partial uniqueness; retired history
-- remains immutable and does not occupy a new active owner slot.
INSERT INTO schema_migrations(version) VALUES(110);
