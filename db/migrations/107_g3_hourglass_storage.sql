CREATE OR REPLACE FUNCTION g3_hourglass_immutable_original() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE previous jsonb; current jsonb; i integer;
BEGIN
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'Retained hourglass original'; END IF;
 IF NEW.run_id<>OLD.run_id OR NEW.activation_id<>OLD.activation_id OR NEW.sequence<>OLD.sequence OR NEW.created_at<>OLD.created_at
 OR NEW.record->'worldTokenEncoding' IS DISTINCT FROM OLD.record->'worldTokenEncoding' OR NEW.record->'plan' IS DISTINCT FROM OLD.record->'plan' OR NEW.record->'digest' IS DISTINCT FROM OLD.record->'digest' THEN
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
INSERT INTO schema_migrations(version) VALUES(107);
