import { beforeEach } from "vitest";
beforeEach(() => {
  window.history.replaceState(null, "", "/");
  window.dispatchEvent(new Event("popstate"));
});
