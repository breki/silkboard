CREATE TABLE owner (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    password_hash TEXT NOT NULL,
    generation INTEGER NOT NULL
);
CREATE TABLE owner_sessions (
    token_hash BLOB PRIMARY KEY NOT NULL,
    expires_at INTEGER NOT NULL
);
PRAGMA user_version = 2;
