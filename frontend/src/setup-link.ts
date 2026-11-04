// Fragments never enter HTTP request URLs. Remove this history entry before
// rendering; never put the credential in DOM, cookies or browser storage.
export function takeSetupLink(
  location: Pick<Location, "hash" | "pathname"> &
    Partial<Pick<Location, "search">>,
  history: Pick<History, "replaceState">,
): string {
  if (location.pathname !== "/setup") return "";
  const fragment = location.hash;
  const query = new URLSearchParams(location.search ?? "");
  const theme = query.get("theme");
  const appearance =
    query.getAll("theme").length === 1 &&
    ["system", "light", "dark"].includes(theme ?? "")
      ? `?theme=${theme}`
      : "";
  history.replaceState(null, "", "/setup" + appearance);
  return /^#setup=[0-9a-f]{64}$/.test(fragment) ? fragment.slice(7) : "";
}
