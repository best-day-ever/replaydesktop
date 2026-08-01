ALTER TABLE workstations ADD COLUMN registration_id TEXT;
ALTER TABLE workstations ADD COLUMN registered_hostname TEXT;
ALTER TABLE workstations ADD COLUMN agent_version TEXT;
ALTER TABLE workstations ADD COLUMN last_seen_at INTEGER;

CREATE UNIQUE INDEX workstations_registration_id
    ON workstations(registration_id)
    WHERE registration_id IS NOT NULL;

CREATE INDEX workstations_last_seen
    ON workstations(active, last_seen_at);
