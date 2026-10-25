import { expect, test } from "vitest";
import { groupEvents, sessionKey } from "./event-hierarchy";
const event = (id, changes = {}) => ({
  project: "silkboard",
  hostname: "dev",
  tool: "codex",
  session_id: "root",
  subagent_id: null,
  parent_session_id: null,
  session_name: null,
  event_id: id,
  sequence: "1",
  received_at: 1,
  message: id,
  ...changes,
});
test("groups explicit child sessions under their scoped visible primary session without losing events", () => {
  const values = [
    event("child-new", { subagent_id: "artisan", parent_session_id: "root" }),
    event("root-new", { sequence: "3", session_name: "Current name" }),
    event("child-old", {
      subagent_id: "artisan",
      parent_session_id: "root",
      sequence: "2",
    }),
    event("root-old", { session_name: "Old name" }),
    event("independent", { session_id: "other" }),
  ];
  const { roots, unmatched } = groupEvents(values);
  expect(unmatched).toEqual([]);
  expect(roots).toHaveLength(2);
  expect(roots[0].name).toBe("Current name");
  expect(roots[0].events.map((x) => x.event.event_id)).toEqual([
    "root-new",
    "root-old",
  ]);
  expect(roots[0].children[0].events.map((x) => x.event.event_id)).toEqual([
    "child-new",
    "child-old",
  ]);
  expect(roots[0].children[0].events[0].index).toBe(0);
  expect(roots[1].events[0].event.event_id).toBe("independent");
});
test("never infers parent links from names, another host/project/tool or an unrelated child", () => {
  const root = event("root", { session_name: "Same name" });
  const cases = [
    { project: "other" },
    { hostname: "other" },
    { tool: "claude" },
    { parent_session_id: "missing" },
    { parent_session_id: null },
  ];
  for (const changes of cases) {
    const child = event("child", {
      subagent_id: "review",
      parent_session_id: "root",
      session_name: "Same name",
      ...changes,
    });
    const result = groupEvents([child, root]);
    expect(result.unmatched).toHaveLength(1);
    expect(result.roots[0].children).toEqual([]);
    expect(result.unmatched[0].key).toBe(sessionKey(child));
  }
  expect(
    groupEvents([
      event("child", { subagent_id: "one", parent_session_id: "root" }),
      event("other-child", { subagent_id: "two", parent_session_id: "root" }),
    ]).unmatched,
  ).toHaveLength(2);
});
test("empty input and duplicate identities keep all reports, ordered by their page positions", () => {
  expect(groupEvents([])).toEqual({ roots: [], unmatched: [] });
  const result = groupEvents([
    event("b", { session_id: "b" }),
    event("a"),
    event("b-old", { session_id: "b", sequence: "2" }),
  ]);
  expect(
    result.roots.map((g) => g.events.map((e) => e.event.event_id)),
  ).toEqual([["b", "b-old"], ["a"]]);
  expect(result.roots[0].name).toBe("Session b");
  expect(
    groupEvents([event("orphan", { subagent_id: "review" })]).unmatched[0].name,
  ).toBe("Subagent review");
});

test("child receipt positions order families and large sequences select names without rounding", () => {
  const child = event("recent-child", {
    subagent_id: "child",
    parent_session_id: "root",
  });
  const other = event("independent", { session_id: "independent" });
  const older = event("older", {
    sequence: "9007199254740992",
    session_name: "Previous name",
  });
  const newer = event("newer", {
    sequence: "9007199254740993",
    session_name: "Latest name",
  });
  const result = groupEvents([child, other, older, newer]);
  expect(result.roots.map((g) => g.name)).toEqual([
    "Latest name",
    "Session independent",
  ]);
  expect(result.roots[0].children[0].events[0].event.event_id).toBe(
    "recent-child",
  );
  expect(result.roots[0].events.map((x) => x.event.event_id)).toEqual([
    "older",
    "newer",
  ]);
});
