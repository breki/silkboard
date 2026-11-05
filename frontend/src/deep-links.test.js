import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { updateQuery } from "./link-state";
import Agents from "./Agents.svelte";
import EventFeed from "./EventFeed.svelte";
import Todos from "./Todos.svelte";
import PublisherTokens from "./PublisherTokens.svelte";
import TaskPlans from "./TaskPlans.svelte";
import App from "./App.svelte";
import { agent } from "../test/agents";
import { detail, task } from "../test/plans";
import { feedEvent as message, page, response } from "../test/support";
const todo = {
  id: "1",
  parent_id: null,
  text: "Stored todo",
  completed: false,
};
function location(path) {
  window.history.replaceState(null, "", path);
  window.dispatchEvent(new Event("popstate"));
}
beforeEach(() => {
  vi.spyOn(document, "hidden", "get").mockReturnValue(false);
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});
test("a bookmarked agent resolves off-page by exact identity and restores the filter controls", async () => {
  const identity = JSON.stringify([
    agent.project,
    agent.hostname,
    agent.tool,
    agent.session_id,
    null,
  ]);
  location(
    "/?" +
      new URLSearchParams({
        project: agent.project,
        agent_details: identity,
        before: "cursor",
      }),
  );
  const fetch = vi.fn(async (path) =>
    response(path.includes("session_id=") ? page([agent]) : page([])),
  );
  vi.stubGlobal("fetch", fetch);
  render(Agents, { onexpire: vi.fn() });
  await screen.findByRole("region", { name: "Linked main agent" });
  expect(screen.getByText("Latest agents")).toBeTruthy();
  expect(
    screen.getByRole("button", { name: `Project: ${agent.project}` }),
  ).toBeTruthy();
  expect(document.querySelector("details").open).toBe(true);
  await fireEvent.click(screen.getByRole("button", { name: "Latest agents" }));
  expect(new URLSearchParams(window.location.search).has("before")).toBe(false);
});
test("an event bookmark resolves beyond the current page and handles unavailable targets", async () => {
  location(
    "/events?" +
      new URLSearchParams({
        event: JSON.stringify([message.project, message.event_id]),
        from: "0",
        to: "86400000",
        view: "hierarchy",
      }),
  );
  let missing = false;
  const fetch = vi.fn(async (path) =>
    response(
      path.includes("event_id=") ? page(missing ? [] : [message]) : page([]),
    ),
  );
  vi.stubGlobal("fetch", fetch);
  render(EventFeed, { onexpire: vi.fn() });
  await screen.findByRole("region", { name: "Linked event" });
  expect(screen.getByRole("region", { name: "Event details" })).toBeTruthy();
  missing = true;
  updateQuery({ event: JSON.stringify([message.project, "gone"]) });
  await screen.findByText(/The linked event is unavailable/);
  await fireEvent.click(screen.getByText("Clear linked event"));
  expect(new URLSearchParams(window.location.search).has("event")).toBe(false);
  expect(screen.queryByRole("region", { name: "Linked event" })).toBeNull();
});
test.each(["edit", "child", "view"])(
  "restoring todo %s mode only reads persisted text",
  async (mode) => {
    location(`/todos?todo=1&mode=${mode}`);
    const fetch = vi.fn(async () => response({ revision: "1", items: [todo] }));
    vi.stubGlobal("fetch", fetch);
    Element.prototype.scrollIntoView ??= () => {};
    vi.spyOn(Element.prototype, "scrollIntoView").mockImplementation(() => {});
    render(Todos, { onexpire: vi.fn() });
    await screen.findByRole("checkbox", { name: todo.text });
    if (mode === "edit")
      expect(screen.getByLabelText("Edit todo text").value).toBe(todo.text);
    if (mode === "child")
      expect(screen.getByLabelText("New child of Stored todo").value).toBe("");
    expect(
      fetch.mock.calls.every(
        ([, opts]) => !opts.method || opts.method === "GET",
      ),
    ).toBe(true);
    expect(window.location.search).not.toContain("Stored");
  },
);
test("credential bookmarks load metadata and never issue a token or restore a secret", async () => {
  location(
    "/status?panel=credentials&credential_project=silkboard&credential_after=dev",
  );
  const fetch = vi.fn(async () =>
    response({ items: [{ label: "dev", revoked: false }], next_cursor: null }),
  );
  vi.stubGlobal("fetch", fetch);
  render(PublisherTokens, { onexpire: vi.fn() });
  await screen.findByText("dev");
  expect(screen.getByLabelText("Credential project").value).toBe("silkboard");
  expect(fetch.mock.calls[0][0]).toContain("after=dev");
  expect(fetch.mock.calls.every(([, opts]) => opts.method !== "POST")).toBe(
    true,
  );
  expect(screen.queryByLabelText("New publisher token")).toBeNull();
  updateQuery({ credential_project: null, credential_after: null });
  await screen.findByLabelText("Installation label");
  expect(screen.getByLabelText("Credential project").value).toBe("");
});
test("workflow bookmarks restore selected step, Steps presentation and revision history", async () => {
  location(
    `/plans/silkboard/${task}?step=ui&presentation=steps&history=revisions`,
  );
  const value = detail();
  vi.stubGlobal(
    "fetch",
    vi.fn(async (path) =>
      response(path.includes("/revisions") ? page([value.summary]) : value),
    ),
  );
  render(TaskPlans, { onexpire: vi.fn() });
  await screen.findByRole("list", { name: "Steps in prerequisite order" });
  expect(
    screen.getByRole("heading", { name: "Build graph page" }),
  ).toBeTruthy();
  await screen.findByRole("list", { name: "Received workflow revisions" });
  expect(
    screen
      .getByRole("button", { name: "Definition revisions", exact: true })
      .getAttribute("aria-pressed"),
  ).toBe("true");
  updateQuery({ step: "missing", presentation: null });
  await screen.findByText("The selected step is not in this snapshot.");
  await fireEvent.click(screen.getByText("Clear step selection"));
  expect(new URLSearchParams(window.location.search).has("step")).toBe(false);
});
test("unknown pages and unsupported setup state offer local recovery", () => {
  const view = render(App, { unknownPage: true });
  expect(screen.getByText("Page not found")).toBeTruthy();
  view.unmount();
  location("/setup?panel=credentials");
  render(App, { setupPage: true });
  expect(screen.getByRole("alert").textContent).toContain("link");
});

test("invalid agent links reset to a usable view without fetching private data first", async () => {
  location("/?project=BAD");
  const fetch = vi.fn(async () => response(page([agent])));
  vi.stubGlobal("fetch", fetch);
  render(Agents, { onexpire: vi.fn() });
  expect(screen.getByRole("alert").textContent).toContain("link");
  expect(fetch).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole("link", { name: "Reset view" }));
  await screen.findByRole("article");
  expect(window.location.search).toBe("");
  const disclosure = document.querySelector("details");
  disclosure.open = true;
  await fireEvent(disclosure, new Event("toggle"));
  expect(
    new URLSearchParams(window.location.search).get("agent_details"),
  ).toContain(agent.session_id);
  disclosure.open = false;
  await fireEvent(disclosure, new Event("toggle"));
  expect(new URLSearchParams(window.location.search).has("agent_details")).toBe(
    false,
  );
});
test.each([404, 401, 503])(
  "linked agent lookup failure %s stays recoverable and expires when unauthorized",
  async (status) => {
    const key = JSON.stringify([
      agent.project,
      agent.hostname,
      agent.tool,
      agent.session_id,
      null,
    ]);
    location("/?" + new URLSearchParams({ agent_details: key }));
    let failure = status;
    const onexpire = vi.fn();
    const fetch = vi.fn(async (path) =>
      path.includes("session_id=")
        ? response(failure === 200 ? page([agent]) : {}, failure)
        : response(page([])),
    );
    vi.stubGlobal("fetch", fetch);
    render(Agents, { onexpire });
    if (status === 401) {
      await vi.waitFor(() => expect(onexpire).toHaveBeenCalled());
      return;
    }
    await screen.findByText(/The linked agent is unavailable/);
    failure = 200;
    await fireEvent.click(screen.getByText("Retry linked agent"));
    await screen.findByRole("region", { name: "Linked main agent" });
    await vi.waitFor(() =>
      expect(document.querySelector("details")).not.toBeNull(),
    );
    const disclosure = document.querySelector("details");
    disclosure.open = false;
    await fireEvent(disclosure, new Event("toggle"));
    expect(
      new URLSearchParams(window.location.search).has("agent_details"),
    ).toBe(false);
    updateQuery({ agent_details: key });
    await screen.findByRole("region", { name: "Linked main agent" });
    failure = 503;
    updateQuery({
      agent_details: JSON.stringify([
        agent.project,
        agent.hostname,
        agent.tool,
        "missing",
        null,
      ]),
    });
    await screen.findByText(/The linked agent is unavailable/);
    await fireEvent.click(screen.getByText("Clear linked agent"));
    expect(
      new URLSearchParams(window.location.search).has("agent_details"),
    ).toBe(false);
  },
);
test("event-based cards retain a native fallback name and explicit disclosure transitions", async () => {
  const { default: AgentCard } = await import("./AgentCard.svelte");
  const changed = vi.fn();
  render(AgentCard, {
    item: {
      ...agent,
      session_name: null,
      latest: { kind: "event", event_id: "one", message: "Event summary" },
    },
    onexpand: changed,
  });
  expect(
    screen.getByRole("link", { name: `${agent.tool} on ${agent.hostname}` }),
  ).toBeTruthy();
  const disclosure = document.querySelector("details");
  await fireEvent(disclosure, new Event("toggle"));
  expect(changed).not.toHaveBeenCalled();
  disclosure.open = true;
  await fireEvent(disclosure, new Event("toggle"));
  expect(changed).toHaveBeenCalledWith(true);
});

test("cards update names, identity context and latest activity without stale links", async () => {
  const { default: AgentCard } = await import("./AgentCard.svelte");
  const view = render(AgentCard, { item: agent, onexpand: vi.fn() });
  await view.rerender({
    item: {
      ...agent,
      hostname: "moved-host",
      tool: "claude",
      latest: { ...agent.latest, title: "Updated workflow", state: "blocked" },
    },
  });
  expect(screen.getByText("Reported blocked")).toBeTruthy();
  expect(screen.getByText("Workflow: Updated workflow")).toBeTruthy();
  expect(screen.getByText("claude · moved-host")).toBeTruthy();
  await view.rerender({
    item: {
      ...agent,
      latest: { kind: "event", event_id: "one", message: "First report" },
    },
  });
  await view.rerender({
    item: {
      ...agent,
      latest: { kind: "event", event_id: "two", message: "Second report" },
    },
  });
  expect(screen.getByText("Event: Second report")).toBeTruthy();
  expect(screen.queryByText("Workflow: Updated workflow")).toBeNull();
});
