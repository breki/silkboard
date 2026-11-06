import { expect, test } from "vitest";
import { act, fromChecklist, serialize, draftText } from "./todo-editor-model";
const row = (key, text, depth = 1, completed = false) => ({
  key,
  text,
  depth,
  completed,
});
const caret = (key, offset) => ({
  anchor: { key, offset },
  head: { key, offset },
});
const point = (result) => result.selection.head;
let n = 0;
const key = () => `new-${++n}`;
test("Enter splits a parent after its subtree and retains children and completion", () => {
  const rows = [
    row("a", "Hello world", 1, true),
    row("b", "Child", 2),
    row("c", "Next"),
  ];
  const result = act(rows, caret("a", 5), "enter", key);
  expect(
    result.rows.map(({ text, depth, completed }) => ({
      text,
      depth,
      completed,
    })),
  ).toEqual([
    { text: "Hello", depth: 1, completed: true },
    { text: "Child", depth: 2, completed: false },
    { text: " world", depth: 1, completed: false },
    { text: "Next", depth: 1, completed: false },
  ]);
  expect(point(result)).toEqual({ key: result.rows[2].key, offset: 0 });
});
test("indent moves the subtree under the preceding sibling and outdent preserves later siblings", () => {
  const rows = [
    row("a", "A"),
    row("b", "B"),
    row("c", "Child", 2),
    row("d", "D"),
  ];
  const indented = act(rows, caret("b", 1), "indent", key);
  expect(indented.rows.map((r) => [r.key, r.depth])).toEqual([
    ["a", 1],
    ["b", 2],
    ["c", 3],
    ["d", 1],
  ]);
  const nested = [
    row("p", "Parent"),
    row("a", "A", 2),
    row("b", "B", 2),
    row("c", "Child", 3),
    row("d", "D", 2),
    row("e", "End"),
  ];
  const out = act(nested, caret("b", 1), "outdent", key);
  expect(out.rows.map((r) => [r.key, r.depth])).toEqual([
    ["p", 1],
    ["a", 2],
    ["d", 2],
    ["b", 1],
    ["c", 2],
    ["e", 1],
  ]);
  expect(point(out)).toEqual({ key: "b", offset: 1 });
});
test("invalid first-item, root outdent and deep-subtree indents keep the document", () => {
  expect(act([row("a", "A")], caret("a", 0), "indent", key).rows).toEqual([
    row("a", "A"),
  ]);
  expect(act([row("a", "A")], caret("a", 0), "outdent", key).hint).toBeTruthy();
  const rows = [
    row("p", "P"),
    row("a", "A"),
    row("b", "B", 2),
    row("c", "C", 3),
    row("d", "D", 4),
    row("e", "E", 5),
  ];
  expect(act(rows, caret("a", 0), "indent", key).rows).toEqual(rows);
});
test("boundary joins promote only removed parents' descendants and retain checkboxes", () => {
  const rows = [
    row("a", "First", 1, true),
    row("b", "Second"),
    row("c", "Child", 2, true),
    row("d", "Grandchild", 3),
  ];
  const result = act(rows, caret("b", 0), "backspace", key);
  expect(result.rows).toEqual([
    row("a", "FirstSecond", 1, true),
    row("c", "Child", 1, true),
    row("d", "Grandchild", 2),
  ]);
  expect(point(result)).toEqual({ key: "a", offset: 5 });
  const forward = act(rows, caret("a", 5), "delete", key);
  expect(forward.rows).toEqual(result.rows);
});
test("deleting an empty parent promotes its children and supplies a useful caret", () => {
  const rows = [row("a", ""), row("b", "Child", 2, true), row("c", "Next")];
  const result = act(rows, caret("a", 0), "delete", key);
  expect(result.rows).toEqual([row("b", "Child", 1, true), row("c", "Next")]);
  expect(point(result)).toEqual({ key: "b", offset: 0 });
});
test("cross-task text replacement preserves unselected descendants and reverse selections", () => {
  const rows = [
    row("a", "Alpha"),
    row("b", "Beta"),
    row("c", "Child", 2, true),
    row("d", "Delta"),
  ];
  const selection = {
    anchor: { key: "b", offset: 2 },
    head: { key: "a", offset: 2 },
  };
  const result = act(rows, selection, { insert: "é" }, key);
  expect(result.rows).toEqual([
    row("a", "Aléta"),
    row("c", "Child", 1, true),
    row("d", "Delta"),
  ]);
  expect(point(result)).toEqual({ key: "a", offset: 3 });
});
test("select-all deletion yields a fresh empty document that serializes to an empty list", () => {
  const rows = [row("a", "A"), row("b", "B", 2)];
  const result = act(
    rows,
    { anchor: { key: "a", offset: 0 }, head: { key: "b", offset: 1 } },
    "delete",
    key,
  );
  expect(result.rows).toHaveLength(1);
  expect(result.rows[0].text).toBe("");
  expect(serialize(result.rows, new Map()).items).toEqual([]);
});
test("invalid text and incomplete entries stay drafts and item limits never truncate", () => {
  expect(serialize([row("a", " ")], new Map([["a", "1"]])).error).toBeTruthy();
  expect(serialize([row("a", "é".repeat(501))], new Map()).error).toBeTruthy();
  const rows = Array.from({ length: 1000 }, (_, n) => row(`r${n}`, "Task"));
  expect(act(rows, caret("r1", 0), "enter", key).rows).toEqual(rows);
});
test("wire conversion retains order, stable server IDs and parent keys", () => {
  const loaded = fromChecklist({
    revision: "3",
    items: [
      { id: "2", parent_id: null, text: "Parent", completed: false },
      { id: "1", parent_id: "2", text: "Child", completed: true },
    ],
  });
  const wire = serialize(loaded.rows, loaded.ids);
  expect(wire.items.map((r) => [r.id, r.parent_key])).toEqual([
    ["2", null],
    ["1", loaded.rows[0].key],
  ]);
  expect(draftText(loaded.rows)).toBe("[ ] Parent\n\t[x] Child");
});
test("multiline paste imports plain checklist text and rejects excessive depth atomically", () => {
  const rows = [row("a", "")];
  const result = act(
    rows,
    caret("a", 0),
    { paste: "[x] Parent\n\t[ ] Child" },
    key,
  );
  expect(result.rows.map((r) => [r.text, r.depth, r.completed])).toEqual([
    ["Parent", 1, true],
    ["Child", 2, false],
  ]);
  const invalid = act(
    rows,
    caret("a", 0),
    { paste: "Parent\n\t\t\t\t\tChild" },
    key,
  );
  expect(invalid.rows).toEqual(rows);
  expect(invalid.hint).toBeTruthy();
});
