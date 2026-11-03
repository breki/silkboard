import {
  identity,
  object,
  page,
  positive,
  projectId,
  request,
  text,
  type AgentIdentity,
  type Page,
} from "./feed-api";
import { taskId } from "./plan-model";
import { filterQuery } from "./list-filters";
export interface AgentFilters {
  project?: string;
  hostname?: string;
  tool?: string;
  session_id?: string;
  q?: string;
}
export interface AgentActivity extends AgentIdentity {
  project: string;
  subagent_id: null;
  parent_session_id: null;
  received_at: number;
  latest:
    | { kind: "event"; event_id: string; message: string }
    | {
        kind: "workflow";
        task_id: string;
        sequence: string;
        title: string;
        state: "pending" | "active" | "blocked" | "completed";
      };
}
function agent(value: unknown): value is AgentActivity {
  if (
    !object(value) ||
    !projectId(value.project) ||
    !identity(value) ||
    value.subagent_id !== null ||
    value.parent_session_id !== null ||
    typeof value.received_at !== "number" ||
    !Number.isSafeInteger(value.received_at) ||
    value.received_at < 0 ||
    value.received_at > 8640000000000000 ||
    !object(value.latest)
  )
    return false;
  const item = value.latest;
  return item.kind === "event"
    ? text(item.event_id, 256) && text(item.message, 2000)
    : item.kind === "workflow" &&
        taskId(item.task_id) &&
        positive(item.sequence) &&
        text(item.title, 160) &&
        ["pending", "active", "blocked", "completed"].includes(
          item.state as string,
        );
}
export const agentKey = (item: AgentActivity) =>
  JSON.stringify([
    item.project,
    item.hostname,
    item.tool,
    item.session_id,
    item.subagent_id,
  ]);
export async function agents(
  before: string | null,
  signal: AbortSignal,
  filters: AgentFilters = {},
): Promise<Page<AgentActivity>> {
  const result = page(
    await request(
      `/api/owner/agents${filterQuery(before, filters)}`,
      {},
      signal,
    ),
    agent,
    (value: unknown): value is string => text(value, 4096),
  );
  if (new Set(result.items.map(agentKey)).size !== result.items.length)
    throw new Error("Invalid agents page");
  return result;
}
