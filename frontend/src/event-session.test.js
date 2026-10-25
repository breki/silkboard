import { expect, test } from "vitest";
import { clearSessionUrl, readSession, sessionEvents } from "./event-session";
import { filterQuery } from "./list-filters";
const identity = {
  project: "alpha",
  hostname: "dev",
  tool: "codex",
  session_id: "native&x",
  subagent_id: null,
};
test("exact primary and child identities round-trip and retain explicit empty primary scope", () => {
  for (const child of [null, "child&x"]) {
    const value = { ...identity, subagent_id: child };
    const url = sessionEvents(value);
    expect(readSession(new URL(url, "https://fixture").search)).toEqual({
      kind: "session",
      value,
    });
  }
  expect(filterQuery(null, { ...identity, subagent_id: "" })).toContain(
    "subagent_id=",
  );
  expect(readSession("")).toEqual({ kind: "none" });
});
test.each([
  "?session_id=native",
  "?project=BAD&hostname=dev&tool=codex&session_id=x&subagent_id=",
  "?project=alpha&hostname=dev&tool=codex&session_id=x&subagent_id=&session_id=y",
  "?project=alpha&hostname=dev&tool=%0A&session_id=x&subagent_id=",
  "?project=alpha&hostname=dev&tool=codex&session_id=x&subagent_id=bad%20child",
])("rejects incomplete or invalid exact filter %s", (query) =>
  expect(readSession(query)).toEqual({ kind: "invalid" }),
);
test("invalid identity cannot create a broad link; clearing preserves unrelated URL state", () => {
  expect(() => sessionEvents({ ...identity, hostname: "" })).toThrow();
  window.history.replaceState(
    null,
    "",
    sessionEvents(identity) + "&keep=1#anchor",
  );
  expect(clearSessionUrl()).toBe("/events?keep=1#anchor");
  window.history.replaceState(null, "", "/");
});
