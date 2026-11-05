-- Delete is an explicit permanent room tombstone, not End or missing discovery.
-- Keep run/access/statistics rows for history and authenticated replay fencing.
CREATE INDEX g3_runs_room ON g3_runs(room_id);
CREATE FUNCTION g3_gold_audit_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM g3_runs g JOIN rooms r ON r.id=g.room_id WHERE g.id=OLD.run_id AND r.deleted_at IS NOT NULL) THEN RETURN OLD; END IF;
 RAISE EXCEPTION 'G3 audit records are append only outside deleted lobby cleanup';
END $$;
DROP TRIGGER g3_ledger_immutable ON g3_ledger;
DROP TRIGGER g3_receipts_immutable ON g3_receipts;
CREATE TRIGGER g3_ledger_immutable BEFORE UPDATE OR DELETE ON g3_ledger FOR EACH ROW EXECUTE FUNCTION g3_gold_audit_guard();
CREATE TRIGGER g3_receipts_immutable BEFORE UPDATE OR DELETE ON g3_receipts FOR EACH ROW EXECUTE FUNCTION g3_gold_audit_guard();
CREATE FUNCTION g3_delete_room_transfers(target uuid) RETURNS void LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM rooms WHERE id=target AND deleted_at IS NOT NULL) THEN RAISE EXCEPTION 'Explicit deleted lobby required'; END IF;
 DELETE FROM g3_receipts WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=target);
 DELETE FROM g3_ledger WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=target);
 DELETE FROM g3_outbox WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=target) AND kind IN ('gold.intent','gold.receipt','gold.deposit');
 DELETE FROM g3_operations WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=target);
 DELETE FROM g3_balances WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=target);
 UPDATE g3_runs SET state='ended' WHERE room_id=target AND state<>'ended';
 UPDATE g3_contexts SET revoked_at=COALESCE(revoked_at,now()) WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=target);
END $$;
-- Candidate deployments have deleted_at. Low-level G3 ledger fixtures do not.
DO $$ DECLARE deleted_room uuid; BEGIN
 IF EXISTS(SELECT 1 FROM pg_attribute WHERE attrelid='rooms'::regclass AND attname='deleted_at' AND NOT attisdropped) THEN
  FOR deleted_room IN SELECT DISTINCT r.id FROM rooms r JOIN g3_runs g ON g.room_id=r.id WHERE r.deleted_at IS NOT NULL LOOP
   PERFORM g3_delete_room_transfers(deleted_room);
  END LOOP;
 END IF;
END $$;
INSERT INTO schema_migrations(version) VALUES(121);
