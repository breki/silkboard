import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import TaskPlan from "./TaskPlan.svelte";
import { detail, task } from "../test/plans";
import { page, response } from "../test/support";
const root = `/api/owner/projects/silkboard/plans/${task}`;
let current, stored, history, code, fetch, onexpire, onsequence;
beforeEach(() => {
  vi.useFakeTimers();
  vi.spyOn(document, "hidden", "get").mockReturnValue(false);
  current = detail({ sequence: "3" });
  stored = detail({ sequence: "1" });
  history = page([stored.summary]);
  code = 200;
  onexpire = vi.fn();
  onsequence = vi.fn();
  fetch = vi.fn(async (path) => {
    if (code !== 200) return response({}, code);
    if (path === "/api/owner/plans") return response(page([current.summary]));
    if (path.includes("/history") || path.includes("/revisions"))
      return response(history);
    return response(path.includes("?") ? stored : current);
  });
  vi.stubGlobal("fetch", fetch);
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.useRealTimers();
});
const settle = () => vi.advanceTimersByTimeAsync(0);
const props = (sequence = null) => ({
  project: "silkboard",
  task,
  sequence,
  onexpire,
  onsequence,
  onopen: vi.fn(),
});
const button = (name) => screen.getByRole("button", { name, exact: true });
test("live reports update without stealing selection; a pinned report only announces newer arrivals", async () => {
  const view = render(TaskPlan, props());
  await settle();
  await fireEvent.click(button("Build graph page, Active"));
  current = detail({ sequence: "4", title: "Changed plan" });
  await vi.advanceTimersByTimeAsync(10000);
  expect(screen.getByRole("heading", { name: "Changed plan" })).toBeTruthy();
  expect(
    screen.getByRole("heading", { name: "Build graph page" }),
  ).toBeTruthy();
  await view.rerender(props("1"));
  await settle();
  expect(
    screen.getAllByText("Earlier snapshot · Pinned").length,
  ).toBeGreaterThan(0);
  expect(screen.getByText(/Newer reports are available/)).toBeTruthy();
  const exact = fetch.mock.calls.filter(([p]) =>
    p.includes("?sequence"),
  ).length;
  current = detail({ sequence: "5" });
  await vi.advanceTimersByTimeAsync(10000);
  expect(
    fetch.mock.calls.filter(([p]) => p.includes("?sequence")),
  ).toHaveLength(exact);
  expect(
    screen.getByRole("heading", { name: stored.snapshot.title }),
  ).toBeTruthy();
  await view.rerender(props());
  await settle();
  expect(screen.queryByText("Earlier snapshot · Pinned")).toBeNull();
});
test("revisions pin the latest received snapshot of that revision and comparisons show only received changes", async () => {
  current.snapshot.steps[1].title = "Changed graph";
  current.snapshot.steps[1].state = "pending";
  current.snapshot.steps[1].description = "Changed";
  current.snapshot.steps[1].depends_on = [];
  current.snapshot.steps.pop();
  current.snapshot.steps.push({
    id: "new",
    title: "New check",
    state: "pending",
    depends_on: [],
    description: null,
    report: null,
    blocked_reason: null,
  });
  current = detail(current.snapshot);
  current.summary.history_incomplete = true;
  current.snapshot.writer_generation = "2";
  current.summary.writer_generation = "2";
  render(TaskPlan, props());
  await settle();
  await fireEvent.click(button("Compare with previous received report"));
  await settle();
  expect(screen.getByText(/spans missing reports/)).toBeTruthy();
  expect(screen.getByText("Added: New check")).toBeTruthy();
  expect(screen.getByText("Removed: Verify browser behavior")).toBeTruthy();
  expect(screen.getByText("Changed prerequisites: Changed graph")).toBeTruthy();
  expect(screen.getByText(/Active → Pending/)).toBeTruthy();
  expect(screen.getByText(/continuity changed/)).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("link", { name: "View previous received snapshot" }),
  );
  await fireEvent.click(button("Received history and revisions"));
  await settle();
  await fireEvent.click(button("Definition revisions"));
  await settle();
  await fireEvent.click(button("Revision 1"));
  await settle();
  expect(onsequence).toHaveBeenCalledWith(
    `/plans/silkboard/${task}?sequence=1`,
  );
});
test("empty and identical comparisons do not invent work", async () => {
  current = detail({ sequence: "2" });
  render(TaskPlan, props());
  await settle();
  await fireEvent.click(button("Compare with previous received report"));
  await settle();
  expect(
    screen.getByText("No step changes between these received snapshots."),
  ).toBeTruthy();
  history = page([]);
  await fireEvent.click(button("Compare with previous received report"));
  await settle();
  expect(
    screen.getByText("No earlier report was received for comparison."),
  ).toBeTruthy();
});
test("failed and malformed refreshes retain displayed reports, with explicit retry", async () => {
  render(TaskPlan, props());
  await settle();
  code = 503;
  await vi.advanceTimersByTimeAsync(10000);
  expect(
    screen.getByRole("heading", { name: current.snapshot.title }),
  ).toBeTruthy();
  expect(screen.getByRole("alert").textContent).toContain("out of date");
  code = 200;
  await fireEvent.click(button("Retry workflow"));
  await settle();
  expect(screen.queryByRole("alert")).toBeNull();
  current = detail({ sequence: "2" });
  await vi.advanceTimersByTimeAsync(10000);
  expect(screen.getByRole("alert")).toBeTruthy();
});
test("expired and absent current plans distinguish disabled APIs and unknown tasks", async () => {
  code = 401;
  const view = render(TaskPlan, props());
  await settle();
  expect(onexpire).toHaveBeenCalled();
  code = 404;
  await vi.advanceTimersByTimeAsync(10000);
  expect(screen.getByRole("alert").textContent).toContain("not available");
  fetch.mockImplementation(async (path) =>
    response(
      path === "/api/owner/plans" ? page([]) : {},
      path === "/api/owner/plans" ? 200 : 404,
    ),
  );
  await fireEvent.click(button("Retry workflow"));
  await settle();
  expect(screen.getByRole("alert").textContent).toContain(
    "workflow was not found",
  );
  fetch.mockImplementation(async (path) =>
    response({}, path === "/api/owner/plans" ? 401 : 404),
  );
  await fireEvent.click(button("Retry workflow"));
  await settle();
  expect(onexpire.mock.calls.length).toBeGreaterThan(1);
  view.unmount();
});
test("a missing selected snapshot never substitutes the current one", async () => {
  fetch.mockImplementation(async (path) =>
    response(path === root ? current : {}, path === root ? 200 : 404),
  );
  render(TaskPlan, props("1"));
  await settle();
  expect(screen.getByRole("alert").textContent).toContain(
    "snapshot was not found",
  );
  expect(
    screen.queryByRole("region", { name: "Task prerequisite graph" }),
  ).toBeNull();
});
test("auxiliary errors, retry and hidden/unmounted replies remain scoped", async () => {
  render(TaskPlan, props());
  await settle();
  code = 503;
  await fireEvent.click(button("Compare with previous received report"));
  await settle();
  expect(screen.getByRole("alert")).toBeTruthy();
  code = 401;
  await fireEvent.click(button("Compare with previous received report"));
  await settle();
  expect(onexpire).toHaveBeenCalled();
  code = 404;
  await fireEvent.click(button("Received history and revisions"));
  await settle();
});
test("a pending selected-snapshot read aborts on hide and ignores late data", async () => {
  let resolve,
    hidden = false;
  vi.spyOn(document, "hidden", "get").mockImplementation(() => hidden);
  fetch.mockImplementation((path) =>
    path === root
      ? Promise.resolve(response(current))
      : new Promise((done) => (resolve = done)),
  );
  const view = render(TaskPlan, props("1"));
  await settle();
  hidden = true;
  document.dispatchEvent(new Event("visibilitychange"));
  await settle();
  resolve(response(stored));
  await settle();
  expect(
    screen.queryByRole("heading", { name: stored.snapshot.title }),
  ).toBeNull();
  view.unmount();
});

test("revision read failures are explicit, retryable, and do not replace the displayed plan", async () => {
  render(TaskPlan, props());
  await settle();
  await fireEvent.click(button("Received history and revisions"));
  await settle();
  await fireEvent.click(button("Definition revisions"));
  await settle();
  code = 503;
  await fireEvent.click(button("Revision 1"));
  await settle();
  expect(screen.getByRole("alert").textContent).toContain("Unable to update");
  expect(
    screen.getByRole("heading", { name: current.snapshot.title }),
  ).toBeTruthy();
  code = 200;
  await fireEvent.click(button("Revision 1"));
  await settle();
  expect(onsequence).toHaveBeenCalledOnce();
});
test("simple plans with no optional prose, issue or readable reporter name remain legible", async () => {
  current = detail({
    description: null,
    github_issue: null,
    reporter: {
      hostname: "dev",
      tool: "codex",
      session_id: "native",
      subagent_id: null,
      parent_session_id: null,
      session_name: null,
    },
  });
  render(TaskPlan, props());
  await settle();
  expect(
    screen.getByRole("heading", { name: current.snapshot.title }),
  ).toBeTruthy();
  expect(screen.queryByRole("link", { name: /GitHub issue/ })).toBeNull();
});
test("history refresh and selection reads cannot outlive a hidden or unmounted view", async () => {
  let hidden = false;
  vi.spyOn(document, "hidden", "get").mockImplementation(() => hidden);
  const view = render(TaskPlan, props());
  await settle();
  let resolve;
  fetch.mockImplementation(() => new Promise((done) => (resolve = done)));
  await fireEvent.click(button("Compare with previous received report"));
  await settle();
  hidden = true;
  document.dispatchEvent(new Event("visibilitychange"));
  await settle();
  resolve(response(page([stored.summary])));
  await settle();
  expect(screen.queryByText("Changes between received snapshots")).toBeNull();
  view.unmount();
});
test("unknown task probing failure does not claim missing support", async () => {
  fetch.mockImplementation(async (path) =>
    response({}, path === "/api/owner/plans" ? 503 : 404),
  );
  render(TaskPlan, props());
  await settle();
  expect(screen.getByRole("alert").textContent).toContain("Unable to update");
});

test("a comparison received for an older displayed report cannot overwrite a newer one", async () => {
  render(TaskPlan, props());
  await settle();
  await vi.advanceTimersByTimeAsync(1000);
  let resolve;
  fetch.mockImplementation((path) =>
    path.includes("?sequence")
      ? new Promise((done) => (resolve = done))
      : Promise.resolve(
          response(path.includes("/history") ? history : current),
        ),
  );
  await fireEvent.click(button("Compare with previous received report"));
  await settle();
  current = detail({ sequence: "4" });
  await vi.advanceTimersByTimeAsync(9000);
  resolve(response(stored));
  await settle();
  expect(screen.queryByText("Changes between received snapshots")).toBeNull();
});
test("a revision selected before unmount cannot navigate after its response arrives", async () => {
  const view = render(TaskPlan, props());
  await settle();
  await fireEvent.click(button("Received history and revisions"));
  await settle();
  await fireEvent.click(button("Definition revisions"));
  await settle();
  let resolve;
  fetch.mockImplementation(() => new Promise((done) => (resolve = done)));
  await fireEvent.click(button("Revision 1"));
  await settle();
  view.unmount();
  resolve(response(stored));
  await settle();
  expect(onsequence).not.toHaveBeenCalled();
});

test("an in-flight current-plan request cannot restore private data after unmount", async () => {
  let resolve;
  fetch.mockImplementation(() => new Promise((done) => (resolve = done)));
  const view = render(TaskPlan, props());
  await settle();
  view.unmount();
  resolve(response(current));
  await settle();
  expect(
    screen.queryByRole("heading", { name: current.snapshot.title }),
  ).toBeNull();
});

test("a requested historical snapshot retains and labels the previous display while loading", async () => {
  const view = render(TaskPlan, props());
  await settle();
  let resolve;
  fetch.mockImplementation((path) =>
    path.includes("?sequence")
      ? new Promise((done) => (resolve = done))
      : Promise.resolve(response(current)),
  );
  await view.rerender(props("1"));
  await settle();
  expect(
    screen.getByText(
      "Loading selected snapshot; the previous report is still displayed.",
    ),
  ).toBeTruthy();
  resolve(response(stored));
  await settle();
  expect(
    screen.queryByText(
      "Loading selected snapshot; the previous report is still displayed.",
    ),
  ).toBeNull();
});
