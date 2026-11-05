-- Internal zero-initialized earning fixture only. 108 no-increase guard unchanged.
CREATE TABLE g3_nemesis_earning_accounts(run_id uuid REFERENCES g3_nemesis_runs(id),principal_id uuid REFERENCES accounts(id),provenance text NOT NULL CHECK(provenance='fixture-validated-client-report-disabled-earnings'),PRIMARY KEY(run_id,principal_id));
CREATE TRIGGER g3_nemesis_earning_accounts_immutable BEFORE UPDATE OR DELETE ON g3_nemesis_earning_accounts FOR EACH ROW EXECUTE FUNCTION g3_immutable_audit();
CREATE FUNCTION g3_nemesis_funding_separation() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
 IF TG_TABLE_NAME='g3_nemesis_earning_accounts' THEN
  IF EXISTS(SELECT 1 FROM g3_nemesis_balances WHERE run_id=NEW.run_id AND principal_id=NEW.principal_id) THEN RAISE EXCEPTION 'No mixed Nemesis funding'; END IF;
 ELSE
  IF EXISTS(SELECT 1 FROM g3_nemesis_earning_accounts WHERE run_id=NEW.run_id AND principal_id=NEW.principal_id) THEN RAISE EXCEPTION 'No mixed Nemesis funding'; END IF;
 END IF;RETURN NEW;
END $$;
CREATE TRIGGER g3_nemesis_earning_separation BEFORE INSERT ON g3_nemesis_earning_accounts FOR EACH ROW EXECUTE FUNCTION g3_nemesis_funding_separation();
CREATE TRIGGER g3_nemesis_fixture_separation BEFORE INSERT ON g3_nemesis_balances FOR EACH ROW EXECUTE FUNCTION g3_nemesis_funding_separation();
CREATE TABLE g3_nemesis_kills(run_id uuid REFERENCES g3_nemesis_runs(id),kill_id uuid NOT NULL,victim_id uuid NOT NULL,principal_id uuid REFERENCES accounts(id),sequence integer NOT NULL CHECK(sequence BETWEEN 1 AND 2048),points bigint NOT NULL CHECK(points BETWEEN 0 AND 4294967295),record jsonb NOT NULL CHECK(octet_length(record::text)<=8192),PRIMARY KEY(run_id,kill_id),UNIQUE(run_id,victim_id),UNIQUE(run_id,sequence),FOREIGN KEY(run_id,principal_id) REFERENCES g3_nemesis_earning_accounts,
 CHECK(record->>'revision'='g3-nemesis-provenance-internal-1'),CHECK(record->>'provenance'='validated-client-report'),CHECK(record->'source'->>'runId'=run_id::text),CHECK(record->'source'->>'principalId'=principal_id::text),CHECK(record->'evidence'->>'killId'=kill_id::text),CHECK(record->'evidence'->>'victimId'=victim_id::text),CHECK((record->>'points')::bigint=points));
CREATE TRIGGER g3_nemesis_kills_immutable BEFORE UPDATE OR DELETE ON g3_nemesis_kills FOR EACH ROW EXECUTE FUNCTION g3_immutable_audit();
CREATE TABLE g3_nemesis_shop_provenance(run_id uuid REFERENCES g3_nemesis_runs(id),group_id uuid NOT NULL,sequence integer NOT NULL CHECK(sequence BETWEEN 1 AND 256),record jsonb NOT NULL CHECK(octet_length(record::text)<=8192),PRIMARY KEY(run_id,group_id),UNIQUE(run_id,sequence),CHECK(record->'source'->>'runId'=run_id::text),CHECK(record->'evidence'->>'groupId'=group_id::text),CHECK(record->>'rngVerified'='false'));
CREATE TRIGGER g3_nemesis_shop_provenance_immutable BEFORE UPDATE OR DELETE ON g3_nemesis_shop_provenance FOR EACH ROW EXECUTE FUNCTION g3_immutable_audit();
INSERT INTO schema_migrations(version) VALUES(109);
