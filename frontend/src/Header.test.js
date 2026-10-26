import {
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from "@testing-library/svelte";
import { afterEach, expect, test, vi } from "vitest";
import Header from "./Header.svelte";
afterEach(cleanup);
const menu = () => screen.getByRole("button", { name: "Navigation menu" });
const drawer = () =>
  screen.queryByRole("navigation", { name: "Main navigation" });
test("primary links are visible without opening the owner menu", async () => {
  render(Header);
  expect(screen.getByRole("heading", { name: "Silkboard" })).toBeTruthy();
  expect(menu().getAttribute("aria-expanded")).toBe("false");
  expect(drawer()).toBeNull();
  expect(
    screen.getByRole("link", { name: "Agents" }).getAttribute("href"),
  ).toBe("/");
  const primary = screen.getByRole("navigation", {
    name: "Primary navigation",
  });
  expect(
    within(primary).getByRole("link", { name: "Events" }).getAttribute("href"),
  ).toBe("/events");
  expect(
    within(primary)
      .getByRole("link", { name: "Workflows" })
      .getAttribute("href"),
  ).toBe("/plans");
  await fireEvent.click(menu());
  expect(menu().getAttribute("aria-expanded")).toBe("true");
  expect(within(drawer()).queryByRole("link", { name: "Events" })).toBeNull();
  expect(
    within(drawer()).queryByRole("link", { name: "Workflows" }),
  ).toBeNull();
  expect(
    screen.getByRole("link", { name: "Events" }).getAttribute("href"),
  ).toBe("/events");
  expect(
    screen
      .getByRole("link", { name: "Application status" })
      .getAttribute("href"),
  ).toBe("/status");
  expect(screen.queryByRole("button", { name: "Sign out" })).toBeNull();
  await fireEvent.click(menu());
  expect(drawer()).toBeNull();
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
  expect(drawer()).toBeNull();
  await view.rerender({ owner: { managing: true, onmanage, onlogout } });
  await fireEvent.click(menu());
  expect(
    screen.getByRole("button", { name: "Close credential manager" }),
  ).toBeTruthy();
  await fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
  expect(onlogout).toHaveBeenCalledOnce();
  expect(drawer()).toBeNull();
});
test("Escape, outside pointer and focus dismiss without capturing other keys", async () => {
  render(Header);
  await fireEvent.keyDown(window, { key: "Escape" });
  await fireEvent.click(menu());
  await fireEvent.keyDown(window, { key: "Tab" });
  expect(drawer()).toBeTruthy();
  await fireEvent.pointerDown(menu());
  expect(drawer()).toBeTruthy();
  await fireEvent.keyDown(window, { key: "Escape" });
  expect(document.activeElement).toBe(menu());
  expect(drawer()).toBeNull();
  await fireEvent.click(menu());
  await fireEvent.pointerDown(document.body);
  expect(drawer()).toBeNull();
  await fireEvent.click(menu());
  await fireEvent.focusIn(document.body);
  expect(drawer()).toBeNull();
});
test("setup header has no navigation", () => {
  render(Header, { showMenu: false });
  expect(screen.queryByRole("button")).toBeNull();
  expect(screen.queryByRole("link")).toBeNull();
});

test("theme preference can be changed and survives closing the menu", async () => {
  render(Header);
  await fireEvent.click(menu());
  const selector = screen.getByRole("combobox", { name: "Theme" });
  expect(selector.value).toBe("system");
  await fireEvent.change(selector, { target: { value: "dark" } });
  expect(document.documentElement.dataset.theme).toBe("dark");
  await fireEvent.click(menu());
  await fireEvent.click(menu());
  expect(screen.getByRole("combobox", { name: "Theme" }).value).toBe("dark");
});

test.each([
  ["agents", "Agents", "/"],
  ["events", "Events", "/events"],
  ["plans", "Workflows", "/plans"],
  ["todos", "Todos", "/todos"],
])(
  "all four primary links expose current page %s",
  async (currentPage, label, path) => {
    render(Header, { currentPage });
    const primary = screen.getByRole("navigation", {
      name: "Primary navigation",
    });
    expect(within(primary).getAllByRole("link")).toHaveLength(4);
    const current = within(primary).getByRole("link", {
      name: label,
      exact: true,
    });
    expect(current.getAttribute("href")).toBe(path);
    expect(current.getAttribute("aria-current")).toBe("page");
    expect(primary.querySelectorAll('[aria-current="page"]')).toHaveLength(1);
    await fireEvent.click(menu());
    expect(
      within(drawer()).queryByRole("link", { name: "Todos", exact: true }),
    ).toBeNull();
  },
);

test("primary navigation identifies the current page and changes with its route", async () => {
  const view = render(Header, { currentPage: "events" });
  expect(
    screen.getByRole("link", { name: "Events" }).getAttribute("aria-current"),
  ).toBe("page");
  expect(
    screen.getByRole("link", { name: "Agents" }).getAttribute("aria-current"),
  ).toBeNull();
  await view.rerender({ currentPage: "plans" });
  expect(
    screen
      .getByRole("link", { name: "Workflows" })
      .getAttribute("aria-current"),
  ).toBe("page");
  expect(
    screen.getByRole("link", { name: "Events" }).getAttribute("aria-current"),
  ).toBeNull();
  await view.rerender({ currentPage: "todos" });
  expect(
    screen.getByRole("link", { name: "Todos" }).getAttribute("aria-current"),
  ).toBe("page");
});
