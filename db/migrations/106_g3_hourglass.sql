CREATE TABLE g3_hourglass_activations (
 run_id uuid NOT NULL REFERENCES g3_runs(id),
 activation_id uuid NOT NULL,
 sequence integer NOT NULL CHECK(sequence BETWEEN 1 AND 256),
 record jsonb NOT NULL CHECK(octet_length(record::text)<=32768)
  CHECK(record->'plan'->>'activationId'=activation_id::text)
  CHECK(record->'plan'->'scope'->>'runId'=run_id::text),
 created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(run_id,activation_id), UNIQUE(run_id,sequence)
);
CREATE FUNCTION g3_hourglass_immutable_original() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE previous jsonb; current jsonb; i integer;
BEGIN
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'Retained hourglass original'; END IF;
 IF NEW.run_id<>OLD.run_id OR NEW.activation_id<>OLD.activation_id OR NEW.sequence<>OLD.sequence OR NEW.created_at<>OLD.created_at
 OR NEW.record->'plan' IS DISTINCT FROM OLD.record->'plan' OR NEW.record->'digest' IS DISTINCT FROM OLD.record->'digest' THEN
  RAISE EXCEPTION 'Immutable hourglass original';
 END IF;
 IF jsonb_array_length(NEW.record->'deliveries')<>jsonb_array_length(OLD.record->'deliveries') THEN RAISE EXCEPTION 'Immutable recipients'; END IF;
 FOR i IN 0..jsonb_array_length(OLD.record->'deliveries')-1 LOOP
  previous=OLD.record->'deliveries'->i;current=NEW.record->'deliveries'->i;
  IF (previous-'outcome'-'receiptId') IS DISTINCT FROM (current-'outcome'-'receiptId') OR
   (previous->>'outcome'<>'pending' AND previous IS DISTINCT FROM current) THEN RAISE EXCEPTION 'Immutable delivery'; END IF;
 END LOOP;
 RETURN NEW;
END $$;
CREATE TRIGGER g3_hourglass_original_guard BEFORE UPDATE OR DELETE ON g3_hourglass_activations FOR EACH ROW EXECUTE FUNCTION g3_hourglass_immutable_original();
CREATE TABLE g3_hourglass_fences (
 run_id uuid NOT NULL REFERENCES g3_runs(id), session_id uuid NOT NULL,
 window_at timestamptz NOT NULL, count integer NOT NULL CHECK(count BETWEEN 1 AND 10),
 PRIMARY KEY(run_id,session_id)
);
INSERT INTO schema_migrations(version) VALUES(106);
