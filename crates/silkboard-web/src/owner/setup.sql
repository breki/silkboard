CREATE TABLE owner_setup (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    token_hash BLOB,
    expires_at INTEGER NOT NULL DEFAULT 0,
    closed INTEGER NOT NULL CHECK (closed IN (0, 1))
);
INSERT INTO owner_setup (id, closed)
VALUES (1, EXISTS(SELECT 1 FROM owner));
PRAGMA user_version = 3;
