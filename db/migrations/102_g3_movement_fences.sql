-- Pose display may expire/evict independently of authenticated sequence/rate fencing.
CREATE TABLE g3_movement_fences(
 attachment_id uuid PRIMARY KEY REFERENCES g3_contexts(attachment_id),
 last_sequence bigint NOT NULL CHECK(last_sequence BETWEEN 1 AND 4294967295),
 window_at timestamptz NOT NULL,
 window_count integer NOT NULL CHECK(window_count BETWEEN 1 AND 10)
);
INSERT INTO schema_migrations(version) VALUES(102);
