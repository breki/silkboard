-- Additive checklist component; base user_version stays 3 for old readers.
CREATE TABLE todo_schema (
 singleton INTEGER PRIMARY KEY CHECK(singleton=1),
 version INTEGER NOT NULL CHECK(version>0),
 revision INTEGER NOT NULL CHECK(revision>=0)
);
INSERT INTO todo_schema VALUES(1,1,0);
CREATE TABLE todos (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 parent_id INTEGER REFERENCES todos(id) ON DELETE RESTRICT CHECK(parent_id>0 AND parent_id<id),
 text TEXT NOT NULL CHECK(length(CAST(text AS BLOB)) BETWEEN 1 AND 1000),
 completed INTEGER NOT NULL DEFAULT 0 CHECK(completed IN (0,1))
);
CREATE INDEX todos_parent ON todos(parent_id,id);
