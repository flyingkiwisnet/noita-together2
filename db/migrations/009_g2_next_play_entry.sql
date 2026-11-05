-- Candidate-only durable entry into this exact selected authorization.
CREATE TABLE authorization_play_entries (
 authorization_uuid uuid NOT NULL REFERENCES inspected_releases(authorization_uuid),
 attachment_id uuid NOT NULL REFERENCES attachments(id),
 entered_playing_at timestamptz NOT NULL,
 PRIMARY KEY(authorization_uuid,attachment_id)
);
INSERT INTO schema_migrations(version) VALUES(9);
