import { beforeEach, expect, test } from "vitest";
import { get } from "svelte/store";
import {
  locationState,
  updateQuery,
  readView,
  defaultView,
  viewKeys,
} from "./link-state";
beforeEach(() => window.history.replaceState(null, "", "/"));
test("filters and exact native identities survive canonical URL encoding", () => {
  const fields = {
    project: "alpha",
    hostname: "host%",
    tool: "codex",
    session_id: "native&one",
    subagent_id: "",
    from: "0",
    to: "1234",
    view: "hierarchy",
    event: JSON.stringify(["alpha", "event-1"]),
  };
  const search = new URLSearchParams(fields).toString();
  const parsed = readView("events", `?${search}`);
  expect(parsed.valid).toBe(true);
  expect(parsed.value.filters).toMatchObject({
    project: "alpha",
    hostname: "host%",
    session_id: "native&one",
    subagent_id: "",
    from: 0,
    to: 1234,
  });
  expect(parsed.value.event).toBe(fields.event);
  expect(parsed.value.hierarchical).toBe(true);
});
test.each([
  "?project=BAD",
  "?project=alpha&project=beta",
  "?from=10&to=1",
  "?from=NaN",
  "?hostname=dev",
  "?view=other",
  "?event=not-json",
  "?token=secret",
  "?panel=credentials&credential_project=BAD",
])("invalid view settings never become data filters: %s", (search) => {
  expect(readView("events", search).valid).toBe(false);
});
test("workflow, todo and credential view state use validated identifiers", () => {
  expect(
    readView(
      "workflow",
      "?sequence=2&step=build&presentation=steps&history=revisions&history_before=3&compare=previous",
    ).value,
  ).toMatchObject({
    step: "build",
    presentation: "steps",
    history: "revisions",
    historyBefore: "3",
    compare: true,
  });
  expect(readView("todos", "?todo=3&mode=edit").value).toMatchObject({
    todo: "3",
    todoMode: "edit",
  });
  expect(
    readView(
      "agents",
      "?panel=credentials&credential_project=alpha&credential_after=dev",
    ).value,
  ).toMatchObject({
    panel: true,
    credentialProject: "alpha",
    credentialAfter: "dev",
  });
  expect(readView("todos", "?todo=0").valid).toBe(false);
  expect(readView("workflow", "?history_before=2").valid).toBe(false);
  expect(readView("agents", "?credential_project=alpha").valid).toBe(false);
  expect(readView("agents", "?hostname=dev").valid).toBe(false);
  expect(defaultView().filters).toEqual({});
  expect(viewKeys.events).toContain("event");
});
test("updates notify mounted views and never carry unknown credential fields forward", () => {
  window.history.replaceState(
    null,
    "",
    "/events?password=never-copy&project=alpha",
  );
  const seen = [];
  const stop = locationState.subscribe((value) => seen.push(value.search));
  updateQuery({ agent: "codex dev", from: 0 });
  expect(get(locationState).search).toContain("agent=codex+dev");
  expect(window.location.search).not.toContain("password");
  expect(seen.at(-1)).toBe(window.location.search);
  stop();
});

test("native navigation rejects external paths and preserves only global state on feature links", async () => {
  const { navigateUrl, featureLink, credentialState } =
    await import("./link-state");
  for (const path of [
    "https://evil.invalid/",
    "//evil.invalid/",
    "/\\evil",
    "x",
    "/" + "a".repeat(65536),
  ])
    expect(() => navigateUrl(path)).toThrow();
  expect(() => updateQuery({ token: "secret" })).toThrow();
  window.history.replaceState(
    null,
    "",
    "/events?project=alpha&theme=dark&panel=credentials&credential_project=alpha",
  );
  expect(featureLink("/todos")).toBe(
    "/todos?panel=credentials&credential_project=alpha&theme=dark",
  );
  expect(
    credentialState(
      "?panel=credentials&credential_project=alpha&credential_project=beta",
    ).panel,
  ).toBe(false);
  const before = window.history.length;
  navigateUrl(window.location.pathname + window.location.search);
  expect(window.history.length).toBe(before);
});
test.each([
  ["agents", "?project=alpha&hostname=dev&tool=codex"],
  ["events", "?from=8640000000000001"],
  ["events", "?to=9007199254740992"],
  ["events", "?agent=%0A"],
  ["events", "?collapsed={}"],
  ["events", "?collapsed=" + encodeURIComponent(JSON.stringify(["bad"]))],
  ["workflow", "?details=closed"],
  ["workflow", "?sequence=0"],
  ["workflow", "?presentation=other"],
  ["workflow", "?compare=issue"],
  ["todos", "?mode=child"],
  ["status", "?panel=credentials&credential_after=dev"],
  ["status", "?theme=unknown"],
  ["setup", "?panel=credentials"],
])("rejects unsupported or incomplete %s combinations", (view, search) =>
  expect(readView(view, search).valid).toBe(false),
);
test("identity disclosures and collapsed groups preserve their complete native identity", () => {
  const root = JSON.stringify(["alpha", "host", "codex", "native", null]);
  const child = JSON.stringify(["alpha", "host", "codex", "native", "child"]);
  expect(
    readView("agents", "?agent_details=" + encodeURIComponent(root)).value
      .agentDetails,
  ).toBe(root);
  expect(
    readView(
      "events",
      "?collapsed=" + encodeURIComponent(JSON.stringify([root, child])),
    ).value.collapsed,
  ).toEqual([root, child]);
  expect(
    readView(
      "events",
      "?collapsed=" + encodeURIComponent(JSON.stringify([root, root])),
    ).valid,
  ).toBe(false);
  expect(
    readView("agents", "?agent_details=" + encodeURIComponent(child)).valid,
  ).toBe(false);
  expect(readView("events", "?" + "a".repeat(65536)).valid).toBe(false);
});

test("equivalent JSON target identities resolve the same selection", () => {
  const parts = ["alpha", "host", "codex", "native", null],
    pretty = JSON.stringify(parts, null, 1),
    canonical = JSON.stringify(parts);
  expect(
    readView("agents", "?agent_details=" + encodeURIComponent(pretty)).value
      .agentDetails,
  ).toBe(canonical);
  expect(
    readView("events", "?event=" + encodeURIComponent('[ "alpha", "event-1" ]'))
      .value.event,
  ).toBe('["alpha","event-1"]');
  expect(
    readView(
      "events",
      "?collapsed=" + encodeURIComponent(JSON.stringify([pretty])),
    ).value.collapsed,
  ).toEqual([canonical]);
  expect(
    readView(
      "events",
      "?collapsed=" + encodeURIComponent(JSON.stringify([pretty, canonical])),
    ).valid,
  ).toBe(false);
});

test("workflow hrefs retain selections for native new-tab and copied links", async () => {
  const { workflowLink } = await import("./link-state");
  window.history.replaceState(
    null,
    "",
    "/plans/alpha/task?step=build&presentation=steps&theme=dark&sequence=3",
  );
  const link = workflowLink("/plans/alpha/task?sequence=1");
  const query = new URLSearchParams(link.split("?")[1]);
  expect(query.get("sequence")).toBe("1");
  expect(query.get("step")).toBe("build");
  expect(query.get("presentation")).toBe("steps");
  expect(query.get("theme")).toBe("dark");
  expect(workflowLink("/plans/alpha/other")).toBe(
    "/plans/alpha/other?theme=dark",
  );
});
