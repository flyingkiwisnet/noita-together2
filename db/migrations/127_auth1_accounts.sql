ALTER TABLE accounts ADD COLUMN role text NOT NULL DEFAULT 'user' CHECK(role IN ('user','power-user','admin','dev'));
ALTER TABLE accounts ADD COLUMN role_revision integer NOT NULL DEFAULT 1 CHECK(role_revision>0);
ALTER TABLE accounts ADD COLUMN last_signin_at timestamptz;
ALTER TABLE session_families ADD COLUMN authenticated_at timestamptz;
ALTER TABLE principal_profiles DROP CONSTRAINT principal_profiles_source_check;
ALTER TABLE principal_profiles ADD CONSTRAINT principal_profiles_source_check CHECK(source IN ('legacy-mock','mock:twitch','mock:website','guest','twitch','google','local'));
ALTER TABLE account_signin_attempts DROP CONSTRAINT account_signin_attempts_provider_check;
ALTER TABLE account_signin_attempts ADD CONSTRAINT account_signin_attempts_provider_check CHECK(provider IN ('twitch','website','google','local'));
ALTER TABLE account_signin_attempts ADD COLUMN provider_session_ciphertext text;
CREATE TABLE auth1_local_credentials (
 account_id uuid PRIMARY KEY REFERENCES accounts(id), username text NOT NULL UNIQUE,
 password_hash text, revision integer NOT NULL DEFAULT 1, temporary_until timestamptz,
 temporary_consumed boolean NOT NULL DEFAULT false
);
CREATE TABLE auth1_audit (
 sequence bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 operation_id text NOT NULL UNIQUE, actor_id uuid REFERENCES accounts(id), operator text,
 target_id uuid REFERENCES accounts(id), action text NOT NULL, old_role text, new_role text,
 reason text NOT NULL DEFAULT '', created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX auth1_recent_users ON accounts(last_signin_at DESC NULLS LAST,id);
CREATE TABLE auth1_login_limits (bucket text PRIMARY KEY, attempts integer NOT NULL, until_at timestamptz NOT NULL);
CREATE TABLE auth1_flows (
 id uuid PRIMARY KEY, attempt_id uuid REFERENCES account_signin_attempts(id), provider text NOT NULL,
 binding_hash text NOT NULL, csrf_hash text NOT NULL, state_hash text UNIQUE, context_ciphertext text,
 expires_at timestamptz NOT NULL, consumed_at timestamptz, password_account_id uuid REFERENCES accounts(id),
 password_revision integer
);
CREATE INDEX auth1_flow_expiry ON auth1_flows(expires_at);
CREATE TABLE auth1_web_sessions (
 handle_hash text PRIMARY KEY, family_id uuid NOT NULL REFERENCES session_families(id),
 credentials_ciphertext text NOT NULL, csrf_hash text NOT NULL, expires_at timestamptz NOT NULL
);
CREATE TABLE auth1_provider_sessions (
 family_id uuid PRIMARY KEY REFERENCES session_families(id), provider text NOT NULL, subject text NOT NULL,
 token_ciphertext text NOT NULL, checked_at timestamptz NOT NULL, valid_until timestamptz NOT NULL,
 revoked_at timestamptz
);
INSERT INTO schema_migrations(version) VALUES(127);
