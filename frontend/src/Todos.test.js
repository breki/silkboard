import { cleanup, render, screen } from "@testing-library/svelte";
import { afterEach, expect, test, vi } from "vitest";
import Todos from "./Todos.svelte";
import { response } from "../test/support";
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
test("shows the persisted checklist with independent labeled checkboxes", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () =>
      response({
        revision: "2",
        items: [
          { id: "1", parent_id: null, text: "Parent", completed: true },
          { id: "2", parent_id: "1", text: "Child", completed: false },
        ],
      }),
    ),
  );
  render(Todos, { onexpire: vi.fn() });
  expect(
    (await screen.findByRole("checkbox", { name: "Parent" })).checked,
  ).toBe(true);
  expect(screen.getByRole("checkbox", { name: "Child" }).checked).toBe(false);
});

test("Todos deep link retains its visible active primary link after owner login", async () => {
  const { default: App } = await import("./App.svelte");
  const { routes, csrf } = await import("../test/support");
  vi.stubGlobal(
    "fetch",
    vi.fn(
      routes({
        "/api/owner/session": response({
          authenticated: true,
          csrf_token: csrf,
        }),
        "/api/owner/todos": response({ revision: "0", items: [] }),
      }),
    ),
  );
  render(App, { todosPage: true });
  await screen.findByText("No todos yet. Add your first todo above.");
  expect(screen.getByRole("link", { name: "Todos" }).getAttribute("href")).toBe(
    "/todos",
  );
  expect(
    screen
      .getByRole("link", { name: "Todos", exact: true })
      .getAttribute("aria-current"),
  ).toBe("page");
  expect(
    screen.queryByRole("navigation", { name: "Main navigation" }),
  ).toBeNull();
});

const root = { id: "1", parent_id: null, text: "Parent", completed: false };
const child = { id: "2", parent_id: "1", text: "Child", completed: false };
async function start(
  items = [root],
  mutate = () => response({ revision: "1", items: [root] }),
) {
  const { csrf } = await import("../test/support");
  let value = { revision: "0", items };
  const fetch = vi.fn(async (path, opts) =>
    path === "/api/owner/session"
      ? response({ authenticated: true, csrf_token: csrf })
      : opts.method
        ? mutate(path, opts)
        : response(value),
  );
  vi.stubGlobal("fetch", fetch);
  const onexpire = vi.fn();
  const view = render(Todos, { onexpire });
  await screen.findByRole("button", { name: "Add todo" });
  return { fetch, onexpire, view, set: (value_) => (value = value_) };
}
test("adds children, edits text and preserves draft while toggling independently", async () => {
  const { fireEvent } = await import("@testing-library/svelte");
  let items = [root];
  let count = 0;
  const { fetch } = await start(items, (_, opts) => {
    const body = JSON.parse(opts.body);
    count++;
    if (opts.method === "POST") items = [root, { ...child, text: body.text }];
    else
      items = items.map((item) =>
        item.id === "1"
          ? { ...item, text: body.text, completed: body.completed }
          : item,
      );
    return response({ revision: String(count), items });
  });
  await fireEvent.click(
    screen.getByRole("button", { name: "Add child to Parent" }),
  );
  expect(document.activeElement).toBe(
    screen.getByLabelText("New child of Parent"),
  );
  await fireEvent.input(screen.getByLabelText("New child of Parent"), {
    target: { value: "Child" },
  });
  await fireEvent.click(screen.getByRole("button", { name: "Add todo" }));
  await screen.findByRole("checkbox", { name: "Child" });
  await fireEvent.click(screen.getByRole("button", { name: "Edit Parent" }));
  await fireEvent.input(screen.getByLabelText("Edit todo text"), {
    target: { value: "Changed" },
  });
  await fireEvent.click(screen.getByRole("checkbox", { name: "Parent" }));
  await screen.findByRole("checkbox", { name: "Parent", checked: true });
  expect(screen.getByRole("checkbox", { name: "Child" }).checked).toBe(false);
  expect(screen.getByLabelText("Edit todo text").value).toBe("Changed");
  await fireEvent.click(screen.getByRole("button", { name: "Save text" }));
  await screen.findByRole("checkbox", { name: "Changed", checked: true });
  expect(screen.queryByLabelText("Edit todo text")).toBeNull();
  expect(
    fetch.mock.calls.filter(([, opts]) => opts.method).at(-1)[1].body,
  ).toContain('"completed":true');
});
test("requires clear parent deletion confirmation and keeps promoted children", async () => {
  const { fireEvent } = await import("@testing-library/svelte");
  await start([root, child], () =>
    response({ revision: "1", items: [{ ...child, parent_id: null }] }),
  );
  await fireEvent.click(screen.getByRole("button", { name: "Delete Parent" }));
  expect(
    screen.getByRole("region", { name: "Confirm todo deletion" }).textContent,
  ).toContain("children will be kept");
  expect(document.activeElement).toBe(
    screen.getByRole("button", { name: "Confirm delete" }),
  );
  await fireEvent.click(screen.getByRole("button", { name: "Cancel delete" }));
  expect(
    screen.queryByRole("region", { name: "Confirm todo deletion" }),
  ).toBeNull();
  await fireEvent.click(screen.getByRole("button", { name: "Delete Parent" }));
  await fireEvent.click(screen.getByRole("button", { name: "Confirm delete" }));
  await screen.findByRole("checkbox", { name: "Child" });
  await vi.waitFor(() =>
    expect(screen.queryByRole("checkbox", { name: "Parent" })).toBeNull(),
  );
  await fireEvent.click(screen.getByRole("button", { name: "Delete Child" }));
  expect(
    screen.getByRole("region", { name: "Confirm todo deletion" }).textContent,
  ).not.toContain("children will be kept");
});
test.each([409, 503])(
  "failed save%s retains drafts and checkbox state until reloaded",
  async (code) => {
    const { fireEvent } = await import("@testing-library/svelte");
    const { set } = await start([root], () => response({}, code));
    await fireEvent.input(screen.getByLabelText("New todo"), {
      target: { value: "Unsent draft" },
    });
    await fireEvent.click(screen.getByRole("checkbox", { name: "Parent" }));
    await screen.findByRole("alert");
    expect(screen.getByRole("checkbox", { name: "Parent" }).checked).toBe(
      false,
    );
    expect(screen.getByRole("checkbox", { name: "Parent" }).disabled).toBe(
      true,
    );
    expect(screen.getByLabelText("New todo").value).toBe("Unsent draft");
    set({ revision: "1", items: [{ ...root, completed: true }] });
    await fireEvent.click(screen.getByRole("button", { name: "Reload todos" }));
    await screen.findByRole("checkbox", { name: "Parent", checked: true });
    expect(screen.getByLabelText("New todo").value).toBe("Unsent draft");
    expect(screen.getByRole("button", { name: "Add todo" }).disabled).toBe(
      false,
    );
  },
);
test("reload reports removed edit targets without losing text", async () => {
  const { fireEvent } = await import("@testing-library/svelte");
  const { set } = await start();
  await fireEvent.click(screen.getByRole("button", { name: "Edit Parent" }));
  await fireEvent.input(screen.getByLabelText("Edit todo text"), {
    target: { value: "Keep my edit" },
  });
  set({ revision: "1", items: [] });
  await fireEvent.click(screen.getByRole("button", { name: "Reload todos" }));
  await screen.findByRole("alert");
  expect(screen.getByLabelText("Edit todo text").value).toBe("Keep my edit");
  expect(screen.getByRole("button", { name: "Save text" }).disabled).toBe(true);
});
test("reload moves a draft to top level when its selected parent is gone", async () => {
  const { fireEvent } = await import("@testing-library/svelte");
  const { set } = await start();
  await fireEvent.click(
    screen.getByRole("button", { name: "Add child to Parent" }),
  );
  await fireEvent.input(screen.getByLabelText("New child of Parent"), {
    target: { value: "Keep child" },
  });
  set({ revision: "1", items: [] });
  await fireEvent.click(screen.getByRole("button", { name: "Reload todos" }));
  await screen.findByRole("alert");
  expect(screen.getByLabelText("New todo").value).toBe("Keep child");
});
test("five levels remain nested with no add-child action on the deepest item", async () => {
  await start(
    Array.from({ length: 5 }, (_, n) => ({
      ...root,
      id: String(n + 1),
      parent_id: n === 0 ? null : String(n),
      text: `Level ${n + 1}`,
    })),
  );
  expect(
    screen.queryByRole("button", { name: "Add child to Level 5" }),
  ).toBeNull();
  expect(screen.getAllByRole("list")).toHaveLength(5);
});
test("expired reads clear owner content through the supplied handler", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response({}, 401)),
  );
  const expired = vi.fn();
  render(Todos, { onexpire: expired });
  await vi.waitFor(() => expect(expired).toHaveBeenCalledOnce());
});

test("unavailable initial load offers reload without inventing a checklist", async () => {
  const { fireEvent } = await import("@testing-library/svelte");
  let fail = true;
  vi.stubGlobal(
    "fetch",
    vi.fn(async () =>
      fail ? response({}, 503) : response({ revision: "0", items: [] }),
    ),
  );
  render(Todos, { onexpire: vi.fn() });
  await screen.findByRole("alert");
  expect(screen.queryByRole("button", { name: "Add todo" })).toBeNull();
  fail = false;
  await fireEvent.click(screen.getByRole("button", { name: "Reload todos" }));
  await screen.findByText("No todos yet. Add your first todo above.");
});
test("removed edit target never overwrites a separate add draft", async () => {
  const { fireEvent } = await import("@testing-library/svelte");
  const { set } = await start();
  await fireEvent.input(screen.getByLabelText("New todo"), {
    target: { value: "Separate draft" },
  });
  await fireEvent.click(screen.getByRole("button", { name: "Edit Parent" }));
  await fireEvent.input(screen.getByLabelText("Edit todo text"), {
    target: { value: "Edit draft" },
  });
  set({ revision: "1", items: [] });
  await fireEvent.click(screen.getByRole("button", { name: "Reload todos" }));
  await screen.findByRole("alert");
  expect(screen.getByLabelText("New todo").value).toBe("Separate draft");
  expect(screen.getByLabelText("Edit todo text").value).toBe("Edit draft");
});
test.each([false, true])(
  "unmount ignores a late load body or error (%s)",
  async (fail) => {
    let done;
    const body = new Promise(
      (resolve, reject) => (done = fail ? reject : resolve),
    );
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({ ok: true, status: 200, json: () => body })),
    );
    const onexpire = vi.fn();
    const view = render(Todos, { onexpire });
    await screen.findByText("Loading todos…");
    await new Promise((done) => setTimeout(done, 0));
    view.unmount();
    done(fail ? new Error("Late failure") : { revision: "0", items: [root] });
    await new Promise((done) => setTimeout(done, 0));
    expect(screen.queryByRole("checkbox")).toBeNull();
    expect(onexpire).not.toHaveBeenCalled();
  },
);
test("cancelling local forms does not mutate the checklist", async () => {
  const { fireEvent } = await import("@testing-library/svelte");
  const { fetch } = await start();
  await fireEvent.click(
    screen.getByRole("button", { name: "Add child to Parent" }),
  );
  await fireEvent.click(screen.getByRole("button", { name: "Use top level" }));
  expect(screen.getByLabelText("New todo")).toBeTruthy();
  await fireEvent.click(screen.getByRole("button", { name: "Edit Parent" }));
  await fireEvent.click(screen.getByRole("button", { name: "Cancel edit" }));
  expect(screen.queryByLabelText("Edit todo text")).toBeNull();
  expect(fetch.mock.calls.filter(([, opts]) => opts.method)).toHaveLength(0);
});

test("oversized edit text explains why Save text is disabled", async () => {
  const { fireEvent } = await import("@testing-library/svelte");
  await start();
  await fireEvent.click(screen.getByRole("button", { name: "Edit Parent" }));
  await fireEvent.input(screen.getByLabelText("Edit todo text"), {
    target: { value: "é".repeat(501) },
  });
  expect(screen.getByRole("button", { name: "Save text" }).disabled).toBe(true);
  expect(
    screen.getByText(
      "Use a non-empty single line of at most 1,000 UTF-8 bytes.",
    ),
  ).toBeTruthy();
});

test.each(["edit", "child"])(
  "a late %s save cannot clear a newer linked selection",
  async (mode) => {
    const { fireEvent } = await import("@testing-library/svelte");
    const { updateQuery } = await import("./link-state");
    let finish;
    const other = {
      id: "3",
      parent_id: null,
      text: "Other todo",
      completed: false,
    };
    await start([root, other], () => new Promise((done) => (finish = done)));
    if (mode === "edit") {
      await fireEvent.click(
        screen.getByRole("button", { name: "Edit Parent" }),
      );
      await fireEvent.input(screen.getByLabelText("Edit todo text"), {
        target: { value: "Changed parent" },
      });
      await fireEvent.click(screen.getByRole("button", { name: "Save text" }));
    } else {
      await fireEvent.click(
        screen.getByRole("button", { name: "Add child to Parent" }),
      );
      await fireEvent.input(screen.getByLabelText("New child of Parent"), {
        target: { value: "Pending child" },
      });
      await fireEvent.click(screen.getByRole("button", { name: "Add todo" }));
    }
    await vi.waitFor(() => expect(finish).toBeTypeOf("function"));
    updateQuery({ todo: "3", mode });
    const label =
      mode === "edit" ? "Edit todo text" : "New child of Other todo";
    await screen.findByLabelText(label);
    finish(response({ revision: "1", items: [root, other] }));
    await vi.waitFor(() =>
      expect(screen.queryByText("Saving or loading todos…")).toBeNull(),
    );
    expect(new URLSearchParams(window.location.search).get("todo")).toBe("3");
    expect(screen.getByLabelText(label).value).toBe(
      mode === "edit" ? "Other todo" : "Pending child",
    );
  },
);
