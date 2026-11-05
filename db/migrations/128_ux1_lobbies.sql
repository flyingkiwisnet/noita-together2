ALTER TABLE rooms ADD COLUMN join_locked boolean NOT NULL DEFAULT false;
CREATE TABLE room_chat_mutes (
 room_id uuid NOT NULL REFERENCES rooms(id), account_id uuid NOT NULL REFERENCES accounts(id),
 actor_id uuid NOT NULL REFERENCES accounts(id), created_at timestamptz NOT NULL,
 PRIMARY KEY(room_id,account_id)
);
CREATE TABLE room_moderation_audit (
 sequence bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
 room_id uuid NOT NULL REFERENCES rooms(id), actor_id uuid NOT NULL REFERENCES accounts(id),
 target_id uuid NOT NULL REFERENCES accounts(id), session_id uuid NOT NULL REFERENCES sessions(id),
 request_id text NOT NULL, action text NOT NULL CHECK(action IN ('kick','ban','unban','mute','unmute')),
 created_at timestamptz NOT NULL, UNIQUE(session_id,request_id)
);
CREATE INDEX room_moderation_recent ON room_moderation_audit(room_id,sequence DESC);
INSERT INTO schema_migrations(version) VALUES(128);
