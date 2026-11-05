-- Bounded current presentation, separate from original-action replay metadata.
CREATE TABLE g3_social_presentation (
 run_id uuid NOT NULL REFERENCES runs(id), world_token text NOT NULL CHECK(octet_length(world_token) BETWEEN 1 AND 80),
 principal_id uuid NOT NULL REFERENCES accounts(id), kind text NOT NULL CHECK(kind IN ('emote','cosmetic')),
 revision bigint NOT NULL CHECK(revision BETWEEN 1 AND 4294967295),
 started_at timestamptz NOT NULL, body jsonb NOT NULL CHECK(octet_length(body::text)<=4096),
 PRIMARY KEY(run_id,world_token,kind)
);
-- Old development projections deliberately do not acquire new retained semantics.
DELETE FROM g3_social_latest WHERE kind IN ('emote','cosmetic');
INSERT INTO schema_migrations(version) VALUES(119);
