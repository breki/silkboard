WITH activity AS (
  SELECT e.project,s.hostname,s.tool,s.native_id AS session_id,s.child_id,
    s.parent_id,
    (SELECT session_name FROM events named WHERE named.session=s.id
      AND named.session_name IS NOT NULL AND named.sequence<=e.sequence
      ORDER BY named.sequence DESC LIMIT 1) AS session_name,
    e.received_at,0 AS kind,e.event_id AS item_key,e.sequence,e.message AS title,NULL AS state
  FROM events e JOIN sessions s ON s.id=e.session
  UNION ALL
  SELECT p.project,json_extract(p.summary,'$.reporter.hostname'),
    json_extract(p.summary,'$.reporter.tool'),json_extract(p.summary,'$.reporter.session_id'),
    coalesce(json_extract(p.summary,'$.reporter.subagent_id'),''),
    json_extract(p.summary,'$.reporter.parent_session_id'),
    json_extract(p.summary,'$.reporter.session_name'),
    p.received_at,1,p.task_id,p.sequence,json_extract(p.summary,'$.title'),
    json_extract(p.summary,'$.state')
  FROM plan_snapshots p
), ranked AS (
  SELECT *,row_number() OVER (
    PARTITION BY project,hostname,tool,session_id,child_id
    ORDER BY received_at DESC,kind DESC,item_key DESC,sequence DESC
  ) AS rank FROM activity
)
SELECT project,hostname,tool,session_id,NULLIF(child_id,''),parent_id,session_name,
  received_at,kind,item_key,sequence,title,state
FROM ranked WHERE rank=1 AND child_id='' AND parent_id IS NULL
  AND (?7 IS NULL OR project=?7)
  AND (?8 IS NULL OR hostname=?8)
  AND (?9 IS NULL OR tool=?9)
  AND (?10 IS NULL OR session_id=?10)
  AND (?11 IS NULL OR instr(lower(project || ' ' || coalesce(session_name,'') || ' ' || tool || ' ' || hostname || ' ' || title),lower(?11))>0)
  AND (received_at,project,hostname,tool,session_id,child_id)<(?1,?2,?3,?4,?5,?6)
ORDER BY received_at DESC,project DESC,hostname DESC,tool DESC,session_id DESC,child_id DESC
LIMIT 21
