import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import TimeRange from "./TimeRange.svelte";
import { mockDialogs } from "../test/dialog";
beforeEach(mockDialogs);
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});
test("custom edits apply together; invalid and cancelled edits preserve the current range", async () => {
  const onchange = vi.fn();
  const value = {
    from: new Date(2026, 10, 3, 9).getTime(),
    to: new Date(2026, 10, 3, 12).getTime(),
  };
  render(TimeRange, { value, onchange });
  await fireEvent.click(
    screen.getByRole("button", { name: /^Received time:/ }),
  );
  await fireEvent.input(screen.getByLabelText("From date"), {
    target: { value: "2026-02-30" },
  });
  await fireEvent.click(screen.getByText("Apply range"));
  expect(screen.getByRole("alert").textContent).toContain(
    "last valid time range stays active",
  );
  expect(onchange).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByText("Cancel"));
  expect(onchange).not.toHaveBeenCalled();
  await fireEvent.click(
    screen.getByRole("button", { name: /^Received time:/ }),
  );
  expect(screen.getByLabelText("From date").value).toBe("2026-11-03");
  await fireEvent.input(screen.getByLabelText("From time"), {
    target: { value: "13:00" },
  });
  await fireEvent.click(screen.getByText("Apply range"));
  expect(onchange).not.toHaveBeenCalled();
  await fireEvent.input(screen.getByLabelText("From time"), {
    target: { value: "10:00" },
  });
  await fireEvent.click(screen.getByText("Apply range"));
  expect(onchange).toHaveBeenCalledExactlyOnceWith({
    ...value,
    from: new Date(2026, 10, 3, 10).getTime(),
  });
});
test("presets use selection time and Escape discards drafts", async () => {
  const onchange = vi.fn();
  render(TimeRange, { value: {}, onchange });
  await fireEvent.click(
    screen.getByRole("button", { name: "Received time: All time" }),
  );
  await fireEvent.input(screen.getByLabelText("From date"), {
    target: { value: "2026-11-03" },
  });
  await fireEvent.keyDown(window, { key: "Escape" });
  expect(onchange).not.toHaveBeenCalled();
  await fireEvent.click(
    screen.getByRole("button", { name: "Received time: All time" }),
  );
  await fireEvent.click(screen.getByText("Last hour"));
  const applied = onchange.mock.calls[0][0];
  expect(applied.to - applied.from).toBe(3600000);
  expect(Math.abs(applied.to - Date.now())).toBeLessThan(1000);
});
