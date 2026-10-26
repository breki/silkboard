import { afterEach, expect, test, vi } from "vitest";
import { agents, agentKey } from "./agents-api";
import { agent } from "../test/agents";
import { page, response } from "../test/support";
afterEach(() => vi.unstubAllGlobals());
const read = () => agents(null, new AbortController().signal);
test("accepts both activities and keys every native identity component", async () => {
  const event = {
    ...agent,
    session_id: "other",
    latest: { kind: "event", event_id: "event", message: "Observed" },
  };
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response(page([agent, event]))),
  );
  expect((await read()).items).toEqual([agent, event]);
  for (const field of [
    "project",
    "hostname",
    "tool",
    "session_id",
    "subagent_id",
  ])
    expect(agentKey({ ...agent, [field]: "different" })).not.toBe(
      agentKey(agent),
    );
});
test.each([
  null,
  {},
  { ...agent, project: "WRONG" },
  { ...agent, subagent_id: "child" },
  { ...agent, parent_session_id: "parent" },
  { ...agent, received_at: -1 },
  { ...agent, received_at: 8640000000000001 },
  { ...agent, received_at: 1.5 },
  { ...agent, latest: null },
  { ...agent, latest: { kind: "unknown" } },
  { ...agent, latest: { ...agent.latest, sequence: "0" } },
  { ...agent, latest: { ...agent.latest, state: "done" } },
  { ...agent, latest: { ...agent.latest, task_id: "bad" } },
  { ...agent, latest: { kind: "event", event_id: "", message: "hi" } },
  { ...agent, latest: { kind: "event", event_id: "ok", message: "" } },
])("rejects malformed agent %j", async (item) => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response(page([item]))),
  );
  await expect(read()).rejects.toThrow();
});
test("rejects repeated identities and oversized cursors", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response(page([agent, agent]))),
  );
  await expect(read()).rejects.toThrow();
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response(page([], "x".repeat(4097)))),
  );
  await expect(read()).rejects.toThrow();
});
