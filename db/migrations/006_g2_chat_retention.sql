CREATE TABLE room_chat_limits (session_id uuid PRIMARY KEY REFERENCES sessions(id), started_at timestamptz NOT NULL, count integer NOT NULL CHECK(count BETWEEN 1 AND 5));
CREATE INDEX room_chat_limits_expiry ON room_chat_limits(started_at);
CREATE INDEX room_chat_expiry ON room_chat(sent_at);
ALTER TABLE command_results ADD COLUMN operation text NOT NULL DEFAULT 'foundation' CHECK(operation IN ('foundation','chat'));
CREATE INDEX command_chat_retention ON command_results(created_at) WHERE operation='chat';
INSERT INTO schema_migrations(version) VALUES (6);
