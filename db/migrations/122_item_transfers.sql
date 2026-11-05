-- Common item deposit/availability/claim delivery under the existing run authority.
ALTER TABLE g3_runs ADD COLUMN item_revision bigint NOT NULL DEFAULT 0 CHECK(item_revision>=0);
CREATE TABLE g3_bank_items(
 run_id uuid NOT NULL REFERENCES g3_runs(id),item_id uuid NOT NULL,principal_id uuid NOT NULL REFERENCES accounts(id),sender_name text NOT NULL,
 kind text NOT NULL CHECK(kind IN ('spell','wand','container','object')),descriptor jsonb NOT NULL CHECK(octet_length(descriptor::text)<=196608),
 state text NOT NULL CHECK(state IN ('awaiting_local','available','claimed','delivered','cancelled','uncertain')),revision bigint NOT NULL CHECK(revision>0),
 presentation jsonb NOT NULL CHECK(octet_length(presentation::text)<=524288),search_text text NOT NULL,spell_type integer NOT NULL CHECK(spell_type BETWEEN 0 AND 7),wand_sort double precision[] NOT NULL,
 created_at timestamptz NOT NULL,PRIMARY KEY(run_id,item_id));
CREATE INDEX g3_bank_item_view ON g3_bank_items(run_id,state,kind,item_id);
CREATE TABLE g3_item_operations(
 run_id uuid NOT NULL REFERENCES g3_runs(id),principal_id uuid NOT NULL REFERENCES accounts(id),operation_id uuid NOT NULL,item_id uuid NOT NULL,
 original_scope jsonb NOT NULL CHECK(octet_length(original_scope::text)<=2048),direction text NOT NULL CHECK(direction IN ('deposit','claim')),
 payload_digest text NOT NULL CHECK(length(payload_digest)=64),delivery_id uuid NOT NULL UNIQUE,
 state text NOT NULL CHECK(state IN ('awaiting_local','reserved','uncertain','completed','cancelled')),local_outcome text NOT NULL DEFAULT 'unknown',created_at timestamptz NOT NULL,
 PRIMARY KEY(run_id,principal_id,operation_id),FOREIGN KEY(run_id,item_id) REFERENCES g3_bank_items(run_id,item_id));
CREATE UNIQUE INDEX g3_one_item_claim ON g3_item_operations(run_id,item_id) WHERE direction='claim';
CREATE INDEX g3_item_pending ON g3_item_operations(run_id,principal_id,operation_id) WHERE state NOT IN ('completed','cancelled');
CREATE TABLE g3_item_receipts(
 run_id uuid NOT NULL,principal_id uuid NOT NULL,operation_id uuid NOT NULL,receipt_id uuid NOT NULL,fingerprint text NOT NULL CHECK(length(fingerprint)=64),
 evidence_digest text NOT NULL CHECK(length(evidence_digest)=64),outcome text NOT NULL,created_at timestamptz NOT NULL,
 PRIMARY KEY(run_id,principal_id,operation_id,receipt_id),FOREIGN KEY(run_id,principal_id,operation_id) REFERENCES g3_item_operations(run_id,principal_id,operation_id));
CREATE TRIGGER g3_item_receipts_immutable BEFORE UPDATE OR DELETE ON g3_item_receipts FOR EACH ROW EXECUTE FUNCTION g3_gold_audit_guard();
CREATE FUNCTION g3_item_original_guard() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
 IF TG_OP='DELETE' THEN
  IF EXISTS(SELECT 1 FROM g3_runs g JOIN rooms r ON r.id=g.room_id WHERE g.id=OLD.run_id AND r.deleted_at IS NOT NULL) THEN RETURN OLD; END IF;
  RAISE EXCEPTION 'Item originals retained until explicit lobby deletion';
 END IF;
 IF ROW(NEW.run_id,NEW.principal_id,NEW.operation_id,NEW.item_id,NEW.original_scope,NEW.direction,NEW.payload_digest,NEW.delivery_id,NEW.created_at)
  IS DISTINCT FROM ROW(OLD.run_id,OLD.principal_id,OLD.operation_id,OLD.item_id,OLD.original_scope,OLD.direction,OLD.payload_digest,OLD.delivery_id,OLD.created_at) THEN
  RAISE EXCEPTION 'Item original identity is immutable';
 END IF;RETURN NEW;
END $$;
CREATE TRIGGER g3_item_original_immutable BEFORE UPDATE OR DELETE ON g3_item_operations FOR EACH ROW EXECUTE FUNCTION g3_item_original_guard();
CREATE TABLE g3_item_inventory(
 run_id uuid NOT NULL REFERENCES g3_runs(id),game_session_id uuid NOT NULL REFERENCES sessions(id),principal_id uuid NOT NULL REFERENCES accounts(id),attachment_id uuid NOT NULL REFERENCES g3_contexts(attachment_id),world_token text NOT NULL,
 revision bigint NOT NULL CHECK(revision BETWEEN 1 AND 4294967295),body jsonb NOT NULL CHECK(octet_length(body::text)<=196608),fingerprint text NOT NULL,held_sprite_id text NOT NULL,captured_at timestamptz NOT NULL,
 PRIMARY KEY(run_id,game_session_id));
CREATE OR REPLACE FUNCTION g3_delete_room_transfers(target uuid) RETURNS void LANGUAGE plpgsql AS $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM rooms WHERE id=target AND deleted_at IS NOT NULL) THEN RAISE EXCEPTION 'Explicit deleted lobby required'; END IF;
 DELETE FROM g3_item_receipts WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=target);
 DELETE FROM g3_item_operations WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=target);
 DELETE FROM g3_bank_items WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=target);
 DELETE FROM g3_item_inventory WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=target);
 DELETE FROM g3_receipts WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=target);
 DELETE FROM g3_ledger WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=target);
 DELETE FROM g3_outbox WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=target) AND kind IN ('gold.intent','gold.receipt','gold.deposit','item.intent','item.receipt','item.deposit');
 DELETE FROM g3_operations WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=target);
 DELETE FROM g3_balances WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=target);
 UPDATE g3_runs SET state='ended' WHERE room_id=target AND state<>'ended';
 UPDATE g3_contexts SET revoked_at=COALESCE(revoked_at,now()) WHERE run_id IN (SELECT id FROM g3_runs WHERE room_id=target);
END $$;
INSERT INTO schema_migrations(version) VALUES(122);
