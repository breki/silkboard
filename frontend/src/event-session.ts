import { projectId } from "./feed-api";
export interface ExactSession {
  project: string;
  hostname: string;
  tool: string;
  session_id: string;
  subagent_id: string | null;
}
const fields = [
  "project",
  "hostname",
  "tool",
  "session_id",
  "subagent_id",
] as const;
const native = (value: string) => /^[!-~]{1,256}$/.test(value);
export type SessionSelection =
  | { kind: "none" }
  | { kind: "invalid" }
  | { kind: "session"; value: ExactSession };
export function readSession(search: string): SessionSelection {
  const params = new URLSearchParams(search);
  if (!fields.slice(1).some((field) => params.has(field)))
    return { kind: "none" };
  if (fields.some((field) => params.getAll(field).length !== 1))
    return { kind: "invalid" };
  const project = params.get("project")!,
    hostname = params.get("hostname")!,
    tool = params.get("tool")!,
    session_id = params.get("session_id")!,
    child = params.get("subagent_id")!;
  if (
    !projectId(project) ||
    ![hostname, tool, session_id].every(native) ||
    (child !== "" && !native(child))
  )
    return { kind: "invalid" };
  return {
    kind: "session",
    value: { project, hostname, tool, session_id, subagent_id: child || null },
  };
}
export function sessionEvents(identity: ExactSession): string {
  const params = new URLSearchParams();
  for (const field of fields) params.set(field, identity[field] ?? "");
  const selection = readSession(`?${params}`);
  if (selection.kind !== "session") throw new Error("Invalid session identity");
  return `/events?${params}`;
}
export function clearSessionUrl(): string {
  const params = new URLSearchParams(window.location.search);
  for (const field of fields) params.delete(field);
  const query = params.toString();
  return (
    window.location.pathname + (query ? `?${query}` : "") + window.location.hash
  );
}
