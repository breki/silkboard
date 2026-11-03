import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import PlanList from "./PlanList.svelte";
import { detail } from "../test/plans";
import { page, response } from "../test/support";
beforeEach(() => {
  vi.useFakeTimers();
  vi.spyOn(document, "hidden", "get").mockReturnValue(false);
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.useRealTimers();
});
const settle = () => vi.advanceTimersByTimeAsync(0);
test("pages, retries and retains the last received rows after a poll failure", async () => {
  let fail = true;
  const value = detail();
  const cursor = "opaque & cursor";
  const fetch = vi.fn(async (path) =>
    fail
      ? response({}, 503)
      : response(path.includes("?") ? page([]) : page([value.summary], cursor)),
  );
  vi.stubGlobal("fetch", fetch);
  const view = render(PlanList, {
    active: true,
    onexpire: vi.fn(),
    onopen: vi.fn(),
  });
  await settle();
  expect(screen.getByRole("alert").textContent).toContain("Unable to update");
  fail = false;
  await fireEvent.click(screen.getByText("Retry workflows"));
  await settle();
  expect(screen.getByRole("table")).toBeTruthy();
  fail = true;
  await vi.advanceTimersByTimeAsync(10000);
  expect(screen.getByRole("table")).toBeTruthy();
  expect(screen.getByRole("alert").textContent).toContain("out of date");
  fail = false;
  await fireEvent.click(screen.getByText("Older workflows"));
  await settle();
  expect(fetch.mock.calls.at(-1)[0]).toContain("before=opaque%20%26%20cursor");
  expect(
    screen.getByText(
      "No workflows on this page. Return to the latest reports.",
    ),
  ).toBeTruthy();
  await fireEvent.click(screen.getByText("Latest workflows"));
  await settle();
  await view.rerender({ active: false, onexpire: vi.fn(), onopen: vi.fn() });
  const calls = fetch.mock.calls.length;
  await vi.advanceTimersByTimeAsync(20000);
  expect(fetch).toHaveBeenCalledTimes(calls);
});
test("expired, unsupported and aborted reads do not invent reports", async () => {
  const onexpire = vi.fn();
  let code = 401;
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response({}, code)),
  );
  render(PlanList, { active: true, onexpire, onopen: vi.fn() });
  await settle();
  expect(onexpire).toHaveBeenCalledOnce();
  code = 404;
  await vi.advanceTimersByTimeAsync(10000);
  expect(screen.getByRole("alert").textContent).toContain("not available");
  code = 503;
  await fireEvent.click(screen.getByText("Retry workflows"));
  await settle();
  expect(screen.getByRole("alert").textContent).toContain("Unable");
});
test("bounded states display active and blocked counts and forward native link clicks", async () => {
  const value = detail();
  value.snapshot.steps[2].state = "blocked";
  value.snapshot.steps[2].blocked_reason = "Waiting";
  const normalized = detail(value.snapshot);
  const onopen = vi.fn();
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response(page([normalized.summary]))),
  );
  render(PlanList, { active: true, onexpire: vi.fn(), onopen });
  await settle();
  expect(screen.getByText("1 blocked")).toBeTruthy();
  await fireEvent.click(screen.getByRole("link"));
  expect(onopen).toHaveBeenCalledOnce();
});
test("aborts an in-flight read on unmount and ignores its eventual result", async () => {
  let done, signal;
  vi.stubGlobal(
    "fetch",
    vi.fn((_, options) => {
      signal = options.signal;
      return new Promise((resolve) => (done = resolve));
    }),
  );
  const view = render(PlanList, {
    active: true,
    onexpire: vi.fn(),
    onopen: vi.fn(),
  });
  await settle();
  view.unmount();
  expect(signal.aborted).toBe(true);
  done(response(page([])));
  await settle();
});

test("applying filters resets the cursor and preserves filters across paging and polling", async () => {
  const fetch = vi.fn(async (path) =>
    response(
      path.includes("filter-options")
        ? page(["silkboard"])
        : page([detail().summary], "older"),
    ),
  );
  vi.stubGlobal("fetch", fetch);
  render(PlanList, { active: true, onexpire: vi.fn(), onopen: vi.fn() });
  await settle();
  await fireEvent.click(screen.getByText("Older workflows"));
  await settle();
  expect(fetch.mock.calls.at(-1)[0]).toContain("before=older");
  await fireEvent.click(
    screen.getByRole("button", { name: "Project: All projects" }),
  );
  await vi.advanceTimersByTimeAsync(200);
  await fireEvent.click(
    screen.getByRole("button", { name: "silkboard", exact: true }),
  );
  await fireEvent.click(
    screen.getByRole("button", { name: "Active", exact: true }),
  );
  await settle();
  expect(fetch.mock.calls.at(-1)[0]).toBe(
    "/api/owner/plans?project=silkboard&state=active",
  );
  await fireEvent.click(screen.getByText("Older workflows"));
  await settle();
  expect(fetch.mock.calls.at(-1)[0]).toBe(
    "/api/owner/plans?before=older&project=silkboard&state=active",
  );
  await vi.advanceTimersByTimeAsync(10000);
  expect(fetch.mock.calls.at(-1)[0]).toContain(
    "project=silkboard&state=active",
  );
  await fireEvent.click(screen.getByText("Clear filters"));
  await settle();
  expect(fetch.mock.calls.at(-1)[0]).toBe("/api/owner/plans");
});

test("empty workflows distinguish unfiltered history from matching filters", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response(page([]))),
  );
  render(PlanList, { active: true, onexpire: vi.fn(), onopen: vi.fn() });
  await settle();
  expect(
    screen.getByText("No workflow reports have arrived yet."),
  ).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("button", { name: "All statuses", exact: true }),
  );
  await settle();
  expect(
    screen.getByText("No workflow reports have arrived yet."),
  ).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("button", { name: "Active", exact: true }),
  );
  await settle();
  expect(
    screen.getByText("No workflows match these filters on this page."),
  ).toBeTruthy();
});
