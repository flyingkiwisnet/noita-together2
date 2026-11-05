ALTER TABLE rooms ADD COLUMN capacity integer NOT NULL DEFAULT 32 CHECK(capacity BETWEEN 1 AND 32);
ALTER TABLE rooms ADD COLUMN password_verifier text;
ALTER TABLE rooms ADD COLUMN chat_sequence bigint NOT NULL DEFAULT 0 CHECK(chat_sequence BETWEEN 0 AND 4294967295);
CREATE TABLE room_bans (room_id uuid NOT NULL REFERENCES rooms(id), account_id uuid NOT NULL REFERENCES accounts(id), PRIMARY KEY(room_id,account_id));
CREATE TABLE room_chat (room_id uuid NOT NULL REFERENCES rooms(id), sequence bigint NOT NULL CHECK(sequence BETWEEN 1 AND 4294967295), account_id uuid NOT NULL REFERENCES accounts(id), session_id uuid NOT NULL REFERENCES sessions(id), session_kind text NOT NULL CHECK(session_kind IN ('browser','desktop')), sent_at timestamptz NOT NULL, text text NOT NULL CHECK(octet_length(text) BETWEEN 1 AND 1024), PRIMARY KEY(room_id,sequence));
CREATE INDEX room_chat_session_rate ON room_chat(session_id,sent_at);
INSERT INTO schema_migrations(version) VALUES (5);
