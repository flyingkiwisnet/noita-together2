-- B1 extends the existing original/ledger/outbox, never a second bank balance.
ALTER TABLE g3_balances ADD COLUMN version bigint NOT NULL DEFAULT 0 CHECK(version>=0);
ALTER TABLE g3_operations ADD COLUMN run_epoch uuid;
ALTER TABLE g3_operations ADD COLUMN config_revision integer;
UPDATE g3_operations o SET run_epoch=r.epoch,config_revision=r.config_revision FROM g3_runs r WHERE r.id=o.run_id;
ALTER TABLE g3_operations ALTER COLUMN run_epoch SET NOT NULL;
ALTER TABLE g3_operations ALTER COLUMN config_revision SET NOT NULL;
ALTER TABLE g3_operations ADD CONSTRAINT g3_original_config CHECK(config_revision>0);
CREATE INDEX g3_gold_pending ON g3_operations(run_id,principal_id,world_token,operation_id) WHERE state NOT IN ('completed','cancelled');
CREATE INDEX g3_gold_notice_delivery ON g3_outbox(run_id,principal_id,sequence) WHERE kind='gold.deposit';
INSERT INTO schema_migrations(version) VALUES(120);
