import { writable, get } from "svelte/store";

export type ThemePreference = "system" | "light" | "dark";
import { locationEvent, updateQuery } from "./link-state";
let fallback: ThemePreference = "system",
  listening = false;
function fromLocation() {
  const value = new URLSearchParams(window.location.search).get("theme");
  apply(
    value === "light" || value === "dark" || value === "system"
      ? value
      : fallback,
  );
}
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
  fallback = normalize(saved);
  fromLocation();
  if (!listening) {
    listening = true;
    window.addEventListener("popstate", fromLocation);
    window.addEventListener(locationEvent, fromLocation);
  }
}
/** Apply immediately; storage failure never blocks this tab or authentication. */
export function setTheme(value: string) {
  if (!new URLSearchParams(window.location.search).has("theme"))
    updateQuery({ theme: get(choice) }, true);
  const next = normalize(value);
  fallback = next;
  apply(next);
  updateQuery({ theme: next });
  try {
    localStorage.setItem(key, next);
  } catch {
    // Keep the in-memory preference when persistence is unavailable.
  }
}
