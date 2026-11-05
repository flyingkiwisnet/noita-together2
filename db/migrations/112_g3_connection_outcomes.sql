-- CR2 disposable/development rejoin namespaces only. No proof/credential bytes.
CREATE TABLE connection_outcomes (
 participant_id uuid NOT NULL REFERENCES rejoin_participants(id),
 request_id text NOT NULL CHECK(length(request_id) BETWEEN 1 AND 128),
 kind text NOT NULL CHECK(kind IN ('resume','offline')),
 original jsonb NOT NULL, fingerprint text NOT NULL CHECK(length(fingerprint)=64),
 effect_id text NOT NULL, created_at timestamptz NOT NULL, expires_at timestamptz NOT NULL,
 PRIMARY KEY(participant_id,request_id,kind)
);
CREATE INDEX connection_outcomes_expiry ON connection_outcomes(participant_id,expires_at);
INSERT INTO schema_migrations(version) VALUES(112);
