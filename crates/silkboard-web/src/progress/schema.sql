CREATE TABLE projects (id TEXT PRIMARY KEY NOT NULL);
CREATE TABLE publishers (
    project TEXT NOT NULL REFERENCES projects(id),
    label TEXT NOT NULL,
    token_hash BLOB NOT NULL UNIQUE,
    revoked INTEGER NOT NULL DEFAULT 0 CHECK (revoked IN (0, 1)),
    PRIMARY KEY (project, label)
);
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY,
    project TEXT NOT NULL REFERENCES projects(id),
    hostname TEXT NOT NULL,
    tool TEXT NOT NULL,
    native_id TEXT NOT NULL,
    child_id TEXT NOT NULL,
    parent_id TEXT,
    UNIQUE (project, hostname, tool, native_id, child_id),
    UNIQUE (project, id)
);
CREATE TABLE events (
    project TEXT NOT NULL,
    event_id TEXT NOT NULL,
    session INTEGER NOT NULL,
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    message TEXT NOT NULL,
    session_name TEXT,
    payload TEXT NOT NULL,
    received_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    PRIMARY KEY (project, event_id),
    UNIQUE (session, sequence),
    FOREIGN KEY (project, session) REFERENCES sessions(project, id)
);
PRAGMA user_version = 1;
