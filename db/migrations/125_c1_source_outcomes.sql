-- Bind the existing accepted-original ledger to its real source device/config.
-- Earlier development rows remain uncovered rather than guessing an identity.
ALTER TABLE c1_events ADD COLUMN source_device text NOT NULL DEFAULT '';
ALTER TABLE c1_events ADD COLUMN source_config_revision bigint NOT NULL DEFAULT 0 CHECK(source_config_revision BETWEEN 0 AND 4294967295);
ALTER TABLE c1_events ADD COLUMN source_reason text NOT NULL DEFAULT '';
INSERT INTO schema_migrations(version) VALUES(125);
