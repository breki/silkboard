import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { get } from "svelte/store";
import { initializeTheme, setTheme, themePreference } from "./theme";
beforeEach(() => {
  localStorage.clear();
  delete document.documentElement.dataset.theme;
});
afterEach(() => vi.unstubAllGlobals());
test("defaults to System and accepts only recognized saved choices", () => {
  initializeTheme();
  expect(get(themePreference)).toBe("system");
  expect(document.documentElement.dataset.theme).toBe("system");
  for (const choice of ["light", "dark", "system"]) {
    localStorage.setItem("silkboard.theme", choice);
    initializeTheme();
    expect(get(themePreference)).toBe(choice);
    expect(document.documentElement.dataset.theme).toBe(choice);
  }
  localStorage.setItem("silkboard.theme", "unexpected");
  initializeTheme();
  expect(get(themePreference)).toBe("system");
});
test("changes apply immediately and survive reinitialization", () => {
  initializeTheme();
  for (const choice of ["dark", "light", "system"]) {
    setTheme(choice);
    expect(document.documentElement.dataset.theme).toBe(choice);
    expect(localStorage.getItem("silkboard.theme")).toBe(choice);
    initializeTheme();
    expect(get(themePreference)).toBe(choice);
  }
  setTheme("invalid");
  expect(get(themePreference)).toBe("system");
});
test("blocked browser storage does not prevent current-tab switching", () => {
  vi.stubGlobal("localStorage", {
    getItem() {
      throw new Error("blocked");
    },
    setItem() {
      throw new Error("blocked");
    },
  });
  initializeTheme();
  expect(get(themePreference)).toBe("system");
  setTheme("dark");
  expect(get(themePreference)).toBe("dark");
  expect(document.documentElement.dataset.theme).toBe("dark");
});
