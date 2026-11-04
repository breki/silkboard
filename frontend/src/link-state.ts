import { readable } from "svelte/store";
import { positive, projectId } from "./feed-api";
import { stepId } from "./plan-model";
import { readSession } from "./event-session";
import type { ListFilters } from "./list-filters";

export const locationEvent = "silkboard:location";
const address = () => ({
  pathname: window.location.pathname,
  search: window.location.search,
});
export const locationState = readable({ pathname: "/", search: "" }, (set) => {
  const changed = () => set(address());
  changed();
  window.addEventListener("popstate", changed);
  window.addEventListener(locationEvent, changed);
  return () => {
    window.removeEventListener("popstate", changed);
    window.removeEventListener(locationEvent, changed);
  };
});
import { globalKeys, viewKeys } from "./link-fields";
export { globalKeys, viewKeys } from "./link-fields";
export type ViewKind = keyof typeof viewKeys;
const allKeys = new Set([...globalKeys, ...Object.values(viewKeys).flat()]);
export function navigateUrl(path: string, replace = false) {
  if (
    !path.startsWith("/") ||
    path.startsWith("//") ||
    path.includes("\\") ||
    path.length > 60000
  )
    throw new Error("Invalid application link");
  const url = new window.URL(path, window.location.href);
  if (url.origin !== window.location.origin)
    throw new Error("Invalid application link");
  if (
    url.pathname + url.search + url.hash ===
    window.location.pathname + window.location.search + window.location.hash
  )
    return;
  window.history[replace ? "replaceState" : "pushState"](
    null,
    "",
    url.pathname + url.search + url.hash,
  );
  window.dispatchEvent(new Event(locationEvent));
}
export function updateQuery(
  changes: Record<string, string | number | null | undefined>,
  replace = false,
) {
  const params = new URLSearchParams(window.location.search);
  for (const key of [...params.keys()])
    if (!allKeys.has(key)) params.delete(key);
  for (const [key, value] of Object.entries(changes)) {
    if (!allKeys.has(key)) throw new Error("Unknown view field");
    if (
      value === undefined ||
      value === null ||
      (value === "" && key !== "subagent_id")
    )
      params.delete(key);
    else params.set(key, String(value));
  }
  const query = params.toString();
  navigateUrl(window.location.pathname + (query ? `?${query}` : ""), replace);
}
export interface ViewState {
  // Fields below describe read-only navigation, never pending mutations.
  filters: ListFilters;
  before: string | null;
  hierarchical: boolean;
  event: string | null;
  collapsed: string[];
  agentDetails: string | null;
  step: string | null;
  presentation: "graph" | "steps";
  detailsClosed: boolean;
  history: "history" | "revisions" | null;
  historyBefore: string | null;
  compare: boolean;
  todo: string | null;
  todoMode: "view" | "edit" | "child";
  panel: boolean;
  credentialProject: string;
  credentialAfter: string | null;
}
export function defaultView(): ViewState {
  return {
    filters: {},
    before: null,
    hierarchical: false,
    event: null,
    collapsed: [],
    agentDetails: null,
    step: null,
    presentation: "graph",
    detailsClosed: false,
    history: null,
    historyBefore: null,
    compare: false,
    todo: null,
    todoMode: "view",
    panel: false,
    credentialProject: "",
    credentialAfter: null,
  };
}
const native = (value: unknown): value is string =>
  typeof value === "string" && /^[!-~]{1,256}$/.test(value);
const bounded = (value: string, max: number) =>
  value.length > 0 &&
  new TextEncoder().encode(value).length <= max &&
  [...value].every((char) => {
    const n = char.codePointAt(0)!;
    return n >= 32 && !(n >= 127 && n <= 159);
  });
function identityKey(value: string, children: boolean) {
  const parts: unknown = JSON.parse(value);
  if (
    !Array.isArray(parts) ||
    parts.length !== 5 ||
    !projectId(parts[0]) ||
    !parts.slice(1, 4).every(native) ||
    !(parts[4] === null || (children && native(parts[4])))
  )
    throw new Error("Invalid identity");
  return JSON.stringify(parts);
}
export type ParsedView =
  { valid: true; value: ViewState } | { valid: false; value: ViewState };
/** Decode only meaningful read/navigation state. Drafts and credentials have no URL fields. */
export function readView(view: ViewKind, search: string): ParsedView {
  const value = defaultView();
  try {
    if (search.length > 60000) throw new Error("Link too large");
    const params = new URLSearchParams(search);
    const allowed = new Set<string>([
      ...(view === "setup" ? ["theme"] : globalKeys),
      ...viewKeys[view],
    ]);
    for (const key of params.keys())
      if (!allowed.has(key) || params.getAll(key).length !== 1)
        throw new Error("Unsupported field");
    // eslint-disable-next-line no-unused-vars -- Type-only validation callback.
    const take = (key: string, valid: (value: string) => boolean) => {
      const raw = params.get(key);
      if (raw !== null && !valid(raw)) throw new Error("Invalid field");
      return raw;
    };
    const project = take("project", projectId),
      agent = take("agent", (raw) => bounded(raw, 513));
    if (project !== null) value.filters.project = project;
    if (agent !== null) value.filters.agent = agent;
    for (const key of ["from", "to"] as const) {
      const raw = take(
        key,
        (text) =>
          /^(0|[1-9][0-9]*)$/.test(text) &&
          Number.isSafeInteger(Number(text)) &&
          Number(text) <= 8640000000000000,
      );
      if (raw !== null) value.filters[key] = Number(raw);
    }
    if (
      value.filters.from !== undefined &&
      value.filters.to !== undefined &&
      value.filters.from > value.filters.to
    )
      throw new Error("Reversed range");
    if (view === "events") {
      const session = readSession(search);
      if (session.kind === "invalid") throw new Error("Invalid session");
      if (session.kind === "session")
        value.filters = {
          ...value.filters,
          ...session.value,
          subagent_id: session.value.subagent_id ?? "",
        };
    } else if (
      view === "agents" &&
      ["hostname", "tool", "session_id"].some((key) => params.has(key))
    ) {
      if (!project) throw new Error("Incomplete agent identity");
      for (const key of ["hostname", "tool", "session_id"] as const) {
        const raw = take(key, native);
        if (raw === null) throw new Error("Incomplete agent identity");
        value.filters[key] = raw;
      }
    }
    const state = take("state", (raw) =>
      ["pending", "active", "blocked", "completed"].includes(raw),
    );
    if (state !== null) value.filters.state = state;
    value.before = take("before", (raw) => bounded(raw, 4096));
    value.hierarchical =
      take("view", (raw) => ["flat", "hierarchy"].includes(raw)) ===
      "hierarchy";
    value.event = take("event", (raw) => {
      const key: unknown = JSON.parse(raw);
      return (
        Array.isArray(key) &&
        key.length === 2 &&
        projectId(key[0]) &&
        native(key[1])
      );
    });
    if (value.event) value.event = JSON.stringify(JSON.parse(value.event));
    value.agentDetails = take(
      "agent_details",
      (raw) => !!identityKey(raw, false),
    );
    if (value.agentDetails)
      value.agentDetails = identityKey(value.agentDetails, false);
    const collapsed = params.get("collapsed");
    if (collapsed !== null) {
      const keys: unknown = JSON.parse(collapsed);
      if (
        !Array.isArray(keys) ||
        keys.length > 20 ||
        !keys.every(
          (key) => typeof key === "string" && identityKey(key, true),
        ) ||
        new Set(keys).size !== keys.length
      )
        throw new Error("Invalid groups");
      value.collapsed = keys.map((key) => identityKey(key, true));
      if (new Set(value.collapsed).size !== value.collapsed.length)
        throw new Error("Repeated groups");
    }
    take("sequence", positive);
    value.step = take("step", stepId);
    value.presentation =
      take("presentation", (raw) => ["graph", "steps"].includes(raw)) ===
      "steps"
        ? "steps"
        : "graph";
    value.detailsClosed =
      take("details", (raw) => raw === "closed") === "closed";
    if (value.detailsClosed && !value.step) throw new Error("Missing step");
    const history = take("history", (raw) =>
      ["reports", "revisions"].includes(raw),
    );
    value.history =
      history === "reports"
        ? "history"
        : history === "revisions"
          ? "revisions"
          : null;
    value.historyBefore = take("history_before", positive);
    if (value.historyBefore && !value.history)
      throw new Error("Missing history");
    value.compare = take("compare", (raw) => raw === "previous") !== null;
    value.todo = take("todo", positive);
    value.todoMode =
      (take("mode", (raw) =>
        ["view", "edit", "child"].includes(raw),
      ) as ViewState["todoMode"]) ?? "view";
    if (value.todoMode !== "view" && !value.todo)
      throw new Error("Missing todo");
    value.panel = take("panel", (raw) => raw === "credentials") !== null;
    value.credentialProject = take("credential_project", projectId) ?? "";
    value.credentialAfter = take("credential_after", native);
    if ((value.credentialProject || value.credentialAfter) && !value.panel)
      throw new Error("Missing panel");
    if (value.credentialAfter && !value.credentialProject)
      throw new Error("Missing project");
    take("theme", (raw) => ["light", "dark", "system"].includes(raw));
    return { valid: true, value };
  } catch {
    return { valid: false, value: defaultView() };
  }
}

export function filterFields(filters: ListFilters) {
  const result: Record<string, string | number | null | undefined> = {
    before: null,
    event: null,
    agent_details: null,
    collapsed: null,
  };
  for (const key of [
    "project",
    "agent",
    "from",
    "to",
    "state",
    "hostname",
    "tool",
    "session_id",
    "subagent_id",
  ] as const)
    result[key] = filters[key] ?? null;
  return result;
}
export function featureLink(path: string, search = window.location.search) {
  const target = new window.URL(path, window.location.origin),
    source = new URLSearchParams(search);
  for (const key of globalKeys)
    if (source.has(key) && !target.searchParams.has(key))
      target.searchParams.set(key, source.get(key)!);
  return target.pathname + target.search;
}

export function credentialState(search: string) {
  const source = new URLSearchParams(search),
    params = new URLSearchParams();
  for (const key of globalKeys)
    for (const value of source.getAll(key)) params.append(key, value);
  return readView("status", params.toString()).value;
}

/** Build the same href for native browser gestures and in-app navigation. */
export function workflowLink(path: string, search = window.location.search) {
  const target = new window.URL(
    featureLink(path, search),
    window.location.origin,
  );
  if (
    target.pathname === window.location.pathname &&
    target.pathname.startsWith("/plans/")
  ) {
    const previous = new URLSearchParams(search);
    for (const key of viewKeys.workflow)
      if (
        key !== "sequence" &&
        previous.has(key) &&
        !target.searchParams.has(key)
      )
        target.searchParams.set(key, previous.get(key)!);
  }
  return target.pathname + target.search;
}
