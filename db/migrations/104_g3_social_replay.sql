-- Opt-in G3 social only. Durable IDs now belong exclusively to POI placements.
-- Transient retry answers are bounded independently for each sender and kind;
-- persistent sequence fences remain authoritative after answers are retired.
CREATE TABLE g3_social_replay(
 run_id uuid REFERENCES runs(id),game_session_id uuid REFERENCES sessions(id),
 kind text NOT NULL CHECK(kind IN ('hud','emote','cosmetic')),
 sequence bigint NOT NULL CHECK(sequence BETWEEN 1 AND 4294967295),
 event_id uuid NOT NULL,digest text NOT NULL CHECK(length(digest)=64),
 PRIMARY KEY(run_id,game_session_id,event_id),
 UNIQUE(run_id,game_session_id,kind,sequence));
-- Existing non-POI answers can conservatively retire: their sequence fences survive.
DELETE FROM g3_social_ids i WHERE NOT EXISTS(
 SELECT 1 FROM g3_social_pois p WHERE p.run_id=i.run_id
 AND p.game_session_id=i.game_session_id AND p.event_id=i.event_id);
INSERT INTO schema_migrations(version) VALUES(104);
