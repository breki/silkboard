import { afterEach, expect, test, vi } from "vitest";
import { changeTodo, checklist, loadTodos, taskText } from "./todos-api";
import { csrf, response } from "../test/support";
const row = { id: "1", parent_id: null, text: "Task", completed: false };
afterEach(() => vi.unstubAllGlobals());
test.each([
  null,
  {},
  { revision: "00", items: [] },
  { revision: "0", items: Array(1001).fill(row) },
  { revision: "1", items: [{ ...row, id: "0" }] },
  { revision: "1", items: [{ ...row, parent_id: "missing" }] },
  { revision: "1", items: [row, row] },
  { revision: "1", items: [{ ...row, completed: 1 }] },
  { revision: "1", items: [{ ...row, text: "" }] },
])("rejects malformed checklist %j", (value) =>
  expect(() => checklist(value)).toThrow(),
);
test("validates hierarchy, safe integers and Unicode byte bounds", () => {
  const items = Array.from({ length: 6 }, (_, n) => ({
    ...row,
    id: String(n + 1),
    parent_id: n === 0 ? null : String(n),
  }));
  expect(() => checklist({ revision: "1", items })).toThrow();
  expect(checklist({ revision: "0", items: [] })).toEqual({
    revision: "0",
    items: [],
  });
  expect(
    checklist({
      revision: "9223372036854775807",
      items: [{ ...row, id: "9223372036854775807" }],
    }).items,
  ).toHaveLength(1);
  expect(taskText("é".repeat(500))).toBe(true);
  for (const text of ["é".repeat(501), " ", "x\ny", "x\u0085y"])
    expect(taskText(text)).toBe(false);
});
test("uses fresh owner CSRF and exact revision for each mutation", async () => {
  const fetch = vi.fn(async (path) =>
    response(
      path.endsWith("/session")
        ? { authenticated: true, csrf_token: csrf }
        : { revision: "1", items: [row] },
    ),
  );
  vi.stubGlobal("fetch", fetch);
  const signal = new AbortController().signal;
  await loadTodos(signal);
  for (const change of [
    { kind: "add", text: "Task", parent_id: null },
    { kind: "edit", id: "1", text: "Changed", completed: true },
    { kind: "delete", id: "1" },
  ])
    await changeTodo("0", change, signal);
  const calls = fetch.mock.calls.filter(([, opts]) => opts.method);
  expect(
    calls.map(([path, opts]) => [path, opts.method, JSON.parse(opts.body)]),
  ).toEqual([
    [
      "/api/owner/todos",
      "POST",
      { revision: "0", parent_id: null, text: "Task" },
    ],
    [
      "/api/owner/todos/1",
      "PATCH",
      { revision: "0", text: "Changed", completed: true },
    ],
    ["/api/owner/todos/1", "DELETE", { revision: "0" }],
  ]);
  expect(
    calls.every(
      ([, opts]) =>
        opts.headers["X-Silkboard-CSRF"] === csrf &&
        opts.credentials === "same-origin",
    ),
  ).toBe(true);
});
