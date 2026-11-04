import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import ListFilters from "./ListFilters.svelte";
import { filterQuery } from "./list-filters";
import { page, response } from "../test/support";
import { agent } from "../test/agents";
import { mockDialogs } from "../test/dialog";
beforeEach(mockDialogs);
afterEach(() => {
  vi.restoreAllMocks();
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

test("event choices and ranges apply; project changes reset dependent agents", async () => {
  options();
  const onapply = vi.fn();
  render(ListFilters, { onapply, onexpire: vi.fn() });
  await choose("Project", "alpha");
  await choose("Agent", "codex · dev");
  await fireEvent.click(
    screen.getByRole("button", { name: "Received time: All time" }),
  );
  for (const [label, value] of [
    ["From date", "2026-10-25"],
    ["From time", "10:00"],
    ["To date", "2026-10-25"],
    ["To time", "11:00"],
  ])
    await fireEvent.input(screen.getByLabelText(label), { target: { value } });
  await fireEvent.click(screen.getByText("Apply range"));
  expect(onapply).toHaveBeenLastCalledWith({
    project: "alpha",
    agent: "codex dev",
    from: new Date("2026-10-25T10:00").getTime(),
    to: new Date("2026-10-25T11:00").getTime(),
  });
  await fireEvent.click(screen.getByRole("button", { name: "Project: alpha" }));
  await fireEvent.click(
    screen.getByRole("button", { name: "All projects", exact: true }),
  );
  expect(onapply.mock.calls.at(-1)[0].agent).toBe("");
  await fireEvent.click(screen.getByText("Clear filters"));
  expect(onapply).toHaveBeenLastCalledWith({});
  expect(
    screen.getByRole("button", { name: "Received time: All time" }),
  ).toBeTruthy();
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
test("an agent selection includes its full project identity and project changes clear it", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (path) =>
      response(
        page(path.startsWith("/api/owner/agents") ? [agent] : ["alpha"]),
      ),
    ),
  );
  const onapply = vi.fn();
  render(ListFilters, { agentsView: true, onapply, onexpire: vi.fn() });
  await fireEvent.click(
    screen.getByRole("button", { name: "Agent: All agents" }),
  );
  await fireEvent.click(
    await screen.findByRole("button", {
      name: /Build feature.*silkboard.*codex/,
    }),
  );
  expect(onapply).toHaveBeenLastCalledWith({
    project: "silkboard",
    hostname: "dev",
    tool: "codex",
    session_id: "native",
  });
  await choose("Project", "alpha");
  expect(onapply).toHaveBeenLastCalledWith({ project: "alpha" });
});
