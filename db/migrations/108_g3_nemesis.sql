-- Opt-in isolated Nemesis runner only; no changes to core/G2 bank or namespace.
CREATE TABLE g3_nemesis_runs(id uuid PRIMARY KEY REFERENCES runs(id),room_id uuid NOT NULL,epoch uuid NOT NULL,config_revision integer NOT NULL,state text NOT NULL CHECK(state IN ('running','ended')));
CREATE TABLE g3_nemesis_access(run_id uuid REFERENCES g3_nemesis_runs(id),principal_id uuid REFERENCES accounts(id),PRIMARY KEY(run_id,principal_id));
CREATE TABLE g3_nemesis_balances(run_id uuid REFERENCES g3_nemesis_runs(id),principal_id uuid REFERENCES accounts(id),points bigint NOT NULL CHECK(points BETWEEN 0 AND 4294967295),provenance text NOT NULL CHECK(provenance='fixture-funded-disabled-earnings'),PRIMARY KEY(run_id,principal_id));
CREATE TABLE g3_nemesis_groups(run_id uuid REFERENCES g3_nemesis_runs(id),group_id uuid NOT NULL,sequence integer NOT NULL CHECK(sequence BETWEEN 1 AND 256),record jsonb NOT NULL CHECK(octet_length(record::text)<=32768),PRIMARY KEY(run_id,group_id),UNIQUE(run_id,sequence));
CREATE TABLE g3_nemesis_offers(run_id uuid,offer_id uuid,group_id uuid,ability_id text NOT NULL,PRIMARY KEY(run_id,offer_id),FOREIGN KEY(run_id,group_id) REFERENCES g3_nemesis_groups);
CREATE TRIGGER g3_nemesis_offers_immutable BEFORE UPDATE OR DELETE ON g3_nemesis_offers FOR EACH ROW EXECUTE FUNCTION g3_immutable_audit();
CREATE TABLE g3_nemesis_purchases(run_id uuid REFERENCES g3_nemesis_runs(id),operation_id uuid NOT NULL,sequence integer NOT NULL CHECK(sequence BETWEEN 1 AND 256),record jsonb NOT NULL CHECK(octet_length(record::text)<=32768) CHECK(record->>'operationId'=operation_id::text) CHECK(record->'source'->>'runId'=run_id::text),created_at timestamptz NOT NULL DEFAULT now(),PRIMARY KEY(run_id,operation_id),UNIQUE(run_id,sequence));
CREATE TABLE g3_nemesis_debits(run_id uuid,operation_id uuid,principal_id uuid REFERENCES accounts(id),cost integer NOT NULL CHECK(cost BETWEEN 35 AND 1265),points_after bigint NOT NULL CHECK(points_after BETWEEN 0 AND 4294967295),created_at timestamptz NOT NULL DEFAULT now(),PRIMARY KEY(run_id,operation_id),FOREIGN KEY(run_id,operation_id) REFERENCES g3_nemesis_purchases);
CREATE TABLE g3_nemesis_fences(run_id uuid REFERENCES g3_nemesis_runs(id),session_id uuid,window_at timestamptz NOT NULL,count integer NOT NULL CHECK(count BETWEEN 1 AND 10),PRIMARY KEY(run_id,session_id));
CREATE FUNCTION g3_nemesis_purchase_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE previous jsonb; current jsonb; i integer;
BEGIN
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'Retained Nemesis original'; END IF;
 IF NEW.run_id<>OLD.run_id OR NEW.operation_id<>OLD.operation_id OR NEW.sequence<>OLD.sequence OR NEW.created_at<>OLD.created_at OR (NEW.record-'recipients') IS DISTINCT FROM (OLD.record-'recipients') OR jsonb_array_length(NEW.record->'recipients')<>jsonb_array_length(OLD.record->'recipients') THEN RAISE EXCEPTION 'Immutable Nemesis original'; END IF;
 FOR i IN 0..jsonb_array_length(OLD.record->'recipients')-1 LOOP
  previous=OLD.record->'recipients'->i;current=NEW.record->'recipients'->i;
  IF (previous-'outcome'-'receiptId') IS DISTINCT FROM (current-'outcome'-'receiptId') OR (previous->>'outcome'<>'pending' AND previous IS DISTINCT FROM current) THEN RAISE EXCEPTION 'Immutable Nemesis delivery'; END IF;
 END LOOP;
 RETURN NEW;
END $$;
CREATE TRIGGER g3_nemesis_original_guard BEFORE UPDATE OR DELETE ON g3_nemesis_purchases FOR EACH ROW EXECUTE FUNCTION g3_nemesis_purchase_guard();
CREATE FUNCTION g3_nemesis_group_guard() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
 IF TG_OP='DELETE' OR NEW.run_id<>OLD.run_id OR NEW.group_id<>OLD.group_id OR NEW.sequence<>OLD.sequence OR (NEW.record-'consumedBy') IS DISTINCT FROM (OLD.record-'consumedBy') OR (OLD.record ? 'consumedBy' AND NEW.record IS DISTINCT FROM OLD.record) THEN RAISE EXCEPTION 'Immutable Nemesis offer group'; END IF;RETURN NEW;
END $$;
CREATE TRIGGER g3_nemesis_group_original BEFORE UPDATE OR DELETE ON g3_nemesis_groups FOR EACH ROW EXECUTE FUNCTION g3_nemesis_group_guard();
CREATE FUNCTION g3_nemesis_balance_guard() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
 IF TG_OP='DELETE' OR NEW.run_id<>OLD.run_id OR NEW.principal_id<>OLD.principal_id OR NEW.provenance<>OLD.provenance OR NEW.points>OLD.points THEN RAISE EXCEPTION 'No Nemesis earning grant'; END IF;RETURN NEW;
END $$;
CREATE TRIGGER g3_nemesis_balance_original BEFORE UPDATE OR DELETE ON g3_nemesis_balances FOR EACH ROW EXECUTE FUNCTION g3_nemesis_balance_guard();
CREATE TRIGGER g3_nemesis_debits_immutable BEFORE UPDATE OR DELETE ON g3_nemesis_debits FOR EACH ROW EXECUTE FUNCTION g3_immutable_audit();
INSERT INTO schema_migrations(version) VALUES(108);
