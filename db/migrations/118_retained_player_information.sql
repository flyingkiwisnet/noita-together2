-- One latest merged player-information state per durable run/world, never TTL.
CREATE TABLE g3_social_hud_state (
 run_id uuid NOT NULL REFERENCES runs(id), world_token text NOT NULL CHECK(octet_length(world_token) BETWEEN 1 AND 80),
 principal_id uuid NOT NULL REFERENCES accounts(id), revision bigint NOT NULL CHECK(revision BETWEEN 1 AND 4294967295),
 body jsonb NOT NULL CHECK(octet_length(body::text)<=4096), PRIMARY KEY(run_id,world_token)
);
INSERT INTO schema_migrations(version) VALUES(118);
