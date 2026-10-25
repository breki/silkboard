import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, expect, test, vi } from "vitest";
import ListFilters from "./ListFilters.svelte";
import { filterQuery } from "./list-filters";
afterEach(cleanup);

test("applies combined event filters and clears every field", async () => {
  const onapply = vi.fn();
  render(ListFilters, { onapply });
  await fireEvent.input(screen.getByLabelText("Project"), {
    target: { value: "alpha" },
  });
  await fireEvent.input(screen.getByLabelText("Agent"), {
    target: { value: "codex dev" },
  });
  await fireEvent.input(screen.getByLabelText("From"), {
    target: { value: "2026-10-25T10:00" },
  });
  await fireEvent.input(screen.getByLabelText("To"), {
    target: { value: "2026-10-25T11:00" },
  });
  await fireEvent.submit(screen.getByRole("form"));
  expect(onapply).toHaveBeenLastCalledWith({
    project: "alpha",
    agent: "codex dev",
    from: new Date("2026-10-25T10:00").getTime(),
    to: new Date("2026-10-25T11:00").getTime(),
  });
  await fireEvent.input(screen.getByLabelText("To"), {
    target: { value: "2026-10-25T09:00" },
  });
  await fireEvent.submit(screen.getByRole("form"));
  expect(screen.getByRole("alert").textContent).toContain("valid time range");
  expect(onapply).toHaveBeenCalledTimes(1);
  await fireEvent.click(screen.getByText("Clear filters"));
  expect(onapply).toHaveBeenLastCalledWith({});
  expect(screen.getByLabelText("Project").value).toBe("");
  expect(screen.queryByRole("alert")).toBeNull();
});
test("workflow controls apply status without event time fields", async () => {
  const onapply = vi.fn();
  render(ListFilters, { workflows: true, onapply });
  await fireEvent.change(screen.getByLabelText("Workflow status"), {
    target: { value: "blocked" },
  });
  await fireEvent.submit(screen.getByRole("form"));
  expect(onapply).toHaveBeenLastCalledWith({
    project: "",
    agent: "",
    state: "blocked",
  });
  expect(screen.queryByLabelText("From")).toBeNull();
});
test("query preserves opaque cursors and encodes literal filter values", () => {
  expect(filterQuery(null, {})).toBe("");
  expect(filterQuery("a & b", { agent: "host%_", from: 0, project: "" })).toBe(
    "?before=a%20%26%20b&agent=host%25_&from=0",
  );
});
