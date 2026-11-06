-- Old applications ignore this additive component and retain readable todos.
CREATE TABLE todo_layout_schema (
 singleton INTEGER PRIMARY KEY CHECK(singleton=1),
 version INTEGER NOT NULL CHECK(version>0),
 revision INTEGER NOT NULL CHECK(revision>=0)
);
INSERT INTO todo_layout_schema VALUES(1,1,0);
CREATE TABLE todo_layout (
 id INTEGER PRIMARY KEY CHECK(id>0),
 parent_id INTEGER CHECK(parent_id>0 AND parent_id<>id),
 position INTEGER NOT NULL UNIQUE CHECK(position>=0)
);
-- No foreign key: old writers can delete todos before a new reader reconciles.
CREATE INDEX todo_layout_parent ON todo_layout(parent_id,position);
