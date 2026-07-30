PRAGMA foreign_keys = ON;

CREATE TABLE users (
    id TEXT PRIMARY KEY NOT NULL,
    username TEXT NOT NULL UNIQUE COLLATE NOCASE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('admin', 'user')),
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at INTEGER NOT NULL
);

CREATE TABLE auth_identities (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    provider_subject TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE(provider, provider_subject)
);

CREATE TABLE workstations (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE,
    lan_ipv4 TEXT NOT NULL,
    kymux_port INTEGER NOT NULL CHECK (kymux_port BETWEEN 1 AND 65535),
    wan_port INTEGER NOT NULL UNIQUE CHECK (wan_port BETWEEN 1 AND 65535),
    certificate_sha256 TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at INTEGER NOT NULL
);

CREATE TABLE workstation_grants (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    workstation_id TEXT NOT NULL REFERENCES workstations(id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL,
    PRIMARY KEY(user_id, workstation_id)
);

CREATE TABLE api_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    access_hash BLOB NOT NULL UNIQUE,
    access_expires_at INTEGER NOT NULL,
    refresh_hash BLOB NOT NULL UNIQUE,
    refresh_expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    revoked_at INTEGER
);

CREATE TABLE connection_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    workstation_id TEXT NOT NULL REFERENCES workstations(id) ON DELETE CASCADE,
    knock_hash BLOB NOT NULL UNIQUE,
    status TEXT NOT NULL CHECK (
        status IN ('pending', 'opening_lease', 'ready', 'failed', 'closed', 'expired')
    ),
    source_ipv4 TEXT,
    admission_lease_id TEXT,
    ticket_jti TEXT,
    ticket_token TEXT,
    ticket_issued_at INTEGER,
    ticket_expires_at INTEGER,
    failure_reason TEXT,
    created_at INTEGER NOT NULL,
    knock_expires_at INTEGER NOT NULL,
    lease_expires_at INTEGER NOT NULL,
    closed_at INTEGER
);

CREATE INDEX connection_sessions_expiry
    ON connection_sessions(status, lease_expires_at);

CREATE TABLE audit_events (
    id TEXT PRIMARY KEY NOT NULL,
    actor_user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    event_type TEXT NOT NULL,
    subject_type TEXT NOT NULL,
    subject_id TEXT NOT NULL,
    remote_ip TEXT,
    created_at INTEGER NOT NULL
);

CREATE INDEX audit_events_created_at ON audit_events(created_at);
