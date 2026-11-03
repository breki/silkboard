SELECT e.project AS value FROM events e JOIN sessions s ON s.id=e.session
WHERE s.child_id='' AND s.parent_id IS NULL
UNION ALL
SELECT project AS value FROM plan_snapshots
WHERE json_extract(summary,'$.reporter.subagent_id') IS NULL
  AND json_extract(summary,'$.reporter.parent_session_id') IS NULL
