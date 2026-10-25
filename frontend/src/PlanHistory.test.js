import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import PlanHistory from "./PlanHistory.svelte";
import { detail, task } from "../test/plans";
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
const button = (name) => screen.getByRole("button", { name, exact: true });
test("lazy history pages and revision selection remain explicit bounded reads", async () => {
  const value = detail();
  const onopen = vi.fn(),
    onrevision = vi.fn();
  const fetch = vi.fn(async (path) =>
    response(path.includes("?") ? page([]) : page([value.summary], "1")),
  );
  vi.stubGlobal("fetch", fetch);
  render(PlanHistory, {
    project: "silkboard",
    task,
    onexpire: vi.fn(),
    onopen,
    onrevision,
  });
  await settle();
  expect(fetch).not.toHaveBeenCalled();
  await fireEvent.click(button("Received history and revisions"));
  await settle();
  expect(
    screen.getByRole("list", { name: "Received plan reports" }),
  ).toBeTruthy();
  await fireEvent.click(screen.getByRole("link"));
  expect(onopen).toHaveBeenCalledOnce();
  await fireEvent.click(button("Older received history"));
  await settle();
  expect(screen.getByText("No received history on this page.")).toBeTruthy();
  await fireEvent.click(button("Latest received history"));
  await settle();
  await fireEvent.click(button("Definition revisions"));
  await settle();
  await fireEvent.click(button("Revision 1"));
  expect(onrevision).toHaveBeenCalledWith("1");
  await fireEvent.click(button("Refresh history"));
  await settle();
  await fireEvent.click(button("Received reports"));
  await settle();
  await fireEvent.click(button("Received history and revisions"));
  await settle();
  const calls = fetch.mock.calls.length;
  await vi.advanceTimersByTimeAsync(20000);
  expect(fetch).toHaveBeenCalledTimes(calls);
});
test("history failure retains data, retries, expires, and pauses while hidden", async () => {
  let code = 503,
    hidden = false;
  vi.spyOn(document, "hidden", "get").mockImplementation(() => hidden);
  const onexpire = vi.fn();
  const fetch = vi.fn(async () =>
    response(code === 200 ? page([detail().summary]) : {}, code),
  );
  vi.stubGlobal("fetch", fetch);
  render(PlanHistory, {
    project: "silkboard",
    task,
    onexpire,
    onopen: vi.fn(),
    onrevision: vi.fn(),
  });
  await fireEvent.click(button("Received history and revisions"));
  await settle();
  expect(screen.getByRole("alert")).toBeTruthy();
  code = 200;
  await fireEvent.click(button("Retry history"));
  await settle();
  expect(screen.getByRole("list")).toBeTruthy();
  code = 503;
  await fireEvent.click(button("Refresh history"));
  await settle();
  expect(screen.getByRole("list")).toBeTruthy();
  code = 401;
  await fireEvent.click(button("Retry history"));
  await settle();
  expect(onexpire).toHaveBeenCalledOnce();
  hidden = true;
  document.dispatchEvent(new Event("visibilitychange"));
  await settle();
  const calls = fetch.mock.calls.length;
  await fireEvent.click(button("Retry history"));
  await settle();
  expect(fetch).toHaveBeenCalledTimes(calls);
  code = 200;
  hidden = false;
  document.dispatchEvent(new Event("visibilitychange"));
  await settle();
  expect(fetch.mock.calls.length).toBeGreaterThan(calls);
});
test("hidden and unmounted histories ignore late replies", async () => {
  let done;
  vi.stubGlobal(
    "fetch",
    vi.fn(() => new Promise((resolve) => (done = resolve))),
  );
  const view = render(PlanHistory, {
    project: "silkboard",
    task,
    onexpire: vi.fn(),
    onopen: vi.fn(),
    onrevision: vi.fn(),
  });
  await fireEvent.click(button("Received history and revisions"));
  await settle();
  view.unmount();
  done(response(page([])));
  await settle();
});

test("a history response received after cancellation is never displayed", async () => {
  let resolve;
  const fetch = vi.fn(() => new Promise((done) => (resolve = done)));
  vi.stubGlobal("fetch", fetch);
  render(PlanHistory, {
    project: "silkboard",
    task,
    onexpire: vi.fn(),
    onopen: vi.fn(),
    onrevision: vi.fn(),
  });
  await fireEvent.click(button("Received history and revisions"));
  await settle();
  await fireEvent.click(button("Received history and revisions"));
  resolve(response(page([detail().summary])));
  await settle();
  expect(screen.queryByRole("list")).toBeNull();
});

test("unnamed reporting sessions retain tool and host in received history", async () => {
  const value = detail();
  value.summary.reporter.session_name = null;
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response(page([value.summary]))),
  );
  render(PlanHistory, {
    project: "silkboard",
    task,
    onexpire: vi.fn(),
    onopen: vi.fn(),
    onrevision: vi.fn(),
  });
  await fireEvent.click(button("Received history and revisions"));
  await settle();
  expect(screen.getByText("Reported by codex on dev")).toBeTruthy();
});

test("clicking an already selected history tab keeps its received results visible", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response(page([detail().summary]))),
  );
  render(PlanHistory, {
    project: "silkboard",
    task,
    onexpire: vi.fn(),
    onopen: vi.fn(),
    onrevision: vi.fn(),
  });
  await fireEvent.click(button("Received history and revisions"));
  await settle();
  await fireEvent.click(button("Received reports"));
  await settle();
  expect(
    screen.getByRole("list", { name: "Received plan reports" }),
  ).toBeTruthy();
  await fireEvent.click(button("Definition revisions"));
  await settle();
  await fireEvent.click(button("Definition revisions"));
  await settle();
  expect(
    screen.getByRole("list", { name: "Received plan revisions" }),
  ).toBeTruthy();
});
