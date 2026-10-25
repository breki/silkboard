import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, expect, test, vi } from "vitest";
import ListFilters from "./ListFilters.svelte";
import { filterQuery } from "./list-filters";
import { page, response } from "../test/support";
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
const options = () =>
  vi.stubGlobal(
    "fetch",
    vi.fn(async (path) =>
      response(
        page(
          new URL(path, "https://fixture").searchParams.get("kind") ===
            "project"
            ? ["alpha"]
            : ["codex dev"],
        ),
      ),
    ),
  );
async function choose(kind, choice) {
  await fireEvent.click(
    screen.getByRole("button", {
      name: `${kind}: All ${kind === "Project" ? "projects" : "agents"}`,
    }),
  );
  await fireEvent.click(
    await screen.findByRole("button", { name: choice, exact: true }),
  );
}

test("event choices and valid time changes filter immediately; invalid ranges retain active filters", async () => {
  options();
  const onapply = vi.fn();
  render(ListFilters, { onapply, onexpire: vi.fn() });
  await choose("Project", "alpha");
  await choose("Agent", "codex · dev");
  await fireEvent.change(screen.getByLabelText("From"), {
    target: { value: "2026-10-25T10:00" },
  });
  await fireEvent.change(screen.getByLabelText("To"), {
    target: { value: "2026-10-25T11:00" },
  });
  expect(onapply).toHaveBeenLastCalledWith({
    project: "alpha",
    agent: "codex dev",
    from: new Date("2026-10-25T10:00").getTime(),
    to: new Date("2026-10-25T11:00").getTime(),
  });
  const calls = onapply.mock.calls.length;
  await fireEvent.change(screen.getByLabelText("To"), {
    target: { value: "2026-10-25T09:00" },
  });
  expect(screen.getByRole("alert").textContent).toContain(
    "last valid time range stays active",
  );
  expect(onapply).toHaveBeenCalledTimes(calls);
  await fireEvent.click(screen.getByRole("button", { name: "Project: alpha" }));
  await fireEvent.click(
    screen.getByRole("button", { name: "All projects", exact: true }),
  );
  expect(onapply).toHaveBeenLastCalledWith({
    project: "",
    agent: "codex dev",
    from: new Date("2026-10-25T10:00").getTime(),
    to: new Date("2026-10-25T11:00").getTime(),
  });

  await fireEvent.change(screen.getByLabelText("To"), {
    target: { value: "" },
  });
  expect(screen.queryByRole("alert")).toBeNull();
  await fireEvent.change(screen.getByLabelText("From"), {
    target: { value: "1969-01-01T00:00" },
  });
  expect(screen.getByRole("alert")).toBeTruthy();
  await fireEvent.click(screen.getByText("Clear filters"));
  expect(onapply).toHaveBeenLastCalledWith({});
  expect(
    screen.getByRole("button", { name: "Project: All projects" }),
  ).toBeTruthy();
  expect(screen.queryByRole("alert")).toBeNull();
  await fireEvent.submit(screen.getByRole("form"));
  expect(screen.queryByText("Apply filters")).toBeNull();
});
test("workflow status buttons filter immediately and show the selected state", async () => {
  const onapply = vi.fn();
  render(ListFilters, { workflows: true, onapply, onexpire: vi.fn() });
  await fireEvent.click(
    screen.getByRole("button", { name: "Blocked", exact: true }),
  );
  expect(onapply).toHaveBeenLastCalledWith({
    project: "",
    agent: "",
    state: "blocked",
  });
  expect(
    screen
      .getByRole("button", { name: "Blocked" })
      .getAttribute("aria-pressed"),
  ).toBe("true");
  await fireEvent.click(screen.getByRole("button", { name: "All statuses" }));
  expect(onapply).toHaveBeenLastCalledWith({
    project: "",
    agent: "",
    state: "",
  });
  expect(screen.queryByLabelText("From")).toBeNull();
  expect(screen.queryByText("Apply filters")).toBeNull();
});
test("query preserves opaque cursors and encodes literal filter values", () => {
  expect(filterQuery(null, {})).toBe("");
  expect(filterQuery("a & b", { agent: "host%_", from: 0, project: "" })).toBe(
    "?before=a%20%26%20b&agent=host%25_&from=0",
  );
});
