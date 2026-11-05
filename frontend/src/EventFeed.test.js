import {
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import OwnerFeed from "./OwnerFeed.svelte";
import { csrf, message, page, response, routes } from "../test/support";
const report = {
  ...message,
  project: "alpha",
  hostname: "dev-alpha",
  tool: "codex",
  session_id: "private-native",
  subagent_id: null,
  parent_session_id: null,
  session_name: "Work on alpha",
  message: "A".repeat(180),
};
beforeEach(() => vi.spyOn(document, "hidden", "get").mockReturnValue(false));
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  vi.useRealTimers();
});
test("starting view is one compact cross-project event table with inline details", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(
      routes({
        "/api/owner/session": response({
          authenticated: true,
          csrf_token: csrf,
        }),
        "/api/owner/events": response(
          page([
            report,
            {
              ...report,
              project: "bravo",
              tool: "claude",
              event_id: report.event_id,
              message: "Second project report",
            },
          ]),
        ),
      }),
    ),
  );
  render(OwnerFeed, { eventsPage: true });
  const table = await screen.findByRole("table", {
    name: "Events from all projects",
  });
  expect(
    screen.getByRole("button", { name: "Project: All projects" }),
  ).toBeTruthy();
  expect(
    within(table)
      .getAllByRole("columnheader")
      .map((node) => node.textContent),
  ).toEqual(["Time", "Project", "Agent", "Message"]);
  expect(within(table).getByText("alpha")).toBeTruthy();
  expect(within(table).getByText("bravo")).toBeTruthy();
  expect(screen.queryByText(report.message)).toBeNull();
  expect(screen.queryByText("private-native")).toBeNull();
  const buttons = within(table).getAllByRole("button");
  await fireEvent.click(buttons[0]);
  expect(screen.getByRole("region", { name: "Event details" })).toBeTruthy();
  expect(screen.getByText(report.message)).toBeTruthy();
  expect(screen.getByText("private-native")).toBeTruthy();
  expect(buttons[0].getAttribute("aria-expanded")).toBe("true");
  await fireEvent.click(buttons[1]);
  expect(screen.getAllByRole("region", { name: "Event details" })).toHaveLength(
    1,
  );
  expect(buttons[0].getAttribute("aria-expanded")).toBe("false");
  await fireEvent.click(buttons[1]);
  expect(screen.queryByRole("region", { name: "Event details" })).toBeNull();
});

test("events retry, preserve stale rows on refresh failure and page without project navigation", async () => {
  vi.useFakeTimers();
  let fail = true;
  const cursor = '{"project":"alpha","event_id":"x&y","received_at":1}';
  const fetch = vi.fn(
    routes({
      "/api/owner/events": () =>
        fail ? response({}, 503) : response(page([report], cursor)),
      [`/api/owner/events?before=${encodeURIComponent(cursor)}`]: response(
        page([]),
      ),
    }),
  );
  vi.stubGlobal("fetch", fetch);
  const { default: EventFeed } = await import("./EventFeed.svelte");
  render(EventFeed, { onexpire: vi.fn() });
  await vi.advanceTimersByTimeAsync(0);
  expect(screen.getByRole("alert")).toBeTruthy();
  fail = false;
  await fireEvent.click(screen.getByRole("button", { name: "Retry events" }));
  await vi.advanceTimersByTimeAsync(0);
  await fireEvent.click(screen.getByRole("button", { name: /Event details:/ }));
  expect(screen.getByRole("region", { name: "Event details" })).toBeTruthy();
  fail = true;
  await vi.advanceTimersByTimeAsync(10000);
  expect(screen.getByRole("table")).toBeTruthy();
  expect(screen.getByRole("alert").textContent).toContain("out of date");
  fail = false;
  await fireEvent.click(screen.getByRole("button", { name: "Earlier events" }));
  await vi.advanceTimersByTimeAsync(0);
  expect(screen.getByText(/No events on this page/)).toBeTruthy();
  expect(screen.queryByRole("region", { name: "Event details" })).toBeNull();
  await fireEvent.click(screen.getByRole("button", { name: "Latest events" }));
  await vi.advanceTimersByTimeAsync(0);
  expect(screen.getByRole("table")).toBeTruthy();
});
test("refresh preserves the selected event, clears vanished details, and surfaces child identity as safe text", async () => {
  vi.useFakeTimers();
  const first = {
    ...report,
    subagent_id: "child",
    parent_session_id: "parent",
    session_name: null,
    message: '<img src=x onerror="window.injected=true">',
  };
  let rows = [first];
  vi.stubGlobal(
    "fetch",
    vi.fn(routes({ "/api/owner/events": () => response(page(rows)) })),
  );
  const { default: EventFeed } = await import("./EventFeed.svelte");
  render(EventFeed, { onexpire: vi.fn() });
  await vi.advanceTimersByTimeAsync(0);
  await fireEvent.click(screen.getByText("alpha"));
  expect(screen.getByRole("region", { name: "Event details" })).toBeTruthy();
  expect(screen.getByText("child")).toBeTruthy();
  expect(screen.getByText("parent")).toBeTruthy();
  expect(document.querySelector(".events-table img")).toBeNull();
  rows = [{ ...report, project: "bravo" }, first];
  await vi.advanceTimersByTimeAsync(10000);
  expect(
    screen.getByRole("region", { name: "Event details" }).textContent,
  ).toContain(first.message);
  rows = [];
  await vi.advanceTimersByTimeAsync(10000);
  expect(screen.queryByRole("region", { name: "Event details" })).toBeNull();
});
test.each(["response", "body"])(
  "teardown cancels late event %s and never restores private rows",
  async (stage) => {
    let finish;
    const pending = new Promise((done) => (finish = done));
    vi.stubGlobal(
      "fetch",
      vi.fn(() =>
        stage === "response"
          ? pending
          : { ok: true, status: 200, json: () => pending },
      ),
    );
    const { default: EventFeed } = await import("./EventFeed.svelte");
    const expired = vi.fn();
    const view = render(EventFeed, { onexpire: expired });
    await screen.findByText("Loading events…");
    await new Promise((done) => setTimeout(done, 0));
    view.unmount();
    finish(stage === "response" ? response(page([report])) : page([report]));
    await new Promise((done) => setTimeout(done, 0));
    expect(screen.queryByRole("table")).toBeNull();
    expect(expired).not.toHaveBeenCalled();
  },
);

test.each([
  [0, "00:04"],
  [15, "15:04"],
])(
  "renders Markdown and exact local timestamps at hour %s",
  async (hour, clock) => {
    const received_at = new Date(2026, 0, 2, hour, 4, 5).getTime();
    vi.stubGlobal(
      "fetch",
      vi.fn(
        routes({
          "/api/owner/events": response(
            page([
              { ...report, received_at, message: "**Review** the `cache`." },
            ]),
          ),
        }),
      ),
    );
    const { default: EventFeed } = await import("./EventFeed.svelte");
    render(EventFeed, { onexpire: vi.fn() });
    const table = await screen.findByRole("table");
    expect(
      [...table.querySelectorAll("time span")].map((node) => node.textContent),
    ).toEqual([clock, "01-02"]);
    await fireEvent.click(
      screen.getByRole("button", { name: "Event details: Review the cache." }),
    );
    const details = screen.getByRole("region", { name: "Event details" });
    expect(details.querySelector("strong").textContent).toBe("Review");
    expect(details.querySelector("code").textContent).toBe("cache");
    expect(details.querySelector("time").textContent).toBe(
      `2026-01-02 ${clock}:05`,
    );
    expect(within(details).queryByText("Event ID")).toBeNull();
    expect(within(details).queryByText("Sequence")).toBeNull();
  },
);

test("explicit workflow links stay separate from message expansion and validate targets", async () => {
  const task = "11111111-1111-4111-8111-111111111111";
  vi.stubGlobal(
    "fetch",
    vi.fn(async () =>
      response(
        page([
          { ...report, event_id: "linked", workflow_task_id: task },
          { ...report, event_id: "legacy", message: "Older report" },
        ]),
      ),
    ),
  );
  const { default: EventFeed } = await import("./EventFeed.svelte");
  render(EventFeed, { onexpire: vi.fn() });
  const link = await screen.findByRole("link", { name: "Open workflow" });
  expect(link.getAttribute("href")).toBe(`/plans/alpha/${task}`);
  expect(screen.getAllByRole("link", { name: "Open workflow" })).toHaveLength(
    1,
  );
  await fireEvent.click(link);
  expect(screen.queryByRole("region", { name: "Event details" })).toBeNull();
  await fireEvent.click(
    screen.getAllByRole("button", { name: /Event details:/ })[0],
  );
  expect(screen.getByRole("region", { name: "Event details" })).toBeTruthy();
});

test("hierarchy groups visible sessions, keeps unmatched children and preserves details when switching", async () => {
  const { default: EventFeed } = await import("./EventFeed.svelte");
  const parent = {
    ...report,
    event_id: "parent",
    message: "Parent report",
    session_id: "root",
    session_name: "Implementation",
  };
  const child = {
    ...parent,
    event_id: "child",
    subagent_id: "review",
    parent_session_id: "root",
    session_name: "Review",
    message: "Child report",
    description: "Detailed child context.\n\nSecond paragraph.",
  };
  const unmatched = {
    ...child,
    event_id: "unmatched",
    session_id: "unmatched",
    parent_session_id: "absent",
    session_name: "Unmatched",
    message: "Unmatched report",
  };
  const fetch = vi.fn(
    routes({ "/api/owner/events": response(page([child, parent, unmatched])) }),
  );
  vi.stubGlobal("fetch", fetch);
  render(EventFeed, { onexpire: vi.fn() });
  await screen.findByRole("button", { name: "Event details: Parent report" });
  expect(
    screen
      .getByRole("button", { name: "Flat list" })
      .getAttribute("aria-pressed"),
  ).toBe("true");
  await fireEvent.click(
    screen.getByRole("button", { name: "Session hierarchy" }),
  );
  const group = screen.getByRole("button", {
    name: "Session group: Implementation",
  });
  const review = screen.getByRole("button", {
    name: "Session group: Review, subagent of Implementation",
  });
  expect(group.getAttribute("aria-expanded")).toBe("true");
  expect(screen.getByText("Parent outside current results")).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("button", { name: "Event details: Child report" }),
  );
  expect(screen.getByText("Detailed child context.")).toBeTruthy();
  await fireEvent.click(review);
  expect(
    screen.queryByRole("button", { name: "Event details: Child report" }),
  ).toBeNull();
  expect(
    screen.getByRole("button", { name: "Event details: Parent report" }),
  ).toBeTruthy();
  await fireEvent.click(review);
  await fireEvent.click(group);
  expect(
    screen.queryByRole("button", { name: "Event details: Parent report" }),
  ).toBeNull();
  expect(
    screen.queryByRole("button", { name: /Session group: Review/ }),
  ).toBeNull();
  expect(
    screen.getByRole("button", { name: "Event details: Unmatched report" }),
  ).toBeTruthy();
  await fireEvent.click(group);
  await fireEvent.click(screen.getByRole("button", { name: "Flat list" }));
  expect(screen.queryByRole("button", { name: /Session group:/ })).toBeNull();
  expect(screen.getByText("Detailed child context.")).toBeTruthy();
  expect(fetch).toHaveBeenCalledTimes(1);
});

test("hierarchy switch retains filters and current cursor; page navigation resets groups", async () => {
  const { default: EventFeed } = await import("./EventFeed.svelte");
  const fetch = vi.fn(
    routes({
      "/api/owner/events": response(page([report])),
      "/api/owner/filter-options?view=events&kind=project&q=": response(
        page(["alpha"]),
      ),
      "/api/owner/filter-options?view=events&kind=agent&q=": response(
        page(["codex dev-alpha"]),
      ),
      "/api/owner/events?project=alpha": response(page([report])),
      "/api/owner/events?project=alpha&agent=codex%20dev-alpha": response(
        page([report], "cursor"),
      ),
      "/api/owner/events?before=cursor&project=alpha&agent=codex%20dev-alpha":
        response(
          page([
            {
              ...report,
              session_name: null,
              subagent_id: "unknown",
              parent_session_id: null,
            },
          ]),
        ),
    }),
  );
  vi.stubGlobal("fetch", fetch);
  render(EventFeed, { onexpire: vi.fn() });
  await screen.findByRole("button", { name: /Event details:/ });
  await fireEvent.click(
    screen.getByRole("button", { name: "Project: All projects" }),
  );
  await fireEvent.click(
    await screen.findByRole("button", { name: "alpha", exact: true }),
  );
  await fireEvent.click(
    screen.getByRole("button", { name: "Agent: All agents" }),
  );
  await fireEvent.click(
    await screen.findByRole("button", {
      name: "codex · dev-alpha",
      exact: true,
    }),
  );
  await screen.findByRole("button", { name: "Earlier events" });
  await fireEvent.click(
    screen.getByRole("button", { name: "Session hierarchy" }),
  );
  const group = await screen.findByRole("button", {
    name: "Session group: Work on alpha",
  });
  await fireEvent.click(group);
  await fireEvent.click(screen.getByRole("button", { name: "Earlier events" }));
  await screen.findByText("Parent not reported");
  const calls = fetch.mock.calls.length;
  await fireEvent.click(screen.getByRole("button", { name: "Flat list" }));
  expect(screen.getByRole("button", { name: "Latest events" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Project: alpha" })).toBeTruthy();
  expect(fetch.mock.calls.length).toBe(calls);
});

test("polling preserves collapsed sessions while allowing newly received parent links", async () => {
  vi.useFakeTimers();
  const { default: EventFeed } = await import("./EventFeed.svelte");
  const child = {
    ...report,
    subagent_id: "child",
    parent_session_id: "root",
    session_name: "Review",
  };
  const independent = {
    ...report,
    event_id: "independent",
    session_id: "other-root",
    session_name: "Other session",
  };
  let reports = [child, independent];
  vi.stubGlobal(
    "fetch",
    vi.fn(routes({ "/api/owner/events": () => response(page(reports)) })),
  );
  render(EventFeed, { onexpire: vi.fn() });
  await vi.advanceTimersByTimeAsync(0);
  await fireEvent.click(
    screen.getByRole("button", { name: "Session hierarchy" }),
  );
  const group = screen.getByRole("button", { name: /Session group: Review/ });
  await fireEvent.click(group);
  group.focus();
  reports = [
    child,
    {
      ...report,
      event_id: "parent",
      session_id: "root",
      session_name: "Parent",
      message: "Parent report",
    },
    independent,
  ];
  await vi.advanceTimersByTimeAsync(10000);
  expect(document.activeElement).toBe(group);
  expect(screen.queryByText("Parent outside current results")).toBeNull();
  expect(
    screen
      .getByRole("button", {
        name: "Session group: Review, subagent of Parent",
      })
      .getAttribute("aria-expanded"),
  ).toBe("false");
  expect(
    screen.getByRole("button", { name: "Event details: Parent report" }),
  ).toBeTruthy();
});

test("session groups show counts for multiple reports and child-only pages never lose events", async () => {
  const { default: EventFeed } = await import("./EventFeed.svelte");
  const first = {
    ...report,
    subagent_id: "review",
    parent_session_id: null,
    session_name: null,
    message: "First child update",
  };
  vi.stubGlobal(
    "fetch",
    vi.fn(
      routes({
        "/api/owner/events": response(
          page([
            first,
            {
              ...first,
              event_id: "second",
              sequence: "2",
              message: "Second child update",
            },
          ]),
        ),
      }),
    ),
  );
  render(EventFeed, { onexpire: vi.fn() });
  await screen.findByRole("button", {
    name: "Event details: First child update",
  });
  await fireEvent.click(
    screen.getByRole("button", { name: "Session hierarchy" }),
  );
  expect(
    screen.getByRole("button", {
      name: "Session group: Subagent review, Parent not reported",
    }).textContent,
  ).toMatch(/2\s+events on this page/);
  expect(
    screen.getAllByRole("button", { name: /Event details:/ }),
  ).toHaveLength(2);
});

test.each([false, true])(
  "event summary stays compact while optional description opens as safe Markdown (hierarchy=%s)",
  async (hierarchical) => {
    const { default: EventFeed } = await import("./EventFeed.svelte");
    vi.stubGlobal(
      "fetch",
      vi.fn(
        routes({
          "/api/owner/events": response(
            page([
              {
                ...report,
                message: "Tests complete",
                description:
                  "First paragraph.\n\n**Second paragraph.**\n\n<script>alert(1)</script>",
              },
            ]),
          ),
        }),
      ),
    );
    render(EventFeed, { onexpire: vi.fn() });
    if (hierarchical) {
      await screen.findByRole("button", {
        name: "Event details: Tests complete",
      });
      await fireEvent.click(
        screen.getByRole("button", { name: "Session hierarchy" }),
      );
    }
    const button = await screen.findByRole("button", {
      name: "Event details: Tests complete",
    });
    expect(screen.queryByText("First paragraph.")).toBeNull();
    await fireEvent.click(button);
    const detail = screen.getByRole("region", { name: "Event details" });
    expect(within(detail).getByText("First paragraph.")).toBeTruthy();
    expect(within(detail).getByText("Second paragraph.").tagName).toBe(
      "STRONG",
    );
    expect(detail.querySelector("script")).toBeNull();
  },
);

test("exact session scope survives paging and view changes, clears and restores with navigation", async () => {
  const { sessionEvents } = await import("./event-session");
  const { default: EventFeed } = await import("./EventFeed.svelte");
  const path = sessionEvents({ ...report, subagent_id: "child" });
  window.history.replaceState(null, "", path);
  const fetch = vi.fn(async (path) =>
    response(
      path.includes("filter-options")
        ? page(["claude other-host"])
        : page([report], path.includes("before=") ? null : "older"),
    ),
  );
  vi.stubGlobal("fetch", fetch);
  render(EventFeed, { onexpire: vi.fn() });
  await screen.findByRole("table");
  expect(
    screen.getByRole("region", { name: "Selected agent session" }),
  ).toBeTruthy();
  expect(
    screen.queryByRole("button", { name: "Project: All projects" }),
  ).toBeNull();
  await fireEvent.click(
    screen.getByRole("button", { name: "Session hierarchy" }),
  );
  await fireEvent.click(screen.getByRole("button", { name: "Earlier events" }));
  await screen.findByRole("button", { name: "Latest events" });
  for (const [url] of fetch.mock.calls) {
    const query = new URL(url, location.origin).searchParams;
    expect(query.get("session_id")).toBe("private-native");
    expect(query.get("subagent_id")).toBe("child");
  }
  await fireEvent.click(
    screen.getByRole("button", { name: "Clear session filter" }),
  );
  await screen.findByRole("button", { name: "Project: All projects" });
  await fireEvent.click(
    screen.getByRole("button", { name: "Agent: All agents" }),
  );
  await fireEvent.click(
    await screen.findByRole("button", {
      name: "claude · other-host",
      exact: true,
    }),
  );
  await screen.findByRole("button", { name: "Agent: claude · other-host" });
  window.history.replaceState(null, "", path);
  window.dispatchEvent(new PopStateEvent("popstate"));
  await screen.findByRole("region", { name: "Selected agent session" });
  expect(fetch.mock.calls.at(-1)[0]).toContain("subagent_id=child");
  expect(
    new URL(fetch.mock.calls.at(-1)[0], location.origin).searchParams.has(
      "agent",
    ),
  ).toBe(false);
  window.history.replaceState(null, "", "/");
});
test("incomplete session links fail closed until cleared", async () => {
  const { default: EventFeed } = await import("./EventFeed.svelte");
  window.history.replaceState(null, "", "/events?session_id=private");
  const fetch = vi.fn(async () => response(page([])));
  vi.stubGlobal("fetch", fetch);
  render(EventFeed, { onexpire: vi.fn() });
  await screen.findByRole("alert");
  expect(fetch).not.toHaveBeenCalled();
  await fireEvent.click(
    screen.getByRole("button", { name: "Clear session filter" }),
  );
  await screen.findByText(/No events on this page/);
  expect(fetch).toHaveBeenCalledOnce();
  window.history.replaceState(null, "", "/");
});

test("clear filters can reset an ordinary feed without adding a session URL", async () => {
  const { default: EventFeed } = await import("./EventFeed.svelte");
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response(page([]))),
  );
  render(EventFeed, { onexpire: vi.fn() });
  await screen.findByText(/No events on this page/);
  await fireEvent.click(screen.getByRole("button", { name: "Clear filters" }));
  expect(location.search).toBe("");
});

test("oversized collapsed identities keep the last valid URL and offer recovery", async () => {
  const { default: EventFeed } = await import("./EventFeed.svelte");
  const long = '"\\'.repeat(128);
  const reports = Array.from({ length: 20 }, (_, i) => ({
    ...report,
    event_id: `group-${i}`,
    hostname: String(i) + long.slice(2),
    tool: long,
    session_id: long,
    session_name: `Group ${i}`,
  }));
  window.history.replaceState(null, "", "/events?view=hierarchy");
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response(page(reports))),
  );
  render(EventFeed, { onexpire: vi.fn() });
  await screen.findByRole("button", { name: "Session group: Group 0" });
  for (let i = 0; i < 10; i++)
    await fireEvent.click(
      screen.getByRole("button", {
        name: `Session group: Group ${i}`,
        exact: true,
      }),
    );
  expect(screen.getByRole("alert").textContent).toContain(
    "bookmark size limit",
  );
  expect(
    window.location.pathname.length + window.location.search.length,
  ).toBeLessThanOrEqual(60000);
  await fireEvent.click(
    screen.getByRole("button", { name: "Clear collapsed groups" }),
  );
  expect(new URLSearchParams(window.location.search).has("collapsed")).toBe(
    false,
  );
  expect(screen.queryByRole("alert")).toBeNull();
});
