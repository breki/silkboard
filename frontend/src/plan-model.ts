import { globalKeys, viewKeys } from "./link-fields";
// eslint-disable-next-line no-unused-vars -- Type-only callback parameter names describe the shared interface.
export type PlanLinkHandler = (event: MouseEvent, path: string) => void;
// eslint-disable-next-line no-unused-vars -- Type-only callback parameter name describes the shared interface.
export type PlanSelector = (value: string) => void;
import { positive, projectId } from "./feed-api";

export type StepState = "pending" | "active" | "done" | "blocked";
export interface PlanStep {
  id: string;
  title: string;
  description: string | null;
  depends_on: string[];
  state: StepState;
  report: string | null;
  blocked_reason: string | null;
}
interface GraphStep {
  id: string;
  depends_on: string[];
}
export const taskId = (value: unknown): value is string =>
  typeof value === "string" &&
  /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(
    value,
  );
export const stepId = (value: unknown): value is string =>
  typeof value === "string" && /^[a-z0-9][a-z0-9_-]{0,63}$/.test(value);
export const stateLabel: Record<StepState, string> = {
  pending: "Pending",
  active: "Active",
  done: "Done",
  blocked: "Blocked",
};
export const stateSymbol: Record<StepState, string> = {
  pending: "○",
  active: "●",
  done: "✓",
  blocked: "!",
};

/** Bounded, deterministic prerequisite layout; statuses never affect geometry. */
export function graphLayout(steps: GraphStep[], columns = 3) {
  if (!steps.length || steps.length > 128 || ![1, 3].includes(columns))
    throw new Error("Invalid task graph");
  const index = new Map(steps.map((step, i) => [step.id, i]));
  if (index.size !== steps.length) throw new Error("Invalid task graph");
  const pending = steps.map((s) => s.depends_on.length);
  const children = steps.map(() => [] as number[]);
  const depth = steps.map(() => 0);
  let edgeCount = 0;
  steps.forEach((step, i) => {
    if (
      !stepId(step.id) ||
      new Set(step.depends_on).size !== step.depends_on.length
    )
      throw new Error("Invalid task graph");
    for (const prerequisite of step.depends_on) {
      const parent = index.get(prerequisite);
      if (parent === undefined || parent === i || ++edgeCount > 512)
        throw new Error("Invalid task graph");
      children[parent].push(i);
    }
  });
  const ready = pending.flatMap((n, i) => (n === 0 ? [i] : []));
  let visited = 0;
  while (ready.length) {
    const i = ready.shift()!;
    visited++;
    for (const child of children[i]) {
      depth[child] = Math.max(depth[child], depth[i] + 1);
      if (--pending[child] === 0) ready.push(child);
    }
  }
  if (visited !== steps.length) throw new Error("Invalid task graph");
  const width = columns * 224 + 32;
  const nodes: { id: string; x: number; y: number }[] = [];
  let y = 16;
  for (let level = 0; level <= Math.max(...depth); level++) {
    const layer = steps.filter((_, i) => depth[i] === level);
    layer.forEach((step, i) => {
      const row = Math.floor(i / columns);
      const rowSize = Math.min(columns, layer.length - row * columns);
      nodes.push({
        id: step.id,
        x: 16 + (width - 32 - rowSize * 224) / 2 + (i % columns) * 224,
        y: y + row * 136,
      });
    });
    y += Math.ceil(layer.length / columns) * 136 + 16;
  }
  const positioned = new Map(nodes.map((n) => [n.id, n]));
  const edges = steps.flatMap((step) =>
    [...step.depends_on].sort().map((from) => {
      const source = positioned.get(from)!,
        target = positioned.get(step.id)!;
      // Side lanes keep long prerequisite edges out of step labels.
      const lane = 2 + (index.get(from)! % 4) * 4;
      const sx = source.x + 104,
        sy = source.y + 112,
        tx = target.x + 104;
      if (target.y - sy <= 40) {
        const middle = sy + (target.y - sy) / 2;
        return {
          from,
          to: step.id,
          path: `M${sx} ${sy} V${middle} H${tx} V${target.y}`,
        };
      }
      return {
        from,
        to: step.id,
        path: `M${sx} ${sy} V${sy + 8} H${lane} V${target.y - 8} H${tx} V${target.y}`,
      };
    }),
  );
  return { nodes, edges, width, height: y };
}

export type PlanRoute =
  | { kind: "list" }
  | { kind: "invalid" }
  | { kind: "task"; project: string; task: string; sequence: string | null };
export function routeFor(
  project: string,
  task: string,
  sequence: string | null = null,
) {
  if (
    !projectId(project) ||
    !taskId(task) ||
    (sequence !== null && !positive(sequence))
  )
    throw new Error("Invalid task link");
  return `/plans/${project}/${task}${sequence === null ? "" : `?sequence=${sequence}`}`;
}
export function parsePlanRoute(path: string): PlanRoute {
  if (
    !path.startsWith("/") ||
    path.startsWith("//") ||
    path.length > 65536 ||
    path.includes("\\") ||
    path.includes("#")
  )
    return { kind: "invalid" };
  const url = new URL(path, "https://silkboard.invalid");
  const scope =
    url.pathname === "/plans" ? viewKeys.workflows : viewKeys.workflow;
  const allowed = new Set<string>([...globalKeys, ...scope]);
  if (
    [...url.searchParams.keys()].some(
      (key) => !allowed.has(key) || url.searchParams.getAll(key).length !== 1,
    )
  )
    return { kind: "invalid" };
  if (url.pathname === "/plans") return { kind: "list" };
  const parts = url.pathname.split("/");
  const sequence = url.searchParams.get("sequence");
  if (
    parts.length !== 4 ||
    parts[1] !== "plans" ||
    !projectId(parts[2]) ||
    !taskId(parts[3]) ||
    (sequence !== null && !positive(sequence))
  )
    return { kind: "invalid" };
  return { kind: "task", project: parts[2], task: parts[3], sequence };
}
export function reportAge(received: number, now = Date.now()) {
  const minutes = Math.max(0, Math.floor((now - received) / 60000));
  if (!minutes) return "just received";
  if (minutes < 60) return `${minutes} minute${minutes === 1 ? "" : "s"} ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} hour${hours === 1 ? "" : "s"} ago`;
  const days = Math.floor(hours / 24);
  return `${days} day${days === 1 ? "" : "s"} ago`;
}

/** Compare only received definitions/states; never fill gaps with guessed work. */
export function receivedChanges(
  before: import("./plan-api").PlanSnapshot,
  after: import("./plan-api").PlanSnapshot,
) {
  const old = new Map(before.steps.map((s) => [s.id, s]));
  const current = new Map(after.steps.map((s) => [s.id, s]));
  return {
    added: after.steps.filter((s) => !old.has(s.id)),
    removed: before.steps.filter((s) => !current.has(s.id)),
    prerequisites: after.steps.filter(
      (s) =>
        old.has(s.id) &&
        [...s.depends_on].sort().join("/") !==
          [...old.get(s.id)!.depends_on].sort().join("/"),
    ),
    definitions: after.steps.filter(
      (s) =>
        old.has(s.id) &&
        (old.get(s.id)!.title !== s.title ||
          old.get(s.id)!.description !== s.description),
    ),
    states: after.steps
      .filter((s) => old.has(s.id) && old.get(s.id)!.state !== s.state)
      .map((s) => ({
        id: s.id,
        title: s.title,
        from: old.get(s.id)!.state,
        to: s.state,
      })),
    gap: BigInt(after.sequence) > BigInt(before.sequence) + BigInt(1),
    continuity: before.writer_generation !== after.writer_generation,
  };
}
