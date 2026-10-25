import { cleanup, render, screen, fireEvent } from "@testing-library/svelte";
import { afterEach, expect, test, vi } from "vitest";
import TaskPlan from "./TaskPlan.svelte";
import { detail, task } from "../test/plans";
import { response } from "../test/support";
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.useRealTimers();
});
test("silence and read failure age the received report without changing active or blocked work", async () => {
  vi.useFakeTimers();
  vi.spyOn(document, "hidden", "get").mockReturnValue(false);
  const value = detail();
  value.snapshot.steps[2].state = "blocked";
  value.snapshot.steps[2].blocked_reason = "Waiting for owner review";
  const received = detail(value.snapshot);
  vi.setSystemTime(received.summary.received_at);
  let code = 200;
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response(code === 200 ? received : {}, code)),
  );
  render(TaskPlan, {
    project: "silkboard",
    task,
    sequence: null,
    onexpire: vi.fn(),
    onsequence: vi.fn(),
    onopen: vi.fn(),
  });
  await vi.advanceTimersByTimeAsync(0);
  expect(
    screen.getByText(
      "These are reported states, not verified results or a live connection status.",
    ),
  ).toBeTruthy();
  expect(screen.getByText(/just received/)).toBeTruthy();
  const receipt = screen
    .getByText(/just received/)
    .querySelector("time").dateTime;
  await vi.advanceTimersByTimeAsync(60000);
  expect(screen.getByText(/1 minute ago/)).toBeTruthy();
  code = 503;
  await vi.advanceTimersByTimeAsync(60000);
  expect(screen.getByRole("alert").textContent).toContain("out of date");
  expect(screen.getByText(/2 minutes ago/)).toBeTruthy();
  expect(
    screen.getByRole("button", { name: "Build graph page, Active" }),
  ).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("button", { name: "Verify browser behavior, Blocked" }),
  );
  expect(screen.getByText("Waiting for owner review")).toBeTruthy();
  expect(screen.getByText(/2 minutes ago/).querySelector("time").dateTime).toBe(
    receipt,
  );
});
