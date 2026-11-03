import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import AgentPicker from "./AgentPicker.svelte";
import { agent } from "../test/agents";
import { page, response } from "../test/support";
import { mockDialogs } from "../test/dialog";
beforeEach(() => {
  mockDialogs();
  vi.useFakeTimers();
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.useRealTimers();
});
const settle = () => vi.advanceTimersByTimeAsync(200);
const props = () => ({
  project: "silkboard",
  value: null,
  onselect: vi.fn(),
  onexpire: vi.fn(),
});
const open = () =>
  fireEvent.click(screen.getByRole("button", { name: /^Agent:/ }));
test("selected identities stay distinguishable and unnamed event agents use a tool/host fallback", async () => {
  const unnamed = {
    ...agent,
    session_name: null,
    session_id: "unnamed",
    latest: { kind: "event", event_id: "e", message: "Update" },
  };
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response(page([agent, unnamed]))),
  );
  const values = { ...props(), project: "", value: agent };
  render(AgentPicker, values);
  await open();
  await settle();
  expect(
    screen
      .getByRole("button", { name: /Build feature.*silkboard/ })
      .getAttribute("aria-pressed"),
  ).toBe("true");
  await fireEvent.click(
    screen.getByRole("button", { name: /codex on dev.*silkboard/ }),
  );
  expect(values.onselect).toHaveBeenCalledWith(unnamed);
});
test("searches complete main-session history with scope, pages and selects exact identity", async () => {
  const second = { ...agent, session_id: "second", session_name: null };
  const fetch = vi.fn(async (url) =>
    response(
      url.includes("before=") ? page([second]) : page([agent], "cursor & one"),
    ),
  );
  vi.stubGlobal("fetch", fetch);
  const values = props();
  render(AgentPicker, values);
  await open();
  await settle();
  await fireEvent.click(screen.getByText("More agents"));
  await settle();
  expect(fetch.mock.calls.at(-1)[0]).toContain("before=cursor%20%26%20one");
  await fireEvent.input(screen.getByLabelText("Search agents"), {
    target: { value: "Build" },
  });
  await settle();
  expect(fetch.mock.calls.at(-1)[0]).toContain("project=silkboard&q=Build");
  expect(fetch.mock.calls.at(-1)[0]).not.toContain("before=");
  await fireEvent.click(
    screen.getByRole("button", { name: /Build feature.*silkboard/ }),
  );
  expect(values.onselect).toHaveBeenCalledWith(agent);
  await open();
  await settle();
  await fireEvent.click(
    screen.getByRole("button", { name: "All agents", exact: true }),
  );
  expect(values.onselect).toHaveBeenLastCalledWith(null);
});
test("failed reads retry, empty results recover, and expiry reaches the owner boundary", async () => {
  let status = 503;
  vi.stubGlobal(
    "fetch",
    vi.fn(async () =>
      status === 200 ? response(page([])) : response({}, status),
    ),
  );
  const values = props();
  render(AgentPicker, values);
  await open();
  await settle();
  expect(screen.getByRole("alert").textContent).toContain("Unable");
  status = 200;
  await fireEvent.click(screen.getByText("Retry agent choices"));
  await settle();
  expect(screen.getByText(/No matching main agents/)).toBeTruthy();
  await fireEvent.keyDown(window, { key: "Escape" });
  status = 401;
  await open();
  await settle();
  expect(values.onexpire).toHaveBeenCalledOnce();
});
test.each([true, false])(
  "aborted reads ignore late resolution or rejection (%s)",
  async (reject) => {
    let finish, signal;
    vi.stubGlobal(
      "fetch",
      vi.fn((_, options) => {
        signal = options.signal;
        return new Promise(
          (resolve, fail) => (finish = reject ? fail : resolve),
        );
      }),
    );
    const values = props(),
      view = render(AgentPicker, values);
    await open();
    await settle();
    view.unmount();
    expect(signal.aborted).toBe(true);
    finish(reject ? new Error("late") : response(page([agent])));
    await settle();
    expect(values.onexpire).not.toHaveBeenCalled();
  },
);
