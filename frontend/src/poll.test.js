import { afterEach, expect, test, vi } from "vitest";
import { poll } from "./poll";
afterEach(() => {
  vi.restoreAllMocks();
  vi.useRealTimers();
});
test("polls while visible, pauses and aborts while hidden, stops on cleanup", async () => {
  vi.useFakeTimers();
  let hidden = false;
  vi.spyOn(document, "hidden", "get").mockImplementation(() => hidden);
  const load = vi.fn().mockResolvedValue();
  const stop = poll(load);
  await vi.advanceTimersByTimeAsync(10000);
  expect(load).toHaveBeenCalledTimes(2);
  hidden = true;
  document.dispatchEvent(new Event("visibilitychange"));
  await vi.advanceTimersByTimeAsync(20000);
  expect(load).toHaveBeenCalledTimes(2);
  hidden = false;
  document.dispatchEvent(new Event("visibilitychange"));
  await vi.advanceTimersByTimeAsync(0);
  expect(load).toHaveBeenCalledTimes(3);
  stop();
  await vi.advanceTimersByTimeAsync(20000);
  expect(load).toHaveBeenCalledTimes(3);
});
test("does not overlap slow requests and aborts on hiding or teardown", async () => {
  vi.useFakeTimers();
  let hidden = false;
  let resolve;
  vi.spyOn(document, "hidden", "get").mockImplementation(() => hidden);
  const load = vi.fn(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const stop = poll(load);
  await vi.advanceTimersByTimeAsync(20000);
  expect(load).toHaveBeenCalledTimes(1);
  hidden = true;
  document.dispatchEvent(new Event("visibilitychange"));
  expect(load.mock.calls[0][0].aborted).toBe(true);
  resolve();
  await vi.advanceTimersByTimeAsync(0);
  hidden = false;
  document.dispatchEvent(new Event("visibilitychange"));
  expect(load).toHaveBeenCalledTimes(2);
  stop();
  expect(load.mock.calls[1][0].aborted).toBe(true);
  resolve();
});
test("a failed load does not stop later refreshes", async () => {
  vi.useFakeTimers();
  vi.spyOn(document, "hidden", "get").mockReturnValue(false);
  const load = vi.fn().mockRejectedValue(new Error("unavailable"));
  const stop = poll(load);
  await vi.advanceTimersByTimeAsync(10000);
  expect(load).toHaveBeenCalledTimes(2);
  stop();
});
