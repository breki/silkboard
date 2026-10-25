import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import TaskPlans from "./TaskPlans.svelte";
import { detail, task } from "../test/plans";
import { page, response } from "../test/support";
beforeEach(() => {
  vi.spyOn(document, "hidden", "get").mockReturnValue(false);
  window.history.replaceState(null, "", "/plans");
  vi.spyOn(window, "scrollTo").mockImplementation(() => {});
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.history.replaceState(null, "", "/");
});
test("lists plans and opens a stable directed graph with selected step details", async () => {
  const value = detail();
  vi.stubGlobal(
    "fetch",
    vi.fn(async (path) =>
      response(path === "/api/owner/plans" ? page([value.summary]) : value),
    ),
  );
  render(TaskPlans, { onexpire: vi.fn() });
  await screen.findByRole("table", { name: "Task plans from all projects" });
  await fireEvent.click(
    screen.getByRole("link", { name: value.snapshot.title }),
  );
  await screen.findByRole("region", { name: "Task prerequisite graph" });
  expect(window.location.pathname).toBe(`/plans/silkboard/${task}`);
  await fireEvent.click(
    screen.getByRole("button", { name: "Build graph page, Active" }),
  );
  expect(
    await screen.findByRole("heading", { name: "Build graph page" }),
  ).toBeTruthy();
  expect(screen.getByText("Implementing the owner view.")).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("button", { name: "Steps", exact: true }),
  );
  expect(
    screen.getByRole("list", { name: "Steps in prerequisite order" }),
  ).toBeTruthy();
});
test("shows explicit empty and unavailable states without fabricating tasks", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response(page([]))),
  );
  render(TaskPlans, { onexpire: vi.fn() });
  expect(
    await screen.findByText("No task-plan reports have arrived yet."),
  ).toBeTruthy();
});

test("invalid links recover locally and Back restores the previous list position", async () => {
  window.history.replaceState(null, "", "/plans/bad");
  const value = detail();
  vi.stubGlobal(
    "fetch",
    vi.fn(async (path) =>
      response(
        path === "/api/owner/plans"
          ? page([value.summary])
          : path.includes("/history") || path.includes("/revisions")
            ? page([value.summary])
            : value,
      ),
    ),
  );
  render(TaskPlans, { onexpire: vi.fn() });
  expect(
    screen.getByRole("heading", { name: "Invalid task-plan link" }),
  ).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("link", { name: "Return to Task plans" }),
  );
  await screen.findByRole("table");
  vi.spyOn(window, "scrollY", "get").mockReturnValue(123);
  const link = screen.getByRole("link", { name: value.snapshot.title });
  await fireEvent.click(link, { ctrlKey: true });
  expect(window.location.pathname).toBe("/plans");
  await fireEvent.click(link);
  await screen.findByRole("region", { name: "Task prerequisite graph" });
  await fireEvent.click(
    screen.getByRole("button", { name: "Received history and revisions" }),
  );
  await screen.findByRole("list", { name: "Received plan reports" });
  await fireEvent.click(
    screen.getByRole("button", { name: "Definition revisions", exact: true }),
  );
  await screen.findByRole("button", { name: "Revision 1", exact: true });
  await fireEvent.click(
    screen.getByRole("button", { name: "Revision 1", exact: true }),
  );
  expect(window.location.search).toBe("?sequence=1");
  await fireEvent.click(
    await screen.findByRole("link", { name: "Current plan", exact: true }),
  );
  expect(window.location.search).toBe("");
  window.history.replaceState(null, "", "/plans");
  window.dispatchEvent(new PopStateEvent("popstate"));
  await screen.findByRole("table");
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(window.scrollTo).toHaveBeenLastCalledWith(0, 123);
});
