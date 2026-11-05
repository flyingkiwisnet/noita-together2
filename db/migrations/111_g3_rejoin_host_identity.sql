-- Private host identity: never copied to client resources or package files.
CREATE TABLE rejoin_host_identity (
 singleton boolean PRIMARY KEY DEFAULT true CHECK(singleton),
 service_id text NOT NULL, issuer text NOT NULL,
 keys jsonb NOT NULL, created_at timestamptz NOT NULL DEFAULT now()
);
INSERT INTO schema_migrations(version) VALUES(111);
