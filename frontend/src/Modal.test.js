import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import ModalFixture from "../test/ModalFixture.svelte";
import { tick } from "svelte";
import { mockDialogs } from "../test/dialog";
beforeEach(() => {
  mockDialogs();
  vi.spyOn(HTMLElement.prototype, "getClientRects").mockReturnValue([{}]);
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});
test("dialog focuses search, wraps keyboard navigation and closes without accepting edits", async () => {
  render(ModalFixture);
  await tick();
  expect(document.activeElement).toBe(screen.getByLabelText("Search choices"));
  const first = screen.getByRole("button", { name: "Close test choices" }),
    last = screen.getByRole("button", { name: "Last choice" });
  first.focus();
  await fireEvent.keyDown(window, { key: "Tab", shiftKey: true });
  expect(document.activeElement).toBe(last);
  await fireEvent.keyDown(window, { key: "Tab" });
  expect(document.activeElement).toBe(first);
  await fireEvent.keyDown(window, { key: "x" });
  expect(document.activeElement).toBe(first);
  await fireEvent.keyDown(window, { key: "Escape" });
  expect(screen.queryByRole("dialog")).toBeNull();
  await fireEvent.keyDown(window, { key: "Tab" });
  await fireEvent.click(screen.getByText("Open choices"));
  await fireEvent(
    screen.getByRole("dialog"),
    new Event("cancel", { cancelable: true }),
  );
  expect(screen.queryByRole("dialog")).toBeNull();
  await fireEvent.click(screen.getByText("Open choices"));
  await fireEvent.click(
    screen.getByRole("button", { name: "Close test choices" }),
  );
  expect(screen.queryByRole("dialog")).toBeNull();
});
