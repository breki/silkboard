import { expect, test } from "vitest";
import {
  tagsIn,
  mentionAt,
  assignProject,
  projectRows,
  tagValidation,
  loadProjectNames,
} from "./todo-projects";
const known = ["hoard", "silkboard"];
const row = (key, text, depth = 1) => ({ key, text, depth, completed: false });
test("only complete existing project mentions are tags, not emails, prefixes or unknown names", () => {
  const text =
    "Email user@silkboard.test (@silkboard), @hoard-extra @unknown @hoard";
  expect(tagsIn(text, known).map((tag) => tag.project)).toEqual([
    "silkboard",
    "hoard",
  ]);
  expect(tagsIn("@silkboard @silkboard", known)).toHaveLength(2);
  expect(tagValidation([row("a", "@silkboard @hoard")], known)).toMatch(
    /one project/i,
  );
  expect(tagValidation([row("a", "@unknown")], known)).toBe("");
});
test("autocomplete replaces the old project and the current query in one edit", () => {
  const text = "Ship @hoard then @sil";
  const mention = mentionAt(text, text.length);
  expect(mention).toEqual({ query: "sil", from: 17, to: 21 });
  const result = assignProject(text, "silkboard", known, mention);
  expect(result.text).toBe("Ship  then @silkboard");
  expect(result.offset).toBe(result.text.length);
  expect(mentionAt("user@sil", 8)).toBeNull();
  expect(mentionAt("@silkboard", 4)).toEqual({ query: "sil", from: 0, to: 10 });
  expect(assignProject("Plain task", "hoard", known).text).toBe(
    "Plain task @hoard",
  );
  expect(assignProject("@hoard Work", "silkboard", known).text).toBe(
    "@silkboard Work",
  );
});
test("project filtering includes ancestor context without inheriting tags or including unrelated children", () => {
  const rows = [
    row("a", "Parent"),
    row("b", "Match @silkboard", 2),
    row("c", "Other", 3),
    row("d", "Sibling @hoard", 2),
    row("e", "Other root"),
  ];
  const result = projectRows(rows, known, "silkboard");
  expect([...result.matching]).toEqual(["b"]);
  expect([...result.visible]).toEqual(["a", "b"]);
  expect(projectRows(rows, known, "").visible.size).toBe(5);
  expect(projectRows(rows, known, "missing").visible.size).toBe(0);
});
test("project autocomplete loads every page and refuses nonadvancing cursors", async () => {
  const calls = [];
  const api = async (after) => {
    calls.push(after);
    return after
      ? { items: [{ id: "silkboard" }], next_cursor: null }
      : { items: [{ id: "hoard" }], next_cursor: "hoard" };
  };
  expect(await loadProjectNames(new AbortController().signal, api)).toEqual(
    known,
  );
  expect(calls).toEqual([null, "hoard"]);
  await expect(
    loadProjectNames(new AbortController().signal, async () => ({
      items: [{ id: "hoard" }],
      next_cursor: "hoard",
    })),
  ).rejects.toThrow();
});
