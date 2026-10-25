import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test } from "vitest";
import ThemeSelector from "./ThemeSelector.svelte";
import { initializeTheme, setTheme } from "./theme";
beforeEach(() => {
  localStorage.clear();
  initializeTheme();
});
afterEach(cleanup);
test("offers System by default and persists each user choice", async () => {
  render(ThemeSelector);
  const select = screen.getByRole("combobox", { name: "Theme" });
  expect(select.value).toBe("system");
  expect(screen.getAllByRole("option").map((x) => x.textContent)).toEqual([
    "System",
    "Light",
    "Dark",
  ]);
  for (const value of ["dark", "light", "system"]) {
    await fireEvent.change(select, { target: { value } });
    expect(document.documentElement.dataset.theme).toBe(value);
    expect(localStorage.getItem("silkboard.theme")).toBe(value);
  }
});
test("reflects the shared preference when it changes outside this control", async () => {
  render(ThemeSelector);
  setTheme("dark");
  await waitFor(() =>
    expect(screen.getByRole("combobox", { name: "Theme" }).value).toBe("dark"),
  );
  setTheme("light");
  await waitFor(() =>
    expect(screen.getByRole("combobox", { name: "Theme" }).value).toBe("light"),
  );
});
