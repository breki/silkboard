import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import FilterPicker from "./FilterPicker.svelte";
import { page, response } from "../test/support";
beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});
const props = () => ({
  view: "events",
  kind: "project",
  value: "",
  onselect: vi.fn(),
  onexpire: vi.fn(),
});
const settle = () => vi.advanceTimersByTimeAsync(200);
const trigger = () => screen.getByRole("button", { name: /Project:/ });
test("opens a searchable bounded list, pages, searches, selects and restores focus", async () => {
  const fetch = vi.fn(async (path) => {
    const q = new URL(path, "https://fixture").searchParams;
    return response(
      q.get("q")
        ? page(["zebra"])
        : q.has("after")
          ? page(["zebra"])
          : page(["alpha"], "alpha"),
    );
  });
  vi.stubGlobal("fetch", fetch);
  const values = props();
  render(FilterPicker, values);
  await fireEvent.click(trigger());
  expect(document.activeElement).toBe(screen.getByLabelText("Search projects"));
  await settle();
  await fireEvent.click(screen.getByText("More projects"));
  await settle();
  expect(fetch.mock.calls.at(-1)[0]).toContain("after=alpha");
  await fireEvent.input(screen.getByLabelText("Search projects"), {
    target: { value: "z" },
  });
  await settle();
  expect(fetch.mock.calls.at(-1)[0]).not.toContain("after=");
  await fireEvent.click(
    screen.getByRole("button", { name: "zebra", exact: true }),
  );
  expect(values.onselect).toHaveBeenCalledWith("zebra");
  expect(document.activeElement).toBe(trigger());
  expect(trigger().getAttribute("aria-expanded")).toBe("false");
});
test("all choices clears selection, Escape and outside interactions dismiss the picker", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response(page([]))),
  );
  const values = { ...props(), value: "alpha" };
  render(FilterPicker, values);
  await fireEvent.click(trigger());
  await settle();
  expect(screen.getByText("No matching projects.")).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("button", { name: "All projects", exact: true }),
  );
  expect(values.onselect).toHaveBeenCalledWith("");
  await fireEvent.click(trigger());
  await fireEvent.keyDown(window, { key: "Escape" });
  expect(document.activeElement).toBe(trigger());
  await fireEvent.click(trigger());
  await fireEvent.pointerDown(document.body);
  expect(trigger().getAttribute("aria-expanded")).toBe("false");
  await fireEvent.click(trigger());
  await fireEvent.focusIn(document.body);
  expect(trigger().getAttribute("aria-expanded")).toBe("false");
  await fireEvent.click(trigger());
  await fireEvent.click(trigger());
  expect(trigger().getAttribute("aria-expanded")).toBe("false");
});
test("failed reads retry, and expired reads notify the owner boundary", async () => {
  let status = 503;
  vi.stubGlobal(
    "fetch",
    vi.fn(async () =>
      status === 200 ? response(page(["alpha"])) : response({}, status),
    ),
  );
  const values = props();
  render(FilterPicker, values);
  await fireEvent.click(trigger());
  await settle();
  expect(screen.getByRole("alert").textContent).toContain(
    "Unable to load projects",
  );
  status = 200;
  await fireEvent.click(screen.getByText("Retry projects"));
  await settle();
  expect(
    screen.getByRole("button", { name: "alpha", exact: true }),
  ).toBeTruthy();
  await fireEvent.click(trigger());
  status = 401;
  await fireEvent.click(trigger());
  await settle();
  expect(values.onexpire).toHaveBeenCalledOnce();
});
test.each([true, false])(
  "late response is ignored after closing or teardown (%s)",
  async (unmount) => {
    let finish, signal;
    vi.stubGlobal(
      "fetch",
      vi.fn((_, options) => {
        signal = options.signal;
        return new Promise((resolve) => (finish = resolve));
      }),
    );
    const values = props();
    const view = render(FilterPicker, values);
    await fireEvent.click(trigger());
    await settle();
    if (unmount) view.unmount();
    else await fireEvent.click(trigger());
    expect(signal.aborted).toBe(true);
    finish(response(page(["alpha"])));
    await settle();
    expect(
      screen.queryByRole("button", { name: "alpha", exact: true }),
    ).toBeNull();
  },
);
