-- Additive component schema. Base user_version stays 3 for legacy readers.
CREATE TABLE plan_schema (
    singleton INTEGER PRIMARY KEY CHECK (singleton=1),
    version INTEGER NOT NULL CHECK (version>0)
);
INSERT INTO plan_schema VALUES (1,1);
CREATE TABLE task_plans (
    project TEXT NOT NULL REFERENCES projects(id),
    task_id TEXT NOT NULL,
    latest_sequence INTEGER NOT NULL CHECK (latest_sequence>0),
    received_at INTEGER NOT NULL,
    summary TEXT NOT NULL,
    PRIMARY KEY (project,task_id),
    FOREIGN KEY (project,task_id,latest_sequence) REFERENCES plan_snapshots(project,task_id,sequence) DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX task_plans_feed ON task_plans(received_at DESC,project DESC,task_id DESC);
CREATE TABLE plan_snapshots (
    project TEXT NOT NULL,
    task_id TEXT NOT NULL,
    sequence INTEGER NOT NULL CHECK (sequence>0),
    plan_revision INTEGER NOT NULL CHECK (plan_revision>0),
    writer_generation INTEGER NOT NULL CHECK (writer_generation>0),
    payload TEXT NOT NULL,
    summary TEXT NOT NULL,
    received_at INTEGER NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 AS INTEGER)),
    PRIMARY KEY (project,task_id,sequence),
    FOREIGN KEY (project,task_id) REFERENCES task_plans(project,task_id)
);
CREATE TABLE plan_revisions (
    project TEXT NOT NULL,
    task_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision>0),
    latest_sequence INTEGER NOT NULL CHECK (latest_sequence>0),
    summary TEXT NOT NULL,
    PRIMARY KEY (project,task_id,revision),
    FOREIGN KEY (project,task_id,latest_sequence) REFERENCES plan_snapshots(project,task_id,sequence)
);
CREATE TABLE plan_step_history (
    project TEXT NOT NULL,
    task_id TEXT NOT NULL,
    step_id TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    state TEXT CHECK (state IN ('pending','active','done','blocked') OR state IS NULL),
    PRIMARY KEY (project,task_id,step_id,sequence),
    FOREIGN KEY (project,task_id,sequence) REFERENCES plan_snapshots(project,task_id,sequence)
);
CREATE INDEX plan_step_completed ON plan_step_history(project,task_id,step_id,state,sequence DESC);
