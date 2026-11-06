import { afterEach, expect, test, vi } from "vitest";
import * as api from "./todos-api";
import { response, csrf } from "../test/support";
afterEach(() => vi.unstubAllGlobals());
const items = [
  {
    key: "parent",
    id: "2",
    parent_key: null,
    text: " Second ",
    completed: false,
  },
  {
    key: "child",
    id: "1",
    parent_key: "parent",
    text: "Child",
    completed: true,
  },
];
const saved = {
  revision: "4",
  items: [
    { id: "2", parent_id: null, text: " Second ", completed: false },
    { id: "1", parent_id: "2", text: "Child", completed: true },
  ],
  keys: { parent: "2", child: "1" },
};
test("admits a preorder checklist after moving an older item beneath a newer one", () => {
  expect(api.checklist(saved).items).toEqual(saved.items);
});
test("saves the whole ordered document with fresh owner integrity and exact key acknowledgement", async () => {
  expect(api.saveTodoDocument).toBeTypeOf("function");
  const fetch = vi.fn(async (path) =>
    response(
      path.endsWith("/session")
        ? { authenticated: true, csrf_token: csrf }
        : saved,
    ),
  );
  vi.stubGlobal("fetch", fetch);
  expect(
    await api.saveTodoDocument("3", items, new AbortController().signal),
  ).toEqual(saved);
  const [path, options] = fetch.mock.calls.at(-1);
  expect(path).toBe("/api/owner/todos/document");
  expect(options.method).toBe("PUT");
  expect(options.headers["X-Silkboard-CSRF"]).toBe(csrf);
  expect(JSON.parse(options.body)).toEqual({ revision: "3", items });
  expect(options.credentials).toBe("same-origin");
});
test.each([
  { ...saved, keys: {} },
  { ...saved, keys: { parent: "1", child: "2" } },
  { ...saved, keys: { ...saved.keys, extra: "3" } },
  {
    ...saved,
    items: saved.items.map((item, n) =>
      n === 0 ? { ...item, text: "Unrelated" } : item,
    ),
  },
  {
    ...saved,
    items: saved.items.map((item, n) =>
      n === 1 ? { ...item, completed: false } : item,
    ),
  },
  {
    ...saved,
    items: saved.items.map((item, n) =>
      n === 1 ? { ...item, parent_id: null } : item,
    ),
  },
])(
  "rejects an acknowledgement that cannot identify the exact submitted draft",
  async (value) => {
    expect(api.saveTodoDocument).toBeTypeOf("function");
    vi.stubGlobal(
      "fetch",
      vi.fn(async (path) =>
        response(
          path.endsWith("/session")
            ? { authenticated: true, csrf_token: csrf }
            : value,
        ),
      ),
    );
    await expect(
      api.saveTodoDocument("3", items, new AbortController().signal),
    ).rejects.toThrow();
  },
);
