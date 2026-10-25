export interface Page<T> {
  items: T[];
  next_cursor: string | null;
}
export interface Project {
  id: string;
}
export interface Publisher {
  label: string;
  revoked: boolean;
}
export interface Message {
  event_id: string;
  sequence: string;
  received_at: number;
  message: string;
}
export interface AgentIdentity {
  hostname: string;
  tool: string;
  session_id: string;
  subagent_id: string | null;
  parent_session_id: string | null;
  session_name: string | null;
}
export interface Session extends AgentIdentity {
  id: string;
  latest: Message;
}
export interface FeedEvent extends AgentIdentity, Message {
  project: string;
}

export class ApiError extends Error {
  status: number;
  constructor(status: number) {
    super("Request failed");
    this.status = status;
  }
}

export async function request(
  path: string,
  options: {
    method?: string;
    headers?: Record<string, string>;
    body?: string;
  } = {},
  signal?: AbortSignal,
): Promise<unknown> {
  const controller = new AbortController();
  const abort = () => controller.abort();
  signal?.addEventListener("abort", abort, { once: true });
  if (signal?.aborted) abort();
  const timeout = setTimeout(abort, 10000);
  try {
    const response = await fetch(path, {
      ...options,
      cache: "no-store",
      credentials: "same-origin",
      signal: controller.signal,
    });
    if (controller.signal.aborted) throw new ApiError(0);
    if (!response.ok) throw new ApiError(response.status);
    return response.status === 204 ? null : await response.json();
  } finally {
    clearTimeout(timeout);
    signal?.removeEventListener("abort", abort);
  }
}

export const object = (value: unknown): value is Record<string, unknown> =>
  value !== null && typeof value === "object" && !Array.isArray(value);
export const text = (value: unknown, max: number): value is string =>
  typeof value === "string" && value.length > 0 && value.length <= max;
export const nullable = (value: unknown, max: number) =>
  value === null || text(value, max);
export const projectId = (value: unknown): value is string =>
  typeof value === "string" && /^[a-z0-9-]{1,64}$/.test(value);
export const positive = (value: unknown): value is string =>
  typeof value === "string" &&
  /^[1-9][0-9]{0,18}$/.test(value) &&
  (value.length < 19 || value <= "9223372036854775807");
const project = (value: unknown): value is Project =>
  object(value) && projectId(value.id);
const message = (value: unknown): value is Message =>
  object(value) &&
  text(value.event_id, 256) &&
  positive(value.sequence) &&
  typeof value.received_at === "number" &&
  Number.isSafeInteger(value.received_at) &&
  value.received_at >= 0 &&
  value.received_at <= 8640000000000000 &&
  text(value.message, 2000);
export const identity = (value: Record<string, unknown>) =>
  text(value.hostname, 256) &&
  text(value.tool, 256) &&
  text(value.session_id, 256) &&
  nullable(value.subagent_id, 256) &&
  nullable(value.parent_session_id, 256) &&
  nullable(value.session_name, 120);
const session = (value: unknown): value is Session =>
  object(value) &&
  positive(value.id) &&
  identity(value) &&
  message(value.latest);
const feedEvent = (value: unknown): value is FeedEvent =>
  object(value) &&
  projectId(value.project) &&
  identity(value) &&
  message(value);

export function page<T>(
  value: unknown,
  valid: (item: unknown) => item is T,
  cursor: typeof positive,
): Page<T> {
  if (
    !object(value) ||
    !Array.isArray(value.items) ||
    value.items.length > 20 ||
    !value.items.every(valid) ||
    !(value.next_cursor === null || cursor(value.next_cursor)) ||
    (value.items.length === 0 && value.next_cursor !== null)
  )
    throw new ApiError(0);
  return {
    items: value.items,
    next_cursor: value.next_cursor as string | null,
  };
}

export async function ownerSession(
  signal?: AbortSignal,
): Promise<{ csrf_token: string }> {
  const value = await request("/api/owner/session", {}, signal);
  if (
    !object(value) ||
    value.authenticated !== true ||
    typeof value.csrf_token !== "string" ||
    !/^[0-9a-f]{64}$/.test(value.csrf_token)
  )
    throw new ApiError(0);
  return { csrf_token: value.csrf_token };
}

export async function signIn(
  password: string,
  signal?: AbortSignal,
): Promise<void> {
  await request(
    "/api/auth/login",
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ password }),
    },
    signal,
  );
}
export async function signOut(signal?: AbortSignal): Promise<void> {
  const { csrf_token } = await ownerSession(signal);
  await request(
    "/api/owner/logout",
    { method: "POST", headers: { "X-Silkboard-CSRF": csrf_token } },
    signal,
  );
}

export async function setupState(signal: AbortSignal): Promise<string> {
  const value = await request("/api/auth/setup", {}, signal);
  if (
    !object(value) ||
    !["available", "waiting", "closed"].includes(value.state as string)
  )
    throw new ApiError(0);
  return value.state as string;
}

export async function registerOwner(
  credential: string,
  password: string,
  signal: AbortSignal,
): Promise<void> {
  await request(
    "/api/auth/setup",
    {
      method: "POST",
      headers: { "Content-Type": "application/json", "X-Silkboard-Setup": "1" },
      body: JSON.stringify({ credential, password }),
    },
    signal,
  );
}
export async function projects(
  after: string | null,
  signal: AbortSignal,
): Promise<Page<Project>> {
  return page(
    await request(
      `/api/owner/projects${after === null ? "" : `?after=${encodeURIComponent(after)}`}`,
      {},
      signal,
    ),
    project,
    projectId,
  );
}

const publisherLabel = (value: unknown): value is string =>
  typeof value === "string" && /^[!-~]{1,256}$/.test(value);
export async function publisherCredentials(
  project: string,
  after: string | null,
  signal: AbortSignal,
): Promise<Page<Publisher>> {
  return page(
    await request(
      `/api/owner/projects/${encodeURIComponent(project)}/publishers${after === null ? "" : `?after=${encodeURIComponent(after)}`}`,
      {},
      signal,
    ),
    (item): item is Publisher =>
      object(item) &&
      Object.keys(item).length === 2 &&
      publisherLabel(item.label) &&
      typeof item.revoked === "boolean",
    publisherLabel,
  );
}
async function credentialMutation(
  project: string,
  label: string,
  revoke: boolean,
  signal: AbortSignal,
): Promise<unknown> {
  const { csrf_token } = await ownerSession(signal);
  return request(
    `/api/owner/projects/${encodeURIComponent(project)}/publishers${revoke ? "/revoke" : ""}`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Silkboard-CSRF": csrf_token,
      },
      body: JSON.stringify({ label }),
    },
    signal,
  );
}
export async function issuePublisher(
  project: string,
  label: string,
  signal: AbortSignal,
): Promise<string> {
  const value = await credentialMutation(project, label, false, signal);
  if (
    !object(value) ||
    Object.keys(value).length !== 1 ||
    typeof value.token !== "string" ||
    !/^[0-9a-f]{64}$/.test(value.token)
  )
    throw new ApiError(0);
  return value.token;
}
export async function revokePublisher(
  project: string,
  label: string,
  signal: AbortSignal,
): Promise<void> {
  await credentialMutation(project, label, true, signal);
}
export async function sessions(
  project: string,
  before: string | null,
  signal: AbortSignal,
): Promise<Page<Session>> {
  return page(
    await request(
      `/api/owner/projects/${encodeURIComponent(project)}/sessions${before === null ? "" : `?before=${encodeURIComponent(before)}`}`,
      {},
      signal,
    ),
    session,
    positive,
  );
}
export async function history(
  project: string,
  id: string,
  before: string | null,
  signal: AbortSignal,
): Promise<Page<Message>> {
  return page(
    await request(
      `/api/owner/projects/${encodeURIComponent(project)}/sessions/${encodeURIComponent(id)}/events${before === null ? "" : `?before=${encodeURIComponent(before)}`}`,
      {},
      signal,
    ),
    message,
    positive,
  );
}

export async function events(
  before: string | null,
  signal: AbortSignal,
): Promise<Page<FeedEvent>> {
  return page(
    await request(
      `/api/owner/events${before === null ? "" : `?before=${encodeURIComponent(before)}`}`,
      {},
      signal,
    ),
    feedEvent,
    (value: unknown): value is string => text(value, 2048),
  );
}
