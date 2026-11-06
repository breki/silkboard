import { afterEach, expect, test, vi } from "vitest";
import { saveTodoDocument, checklist, loadTodos, taskText } from "./todos-api";
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
test("uses fresh owner CSRF and exact revision for each document mutation", async () => {
  let revision = 0;
  const fetch = vi.fn(async (path) =>
    response(
      path.endsWith("/session")
        ? { authenticated: true, csrf_token: csrf }
        : { revision: String(++revision), items: [row], keys: { task: "1" } },
    ),
  );
  vi.stubGlobal("fetch", fetch);
  const signal = new AbortController().signal;
  await loadTodos(signal);
  revision = 0;
  for (const revision of ["0", "1", "2"])
    await saveTodoDocument(
      revision,
      [
        {
          key: "task",
          id: "1",
          parent_key: null,
          text: "Task",
          completed: false,
        },
      ],
      signal,
    );
  const calls = fetch.mock.calls.filter(([, options]) => options.method);
  expect(
    calls.map(([path, options]) => [
      path,
      options.method,
      JSON.parse(options.body).revision,
    ]),
  ).toEqual([
    ["/api/owner/todos/document", "PUT", "0"],
    ["/api/owner/todos/document", "PUT", "1"],
    ["/api/owner/todos/document", "PUT", "2"],
  ]);
  expect(
    fetch.mock.calls.filter(([path]) => path.endsWith("/session")),
  ).toHaveLength(3);
  expect(
    calls.every(
      ([, options]) =>
        options.headers["X-Silkboard-CSRF"] === csrf &&
        options.credentials === "same-origin",
    ),
  ).toBe(true);
});
