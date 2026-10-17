import { cleanup, render, screen, waitFor } from "@testing-library/svelte";
import { afterEach, expect, test, vi } from "vitest";
import App from "./App.svelte";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

test("shows loading, then backend identity and health", async () => {
  const status = {
    health: "ok",
    version: "0.1.0",
    release_tag: "v0.1.0",
    commit: "0123456789abcdef0123456789abcdef01234567",
  };
  const fetch = vi
    .fn()
    .mockResolvedValue({ ok: true, json: async () => status });
  vi.stubGlobal("fetch", fetch);
  render(App);
  expect(screen.getByRole("status").textContent).toContain("Loading");
  await screen.findByText(status.commit);
  expect(screen.getByText("Healthy")).toBeTruthy();
  expect(screen.getByText(status.version)).toBeTruthy();
  expect(screen.getByText(status.release_tag)).toBeTruthy();
  expect(fetch).toHaveBeenCalledWith(
    "/api/status",
    expect.objectContaining({ cache: "no-store" }),
  );
});

test("development identity is visibly unknown", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        health: "ok",
        version: "0.1.0",
        release_tag: "unknown",
        commit: "unknown",
      }),
    }),
  );
  render(App);
  await waitFor(() => expect(screen.getAllByText("unknown")).toHaveLength(2));
});

test.each(["network", "http", "invalid"])(
  "%s failure does not claim health",
  async (failure) => {
    vi.stubGlobal(
      "fetch",
      failure === "network"
        ? vi.fn().mockRejectedValue(new Error("offline"))
        : vi.fn().mockResolvedValue({
            ok: failure !== "http",
            json: async () => ({}),
          }),
    );
    render(App);
    expect((await screen.findByRole("alert")).textContent).toContain(
      "Unable to load",
    );
    expect(screen.queryByText("Healthy")).toBeNull();
  },
);

test("a stalled request times out and offers recovery", async () => {
  vi.useFakeTimers();
  try {
    vi.stubGlobal(
      "fetch",
      vi.fn(
        (_url, { signal }) =>
          new Promise((_resolve, reject) => {
            signal.addEventListener("abort", () =>
              reject(new Error("aborted")),
            );
          }),
      ),
    );
    render(App);
    expect(screen.getByRole("status")).toBeTruthy();
    await vi.advanceTimersByTimeAsync(10000);
    expect(screen.getByRole("alert").textContent).toContain(
      "Reload to try again",
    );
    expect(screen.queryByText("Healthy")).toBeNull();
  } finally {
    vi.useRealTimers();
  }
});
