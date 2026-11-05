-- Combined display invalidation revision. Original identities/effect authority stay separate.
CREATE TABLE g3_bank_versions(run_id uuid PRIMARY KEY REFERENCES runs(id) ON DELETE CASCADE,revision bigint NOT NULL CHECK(revision>=0));
INSERT INTO g3_bank_versions SELECT id,0 FROM runs;
CREATE FUNCTION g3_bank_bump(target uuid) RETURNS void LANGUAGE plpgsql AS $$
BEGIN
 IF target IS NULL THEN RETURN; END IF;
 INSERT INTO g3_bank_versions VALUES(target,1) ON CONFLICT(run_id) DO UPDATE SET revision=g3_bank_versions.revision+1;
END $$;
CREATE FUNCTION g3_bank_row_changed() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 PERFORM g3_bank_bump(CASE WHEN TG_OP='DELETE' THEN OLD.run_id ELSE NEW.run_id END);
 RETURN NULL;
END $$;
CREATE TRIGGER g3_bank_gold_changed AFTER INSERT OR UPDATE OR DELETE ON g3_balances FOR EACH ROW EXECUTE FUNCTION g3_bank_row_changed();
CREATE TRIGGER g3_bank_gold_original_changed AFTER INSERT OR UPDATE OR DELETE ON g3_operations FOR EACH ROW EXECUTE FUNCTION g3_bank_row_changed();
CREATE TRIGGER g3_bank_item_changed AFTER INSERT OR UPDATE OR DELETE ON g3_bank_items FOR EACH ROW EXECUTE FUNCTION g3_bank_row_changed();
CREATE TRIGGER g3_bank_item_original_changed AFTER INSERT OR UPDATE OR DELETE ON g3_item_operations FOR EACH ROW EXECUTE FUNCTION g3_bank_row_changed();
CREATE FUNCTION g3_bank_room_changed() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF (OLD.state,OLD.config_revision,OLD.deleted_at,OLD.current_run_id) IS DISTINCT FROM (NEW.state,NEW.config_revision,NEW.deleted_at,NEW.current_run_id) THEN
  PERFORM g3_bank_bump(OLD.current_run_id);
  IF NEW.current_run_id IS DISTINCT FROM OLD.current_run_id THEN PERFORM g3_bank_bump(NEW.current_run_id); END IF;
 END IF;
 RETURN NULL;
END $$;
CREATE TRIGGER g3_bank_room_changed AFTER UPDATE ON rooms FOR EACH ROW EXECUTE FUNCTION g3_bank_room_changed();
INSERT INTO schema_migrations(version) VALUES(123);
