export const csrf = "c".repeat(64);
export const message = {
  event_id: "event-1",
  sequence: "1",
  received_at: 1792300000000,
  message: "Running tests",
};
export const session = {
  id: "1",
  hostname: "dev",
  tool: "codex",
  session_id: "native-1",
  subagent_id: null,
  parent_session_id: null,
  session_name: "Fix the tests",
  latest: message,
};
export const feedEvent = {
  ...message,
  project: "silkboard",
  hostname: session.hostname,
  tool: session.tool,
  session_id: session.session_id,
  subagent_id: null,
  parent_session_id: null,
  session_name: session.session_name,
  message: "Fix the tests",
};
export const page = (items, next_cursor = null) => ({ items, next_cursor });
export const response = (body, status = 200) => ({
  ok: status >= 200 && status < 300,
  status,
  json: async () => body,
});
export const routes =
  (overrides = {}) =>
  async (path, options) => {
    if (path in overrides) {
      const value = overrides[path];
      return typeof value === "function" ? value(options) : value;
    }
    if (path === "/api/status")
      return response({
        health: "ok",
        version: "0.2.2",
        release_tag: "unknown",
        commit: "unknown",
      });
    if (path === "/api/owner/session") return response({}, 401);
    if (path === "/api/owner/events") return response(page([feedEvent]));
    if (path === "/api/owner/projects")
      return response(page([{ id: "silkboard" }]));
    if (path.endsWith("/sessions")) return response(page([session]));
    if (path.endsWith("/events")) return response(page([message]));
    throw new Error("Unexpected test request");
  };
