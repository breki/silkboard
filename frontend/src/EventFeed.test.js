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
  render(OwnerFeed);
  const table = await screen.findByRole("table", {
    name: "Events from all projects",
  });
  expect(screen.getByLabelText("Project")).toBeTruthy();
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

test("event summary stays compact while optional description opens as safe Markdown", async () => {
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
  const button = await screen.findByRole("button", {
    name: "Event details: Tests complete",
  });
  expect(screen.queryByText("First paragraph.")).toBeNull();
  await fireEvent.click(button);
  const detail = screen.getByRole("region", { name: "Event details" });
  expect(within(detail).getByText("First paragraph.")).toBeTruthy();
  expect(within(detail).getByText("Second paragraph.").tagName).toBe("STRONG");
  expect(detail.querySelector("script")).toBeNull();
});
