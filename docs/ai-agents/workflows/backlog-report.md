# backlog-report

Read local backlog records and, with available authenticated read access,
list all open GitHub issues using pagination. Distinguish an unavailable issue
source from an empty one. Follow explicit issue links/slugs to merge duplicates;
flag uncertain matches. Group by actual project area and distinguish recorded
priority from a suggested priority. Describe scope, dependencies, complexity,
and what remains unknown without inventing acceptance criteria.

Use [html-report](html-report.md) to produce a local table with source links,
status, and useful comparisons. Include collection time and any incomplete
source. Do not change backlog files, issue bodies, labels, or priorities.
