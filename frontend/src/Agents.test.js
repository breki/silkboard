import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import Agents from "./Agents.svelte";
import { agent } from "../test/agents";
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
test("links reported workflow snapshots and exact event identities without merging equal names", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () =>
      response(
        page([
          agent,
          {
            ...agent,
            session_id: "other",
            subagent_id: null,
            latest: {
              kind: "event",
              event_id: "e",
              message: "**A short update**",
            },
          },
        ]),
      ),
    ),
  );
  render(Agents, { onexpire: vi.fn() });
  await settle();
  const links = screen.getAllByRole("link");
  expect(screen.getAllByRole("article")).toHaveLength(2);
  expect(screen.queryByRole("table")).toBeNull();
  expect(links).toHaveLength(2);
  expect(links[0].getAttribute("href")).toBe(
    `/plans/silkboard/${agent.latest.task_id}?sequence=1`,
  );
  const query = new URL(links[1].href).searchParams;
  expect(query.get("session_id")).toBe("other");
  expect(query.get("subagent_id")).toBe("");
  expect(screen.getByText("Reported active")).toBeTruthy();
  expect(screen.getByText("Event: A short update")).toBeTruthy();
  expect(screen.getAllByText("Native session")).toHaveLength(2);
});
test("pages, polls, retains stale rows after failure and recovers", async () => {
  let fail = false;
  const fetch = vi.fn(async (path) =>
    fail
      ? response({}, 503)
      : response(
          path.includes("?") ? page([]) : page([agent], "opaque & cursor"),
        ),
  );
  vi.stubGlobal("fetch", fetch);
  render(Agents, { onexpire: vi.fn() });
  await settle();
  fail = true;
  await vi.advanceTimersByTimeAsync(10000);
  expect(screen.getByRole("list", { name: "Main agents" })).toBeTruthy();
  expect(screen.getByRole("alert").textContent).toContain("out of date");
  fail = false;
  await fireEvent.click(screen.getByText("Retry agents"));
  await settle();
  await fireEvent.click(screen.getByText("Older agents"));
  await settle();
  expect(fetch.mock.calls.at(-1)[0]).toContain("before=opaque%20%26%20cursor");
  expect(
    screen.getByText(
      "No main agents on this page. Return to the latest activity.",
    ),
  ).toBeTruthy();
  await fireEvent.click(screen.getByText("Latest agents"));
  await settle();
  expect(fetch.mock.calls.at(-1)[0]).toBe("/api/owner/agents");
});
test("empty and expired reads do not invent agents", async () => {
  let code = 200;
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => (code === 200 ? response(page([])) : response({}, code))),
  );
  const onexpire = vi.fn();
  render(Agents, { onexpire });
  await settle();
  expect(
    screen.getByText("No main agent reports have arrived yet."),
  ).toBeTruthy();
  code = 401;
  await vi.advanceTimersByTimeAsync(10000);
  expect(onexpire).toHaveBeenCalledOnce();
});
test("unmount aborts late reads and never restores agent content", async () => {
  let done, signal;
  vi.stubGlobal(
    "fetch",
    vi.fn((_, opts) => {
      signal = opts.signal;
      return new Promise((resolve) => (done = resolve));
    }),
  );
  const view = render(Agents, { onexpire: vi.fn() });
  await settle();
  view.unmount();
  expect(signal.aborted).toBe(true);
  done(response(page([agent])));
  await settle();
  expect(screen.queryByRole("article")).toBeNull();
});

test("initial failure retries with an unnamed main session", async () => {
  let fail = true;
  vi.stubGlobal(
    "fetch",
    vi.fn(async () =>
      fail
        ? response({}, 503)
        : response(
            page([
              {
                ...agent,
                session_name: null,
                subagent_id: null,
                parent_session_id: null,
              },
            ]),
          ),
    ),
  );
  render(Agents, { onexpire: vi.fn() });
  await settle();
  expect(screen.getByRole("alert")).toBeTruthy();
  expect(screen.queryByRole("article")).toBeNull();
  fail = false;
  await fireEvent.click(screen.getByRole("button", { name: "Retry agents" }));
  await settle();
  expect(screen.getByRole("link", { name: "codex on dev" })).toBeTruthy();
  expect(screen.getByText("native")).toBeTruthy();
});
test.each([false, true])(
  "teardown ignores late body success or failure (%s)",
  async (fail) => {
    let done;
    const body = new Promise(
      (resolve, reject) => (done = fail ? reject : resolve),
    );
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({ ok: true, status: 200, json: () => body })),
    );
    const onexpire = vi.fn();
    const view = render(Agents, { onexpire });
    await settle();
    view.unmount();
    done(fail ? new Error("Late failure") : page([agent]));
    await settle();
    expect(screen.queryByRole("article")).toBeNull();
    expect(onexpire).not.toHaveBeenCalled();
  },
);
