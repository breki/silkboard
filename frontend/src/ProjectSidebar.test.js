import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import ProjectSidebar from "./ProjectSidebar.svelte";
import { page, response } from "../test/support";
let media, changed;
beforeEach(() => {
  media = {
    matches: true,
    addEventListener: vi.fn((_, fn) => (changed = fn)),
    removeEventListener: vi.fn(),
  };
  vi.stubGlobal("matchMedia", () => media);
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});
const props = () => ({
  view: "agents",
  value: "selected-not-yet-loaded",
  onselect: vi.fn(),
  onexpire: vi.fn(),
});
test("loads all-view projects, pages and selects while retaining button focus", async () => {
  const fetch = vi.fn(async (url) =>
    response(
      url.includes("after=") ? page(["beta"]) : page(["alpha"], "alpha"),
    ),
  );
  vi.stubGlobal("fetch", fetch);
  const values = props();
  render(ProjectSidebar, values);
  const alpha = await screen.findByRole("button", {
    name: "alpha",
    exact: true,
  });
  alpha.focus();
  await fireEvent.click(alpha);
  expect(values.onselect).toHaveBeenCalledWith("alpha");
  expect(document.activeElement).toBe(alpha);
  await fireEvent.click(screen.getByRole("button", { name: "More projects" }));
  expect(
    await screen.findByRole("button", { name: "beta", exact: true }),
  ).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("button", { name: "Show all projects" }),
  );
  expect(values.onselect).toHaveBeenLastCalledWith("");
  await fireEvent.click(
    screen.getByRole("button", { name: "selected-not-yet-loaded" }),
  );
  expect(values.onselect).toHaveBeenLastCalledWith("selected-not-yet-loaded");
  media.matches = false;
  changed();
  expect(media.removeEventListener).not.toHaveBeenCalled();
});
test("failures retry without granting new access and empty choices remain usable", async () => {
  let status = 503;
  const values = props();
  vi.stubGlobal(
    "fetch",
    vi.fn(async () =>
      status === 200 ? response(page([])) : response({}, status),
    ),
  );
  render(ProjectSidebar, values);
  expect(await screen.findByRole("alert")).toBeTruthy();
  status = 200;
  await fireEvent.click(screen.getByText("Retry projects"));
  expect(await screen.findByText("No projects with reports.")).toBeTruthy();
  media.matches = false;
  changed();
  await new Promise((resolve) => setTimeout(resolve, 0));
  status = 401;
  media.matches = true;
  changed();
  await vi.waitFor(() => expect(values.onexpire).toHaveBeenCalledOnce());
});
test.each([true, false])(
  "unmount ignores late read completion (%s)",
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
      view = render(ProjectSidebar, values);
    await new Promise((resolve) => setTimeout(resolve, 0));
    view.unmount();
    expect(signal.aborted).toBe(true);
    finish(reject ? new Error("late") : response(page(["alpha"])));
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(values.onexpire).not.toHaveBeenCalled();
  },
);
