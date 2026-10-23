import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, expect, test, vi } from "vitest";
import Header from "./Header.svelte";
afterEach(cleanup);
const menu = () => screen.getByRole("button", { name: "Navigation menu" });
test("navigation is hidden until opened and owner actions need a session", async () => {
  render(Header);
  expect(screen.getByRole("heading", { name: "Silkboard" })).toBeTruthy();
  expect(menu().getAttribute("aria-expanded")).toBe("false");
  expect(screen.queryByRole("navigation")).toBeNull();
  await fireEvent.click(menu());
  expect(menu().getAttribute("aria-expanded")).toBe("true");
  expect(
    screen.getByRole("link", { name: "Events" }).getAttribute("href"),
  ).toBe("/");
  expect(
    screen
      .getByRole("link", { name: "Application status" })
      .getAttribute("href"),
  ).toBe("/status");
  expect(screen.queryByRole("button", { name: "Sign out" })).toBeNull();
  await fireEvent.click(menu());
  expect(screen.queryByRole("navigation")).toBeNull();
});
test("owner actions close the menu and return focus", async () => {
  const onmanage = vi.fn(),
    onlogout = vi.fn();
  const view = render(Header, {
    owner: { managing: false, onmanage, onlogout },
  });
  await fireEvent.click(menu());
  await fireEvent.click(
    screen.getByRole("button", { name: "Manage publisher credentials" }),
  );
  expect(onmanage).toHaveBeenCalledOnce();
  expect(document.activeElement).toBe(menu());
  expect(screen.queryByRole("navigation")).toBeNull();
  await view.rerender({ owner: { managing: true, onmanage, onlogout } });
  await fireEvent.click(menu());
  expect(
    screen.getByRole("button", { name: "Close credential manager" }),
  ).toBeTruthy();
  await fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
  expect(onlogout).toHaveBeenCalledOnce();
  expect(screen.queryByRole("navigation")).toBeNull();
});
test("Escape, outside pointer and focus dismiss without capturing other keys", async () => {
  render(Header);
  await fireEvent.keyDown(window, { key: "Escape" });
  await fireEvent.click(menu());
  await fireEvent.keyDown(window, { key: "Tab" });
  expect(screen.getByRole("navigation")).toBeTruthy();
  await fireEvent.pointerDown(menu());
  expect(screen.getByRole("navigation")).toBeTruthy();
  await fireEvent.keyDown(window, { key: "Escape" });
  expect(document.activeElement).toBe(menu());
  expect(screen.queryByRole("navigation")).toBeNull();
  await fireEvent.click(menu());
  await fireEvent.pointerDown(document.body);
  expect(screen.queryByRole("navigation")).toBeNull();
  await fireEvent.click(menu());
  await fireEvent.focusIn(document.body);
  expect(screen.queryByRole("navigation")).toBeNull();
});
test("setup header has no navigation", () => {
  render(Header, { showMenu: false });
  expect(screen.queryByRole("button")).toBeNull();
});
