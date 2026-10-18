// Fragments never enter HTTP request URLs. Remove this history entry before
// rendering; never put the credential in DOM, cookies or browser storage.
export function takeSetupLink(
  location: Pick<Location, "hash" | "pathname">,
  history: Pick<History, "replaceState">,
): string {
  if (location.pathname !== "/setup") return "";
  const fragment = location.hash;
  history.replaceState(null, "", "/setup");
  return /^#setup=[0-9a-f]{64}$/.test(fragment) ? fragment.slice(7) : "";
}
