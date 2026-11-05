-- Candidate only. NOT loaded by the accepted G1/G2 migration runner.
CREATE TABLE principal_profiles (
 principal_id uuid PRIMARY KEY REFERENCES accounts(id),
 kind text NOT NULL CHECK(kind IN ('registered','guest')),
 display_name text NOT NULL CHECK(octet_length(display_name) BETWEEN 1 AND 80),
 source text NOT NULL CHECK(source IN ('legacy-mock','mock:twitch','mock:website','guest')),
 created_at timestamptz NOT NULL DEFAULT now(), guest_expires_at timestamptz,
 CHECK((kind='guest')=(source='guest')),
 CHECK((kind='guest')=(guest_expires_at IS NOT NULL))
);
INSERT INTO principal_profiles(principal_id,kind,display_name,source)
 SELECT id,'registered',left(provider_subject,20),'legacy-mock' FROM accounts;
ALTER TABLE rooms ADD COLUMN allow_guests boolean NOT NULL DEFAULT false;
ALTER TABLE rooms DROP CONSTRAINT rooms_state_check;
ALTER TABLE rooms ADD CHECK(state IN ('waiting','preparing','loading','staging','running','suspended','ended'));
ALTER TABLE runs DROP CONSTRAINT runs_state_check;
ALTER TABLE runs ADD CHECK(state IN ('preparing','loading','staging','running','suspended','cancelled','ended'));
ALTER TABLE runs ADD COLUMN review_until timestamptz;
CREATE FUNCTION registered_room_host() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM principal_profiles WHERE principal_id=NEW.host_account_id AND kind='registered') THEN
  RAISE EXCEPTION 'Registered room host required' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER registered_room_host BEFORE INSERT OR UPDATE OF host_account_id ON rooms FOR EACH ROW EXECUTE FUNCTION registered_room_host();
CREATE TABLE world_observations (
 attachment_id uuid PRIMARY KEY REFERENCES attachments(id), world_token text NOT NULL CHECK(octet_length(world_token) BETWEEN 1 AND 128),
 revision integer NOT NULL CHECK(revision>0), body jsonb NOT NULL,
 fingerprint text NOT NULL CHECK(length(fingerprint)=64), content_digest text NOT NULL CHECK(length(content_digest)=64),
 captured_at timestamptz NOT NULL, valid_until timestamptz NOT NULL
);
CREATE TABLE inspected_releases (
 run_id uuid PRIMARY KEY REFERENCES runs(id), authorization_id text NOT NULL CHECK(octet_length(authorization_id) BETWEEN 1 AND 128),
 host_principal_id uuid NOT NULL REFERENCES principal_profiles(principal_id),
 selected_reports jsonb NOT NULL, authorized_at timestamptz NOT NULL,
 revoked_at timestamptz, reason text
);
CREATE TABLE account_signin_attempts (
 id uuid PRIMARY KEY, device_id text NOT NULL, app_instance_id text NOT NULL,
 provider text NOT NULL CHECK(provider IN ('twitch','website')),
 proof_hash text NOT NULL CHECK(length(proof_hash)=64), callback_port integer NOT NULL CHECK(callback_port BETWEEN 1024 AND 65535),
 callback_state_hash text NOT NULL CHECK(length(callback_state_hash)=64), return_context_ciphertext text NOT NULL,
 state text NOT NULL CHECK(state IN ('pending','completed','exchanged','cancelled','expired')),
 principal_id uuid REFERENCES principal_profiles(principal_id), expires_at timestamptz NOT NULL,
 grant_hash text, grant_expires_at timestamptz, consumed_request_id text,
 service_epoch uuid NOT NULL, created_at timestamptz NOT NULL
);
CREATE INDEX signin_expiry ON account_signin_attempts(expires_at);
INSERT INTO schema_migrations(version) VALUES(7);
