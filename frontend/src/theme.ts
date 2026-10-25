import { writable } from "svelte/store";

export type ThemePreference = "system" | "light" | "dark";
const key = "silkboard.theme";
const choice = writable<ThemePreference>("system");
export const themePreference = { subscribe: choice.subscribe };

function normalize(value: unknown): ThemePreference {
  return value === "light" || value === "dark" ? value : "system";
}
function apply(value: ThemePreference) {
  document.documentElement.dataset.theme = value;
  choice.set(value);
}
/** Apply a saved browser choice before mounting the application. */
export function initializeTheme() {
  let saved: unknown;
  try {
    saved = localStorage.getItem(key);
  } catch {
    // Browser privacy settings may make storage unavailable.
  }
  apply(normalize(saved));
}
/** Apply immediately; storage failure never blocks this tab or authentication. */
export function setTheme(value: string) {
  const next = normalize(value);
  apply(next);
  try {
    localStorage.setItem(key, next);
  } catch {
    // Keep the in-memory preference when persistence is unavailable.
  }
}
