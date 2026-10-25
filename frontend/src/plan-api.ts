import {
  ApiError,
  request,
  object,
  identity,
  projectId,
  positive,
  page,
  text,
  type Page,
  type AgentIdentity,
} from "./feed-api";
import { graphLayout, taskId, stepId, type PlanStep } from "./plan-model";

export interface PlanSummary {
  project: string;
  task_id: string;
  sequence: string;
  plan_revision: string;
  writer_generation: string;
  title: string;
  reporter: AgentIdentity;
  state: "pending" | "active" | "completed" | "blocked";
  counts: { total: number; done: number; active: number; blocked: number };
  active_step_ids: string[];
  blocked_step_ids: string[];
  received_at: number;
  history_incomplete: boolean;
}
export interface PlanSnapshot {
  schema_version: 1;
  task_id: string;
  sequence: string;
  plan_revision: string;
  writer_generation: string;
  title: string;
  description: string | null;
  reporter: AgentIdentity;
  github_issue: { repository: string; number: string } | null;
  steps: PlanStep[];
}
export interface PlanDetail {
  snapshot: PlanSnapshot;
  summary: PlanSummary;
  prior_completions: {
    step_id: string;
    sequence: string;
    received_at: number;
  }[];
}
const stamp = (v: unknown): v is number =>
  typeof v === "number" &&
  Number.isSafeInteger(v) &&
  v >= 0 &&
  v <= 8640000000000000;
const words = (v: unknown, max: number, multiline = false): v is string =>
  typeof v === "string" &&
  !!v.trim() &&
  new TextEncoder().encode(v).length <= max &&
  !(
    multiline
      ? // eslint-disable-next-line no-control-regex -- Contract rejects controls but permits Markdown whitespace.
        /[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f-\u009f]/
      : // eslint-disable-next-line no-control-regex -- Single-line contract fields reject control characters.
        /[\u0000-\u001f\u007f-\u009f]/
  ).test(v);
const optional = (v: unknown, max: number) => v === null || words(v, max, true);
const ids = (v: unknown): v is string[] =>
  Array.isArray(v) &&
  v.length <= 128 &&
  v.every(stepId) &&
  new Set(v).size === v.length;
const count = (v: unknown, max: number): v is number =>
  typeof v === "number" && Number.isInteger(v) && v >= 0 && v <= max;
function summary(v: unknown): v is PlanSummary {
  if (
    !object(v) ||
    !projectId(v.project) ||
    !taskId(v.task_id) ||
    !positive(v.sequence) ||
    !positive(v.plan_revision) ||
    !positive(v.writer_generation) ||
    !words(v.title, 160) ||
    !object(v.reporter) ||
    !identity(v.reporter) ||
    !stamp(v.received_at) ||
    typeof v.history_incomplete !== "boolean" ||
    !object(v.counts) ||
    !ids(v.active_step_ids) ||
    !ids(v.blocked_step_ids)
  )
    return false;
  const c = v.counts,
    blocked = v.blocked_step_ids;
  if (
    !count(c.total, 128) ||
    c.total === 0 ||
    !count(c.done, c.total) ||
    !count(c.active, c.total) ||
    !count(c.blocked, c.total) ||
    c.done + c.active + c.blocked > c.total ||
    v.active_step_ids.length !== c.active ||
    v.blocked_step_ids.length !== c.blocked ||
    v.active_step_ids.some((id) => blocked.includes(id))
  )
    return false;
  return (
    v.state ===
    (c.done === c.total
      ? "completed"
      : c.active
        ? "active"
        : c.blocked
          ? "blocked"
          : "pending")
  );
}
function step(v: unknown): v is PlanStep {
  return (
    object(v) &&
    stepId(v.id) &&
    words(v.title, 160) &&
    optional(v.description, 2000) &&
    optional(v.report, 2000) &&
    ids(v.depends_on) &&
    v.depends_on.length <= 32 &&
    ["pending", "active", "done", "blocked"].includes(v.state as string) &&
    (v.state === "blocked"
      ? words(v.blocked_reason, 1000, true)
      : v.blocked_reason === null)
  );
}
function issue(v: unknown) {
  return (
    v === null ||
    (object(v) &&
      typeof v.repository === "string" &&
      v.repository.length <= 100 &&
      v.repository.split("/").length === 2 &&
      v.repository
        .split("/")
        .every((p) => /^[A-Za-z0-9._-]+$/.test(p) && p !== "." && p !== "..") &&
      positive(v.number))
  );
}
function detail(v: unknown): v is PlanDetail {
  if (
    !object(v) ||
    !summary(v.summary) ||
    !object(v.snapshot) ||
    !Array.isArray(v.prior_completions) ||
    v.prior_completions.length > 128
  )
    return false;
  const s = v.snapshot,
    sum = v.summary;
  if (
    s.schema_version !== 1 ||
    s.task_id !== sum.task_id ||
    s.sequence !== sum.sequence ||
    s.plan_revision !== sum.plan_revision ||
    s.writer_generation !== sum.writer_generation ||
    s.title !== sum.title ||
    !optional(s.description, 2000) ||
    !object(s.reporter) ||
    !identity(s.reporter) ||
    !issue(s.github_issue) ||
    !Array.isArray(s.steps) ||
    !s.steps.every(step) ||
    !s.steps.length ||
    s.steps.length > 128
  )
    return false;
  const steps = s.steps as PlanStep[],
    reporter = s.reporter;
  if (
    ![
      "hostname",
      "tool",
      "session_id",
      "subagent_id",
      "parent_session_id",
      "session_name",
    ].every((k) => reporter[k] === sum.reporter[k as keyof AgentIdentity]) ||
    (s.reporter.parent_session_id !== null && s.reporter.subagent_id === null)
  )
    return false;
  try {
    graphLayout(steps);
  } catch {
    return false;
  }
  const byId = new Map(steps.map((s) => [s.id, s]));
  if (
    steps.some(
      (s) =>
        (s.state === "active" || s.state === "done") &&
        s.depends_on.some((id) => byId.get(id)?.state !== "done"),
    )
  )
    return false;
  if (
    sum.counts.total !== steps.length ||
    (["done", "active", "blocked"] as const).some(
      (state) =>
        sum.counts[state] !== steps.filter((s) => s.state === state).length,
    ) ||
    !sum.active_step_ids.every((id) => byId.get(id)?.state === "active") ||
    !sum.blocked_step_ids.every((id) => byId.get(id)?.state === "blocked")
  )
    return false;
  const completions = new Set();
  for (const c of v.prior_completions) {
    if (
      !object(c) ||
      !stepId(c.step_id) ||
      !byId.has(c.step_id) ||
      byId.get(c.step_id)?.state === "done" ||
      completions.has(c.step_id) ||
      !positive(c.sequence) ||
      BigInt(c.sequence) > BigInt(sum.sequence) ||
      !stamp(c.received_at)
    )
      return false;
    completions.add(c.step_id);
  }
  return new TextEncoder().encode(JSON.stringify(s)).length <= 262144;
}
function root(project: string, task: string) {
  if (!projectId(project) || !taskId(task)) throw new ApiError(0);
  return `/api/owner/projects/${project}/plans/${task}`;
}
export async function listPlans(
  before: string | null,
  signal: AbortSignal,
): Promise<Page<PlanSummary>> {
  const result = page(
    await request(
      `/api/owner/plans${before === null ? "" : `?before=${encodeURIComponent(before)}`}`,
      {},
      signal,
    ),
    summary,
    (v: unknown): v is string => text(v, 2048),
  );
  if (
    new Set(result.items.map((s) => `${s.project}/${s.task_id}`)).size !==
    result.items.length
  )
    throw new ApiError(0);
  return result;
}
export async function readPlan(
  project: string,
  task: string,
  selector: { sequence?: string; revision?: string },
  signal: AbortSignal,
): Promise<PlanDetail> {
  if (
    (selector.sequence !== undefined && !positive(selector.sequence)) ||
    (selector.revision !== undefined && !positive(selector.revision)) ||
    (selector.sequence !== undefined && selector.revision !== undefined)
  )
    throw new ApiError(0);
  const query =
    selector.sequence !== undefined
      ? `?sequence=${selector.sequence}`
      : selector.revision !== undefined
        ? `?revision=${selector.revision}`
        : "";
  const result = await request(root(project, task) + query, {}, signal);
  if (
    !detail(result) ||
    result.summary.project !== project ||
    result.summary.task_id !== task ||
    (selector.sequence !== undefined &&
      result.summary.sequence !== selector.sequence) ||
    (selector.revision !== undefined &&
      result.summary.plan_revision !== selector.revision)
  )
    throw new ApiError(0);
  return result;
}
export async function planHistory(
  project: string,
  task: string,
  kind: "history" | "revisions",
  before: string | null,
  signal: AbortSignal,
): Promise<Page<PlanSummary>> {
  if (before !== null && !positive(before)) throw new ApiError(0);
  const result = page(
    await request(
      `${root(project, task)}/${kind}${before === null ? "" : `?before=${before}`}`,
      {},
      signal,
    ),
    summary,
    positive,
  );
  if (
    result.items.some((s) => s.project !== project || s.task_id !== task) ||
    new Set(
      result.items.map((s) =>
        kind === "history" ? s.sequence : s.plan_revision,
      ),
    ).size !== result.items.length
  )
    throw new ApiError(0);
  return result;
}
