import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import Todos from "./Todos.svelte";
import { response, csrf } from "../test/support";
import { clearRetainedDraft } from "./todo-document-session";
const initial = [
  { id: "1", parent_id: null, text: "Parent", completed: false },
  { id: "2", parent_id: "1", text: "Child", completed: false },
];
beforeEach(() => {
  window.history.replaceState(null, "", "/todos");
  window.dispatchEvent(new Event("popstate"));
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
  };
  Range.prototype.getClientRects = () => [];
  Range.prototype.getBoundingClientRect = () => new DOMRect();
  vi.stubGlobal("requestAnimationFrame", (fn) => setTimeout(() => fn(0), 0));
  vi.spyOn(window, "scrollBy").mockImplementation(() => {});
  vi.stubGlobal("matchMedia", () => ({ matches: false }));
});
afterEach(() => {
  cleanup();
  clearRetainedDraft();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});
async function start({
  items = initial,
  save,
  load,
  onregister = () => {},
  projectLoad,
} = {}) {
  let data = { revision: "0", items };
  let next = 3;
  const fetch = vi.fn(async (path, options = {}) => {
    if (path === "/api/owner/projects")
      return projectLoad
        ? projectLoad()
        : response({
            items: [{ id: "hoard" }, { id: "silkboard" }],
            next_cursor: null,
          });
    if (path.endsWith("/session"))
      return response({ authenticated: true, csrf_token: csrf });
    if (options.method === "PUT") {
      if (save) return save(path, options);
      const input = JSON.parse(options.body),
        keys = Object.fromEntries(
          input.items.map((row) => [row.key, row.id ?? String(next++)]),
        );
      data = {
        revision: String(Number(data.revision) + 1),
        items: input.items.map((row) => ({
          id: keys[row.key],
          parent_id: row.parent_key === null ? null : keys[row.parent_key],
          text: row.text,
          completed: row.completed,
        })),
      };
      return response({ ...data, keys });
    }
    return load ? load() : response(data);
  });
  vi.stubGlobal("fetch", fetch);
  const expire = vi.fn();
  render(Todos, { onexpire: expire, onregister });
  await screen.findByRole("textbox", { name: "Todo document" });
  return { fetch, expire };
}
test("shows one task document with independent labels and no per-item edit forms", async () => {
  await start();
  expect(screen.getAllByRole("textbox")).toHaveLength(1);
  expect(screen.getByRole("checkbox", { name: "Parent" }).checked).toBe(false);
  expect(screen.queryByRole("button", { name: "Edit Parent" })).toBeNull();
  expect(screen.queryByRole("button", { name: "Delete Parent" })).toBeNull();
  expect(screen.getByText("All changes saved")).toBeTruthy();
});
async function editText(before, after) {
  const editor = selectText(before);
  const element = [...document.querySelectorAll(".todo-task-text")].find(
    (node) => node.textContent === before,
  );
  element.textContent = after;
  await fireEvent.input(editor, { inputType: "insertText" });
  await screen.findByRole("checkbox", {
    name: after.replace(/\s+/g, " ").trim(),
    exact: true,
  });
  selectText(after);
  await fireEvent.keyDown(editor, { key: "Shift" });
  return editor;
}
test("inline project autocomplete replaces the existing tag, supports keyboard choice and saves the full document", async () => {
  const { fetch } = await start({
    items: [{ ...initial[0], text: "Parent @hoard" }, initial[1]],
  });
  await screen.findByRole("option", { name: "silkboard", exact: true });
  const editor = await editText("Parent @hoard", "Parent @hoard @");
  expect(
    screen.getByRole("listbox", { name: "Project suggestions" }),
  ).toBeTruthy();
  await fireEvent.keyDown(editor, { key: "ArrowDown" });
  expect(
    screen
      .getByRole("option", { name: "@silkboard" })
      .getAttribute("aria-selected"),
  ).toBe("true");
  await fireEvent.keyDown(editor, { key: "Tab" });
  await screen.findByRole("checkbox", { name: /Parent\s+@silkboard/ });
  expect(
    screen.queryByRole("listbox", { name: "Project suggestions" }),
  ).toBeNull();
  await waitFor(() =>
    expect(
      fetch.mock.calls.some(([, options]) => options.method === "PUT"),
    ).toBe(true),
  );
  const sent = JSON.parse(
    fetch.mock.calls.find(([, options]) => options.method === "PUT")[1].body,
  );
  expect(sent.items).toHaveLength(2);
  expect(sent.items[0].text).toBe("Parent  @silkboard");
  expect(sent.items[1].text).toBe("Child");
});
test("Enter after an existing project tag still creates a new task", async () => {
  await start({
    items: [{ ...initial[0], text: "Parent @silkboard" }, initial[1]],
  });
  await screen.findByRole("option", { name: "silkboard", exact: true });
  const editor = selectText("Parent @silkboard");
  await fireEvent.keyDown(editor, { key: "Enter" });
  await screen.findByRole("checkbox", { name: "Empty todo" });
});
test("autocomplete can be dismissed, chosen by pointer, and rejects multiple known project tags", async () => {
  await start();
  await screen.findByRole("option", { name: "hoard", exact: true });
  const editor = await editText("Parent", "Parent @");
  await fireEvent.keyDown(editor, { key: "Escape" });
  expect(
    screen.queryByRole("listbox", { name: "Project suggestions" }),
  ).toBeNull();
  expect(document.activeElement).toBe(editor);
  await editText("Parent @", "Parent @si");
  await fireEvent.pointerDown(
    screen.getByRole("option", { name: "@silkboard" }),
  );
  await fireEvent.click(screen.getByRole("option", { name: "@silkboard" }));
  await screen.findByRole("checkbox", { name: "Parent @silkboard" });
  await editText("Parent @silkboard", "Parent @silkboard @hoard ");
  await screen.findByText(/Use one project tag per item/);
});
test("project filter shows parent context, protects structure and saves hidden items unchanged", async () => {
  const hidden = {
    id: "3",
    parent_id: null,
    text: "Hidden @hoard",
    completed: true,
  };
  const { fetch } = await start({
    items: [initial[0], { ...initial[1], text: "Child @silkboard" }, hidden],
  });
  const filter = screen.getByRole("combobox", {
    name: "Filter todos by project",
  });
  await screen.findByRole("option", { name: "silkboard", exact: true });
  await fireEvent.change(filter, { target: { value: "silkboard" } });
  expect(window.location.search).toContain("project=silkboard");
  await waitFor(() =>
    expect(screen.getByRole("checkbox", { name: "Parent" }).disabled).toBe(
      true,
    ),
  );
  expect(button("New task").disabled).toBe(true);
  await fireEvent.click(
    screen.getByRole("checkbox", { name: "Child @silkboard" }),
  );
  await waitFor(() =>
    expect(
      fetch.mock.calls.some(([, options]) => options.method === "PUT"),
    ).toBe(true),
  );
  const sent = JSON.parse(
    fetch.mock.calls.find(([, options]) => options.method === "PUT")[1].body,
  );
  expect(sent.items).toHaveLength(3);
  expect(sent.items[2]).toMatchObject({
    id: "3",
    text: hidden.text,
    completed: true,
  });
  const editor = selectText("Child @silkboard");
  await fireEvent.keyDown(editor, { key: "Enter" });
  await screen.findByText(/Show the full list to add, delete, move items/);
  await fireEvent.click(button("Show full list"));
  expect(
    screen.queryByText(/Show the full list to add, delete, move items/),
  ).toBeNull();
  expect(window.location.search).not.toContain("project=");
  expect(screen.getByRole("checkbox", { name: "Parent" }).disabled).toBe(false);
});
test("project loading failure retains the task document and has an explicit retry", async () => {
  let failed = true;
  await start({
    projectLoad: () =>
      failed
        ? response({}, 503)
        : response({ items: [{ id: "silkboard" }], next_cursor: null }),
  });
  await screen.findByText(/Unable to load projects/);
  expect(screen.getByRole("checkbox", { name: "Parent" })).toBeTruthy();
  failed = false;
  await fireEvent.click(button("Retry projects"));
  await screen.findByRole("option", { name: "silkboard", exact: true });
});
test("autocomplete supports upward selection and Enter, while completion modifiers keep their meaning", async () => {
  await start();
  await screen.findByRole("option", { name: "silkboard", exact: true });
  const editor = await editText("Parent", "Parent @");
  await fireEvent.keyDown(editor, { key: "ArrowUp" });
  expect(
    screen
      .getByRole("option", { name: "@silkboard" })
      .getAttribute("aria-selected"),
  ).toBe("true");
  await fireEvent.keyDown(editor, { key: "Enter" });
  await screen.findByRole("checkbox", { name: "Parent @silkboard" });
  await editText("Parent @silkboard", "Parent @silkboard @si");
  await fireEvent.keyDown(editor, { key: "Enter", ctrlKey: true });
  expect(
    screen.getByRole("checkbox", { name: "Parent @silkboard @si" }).checked,
  ).toBe(true);
  await fireEvent.pointerDown(document.body);
  expect(
    screen.queryByRole("listbox", { name: "Project suggestions" }),
  ).toBeNull();
  await editText("Parent @silkboard @si", "Parent @unknown");
  await fireEvent.keyDown(editor, { key: "Escape" });
  expect(document.activeElement).toBe(editor);
});
test("unknown project bookmarks show an empty result and can be cleared without changing tasks", async () => {
  window.history.replaceState(null, "", "/todos?project=missing");
  window.dispatchEvent(new Event("popstate"));
  const { fetch } = await start();
  await screen.findByText(/No items currently match this project/);
  expect(screen.getByRole("option", { name: "missing" })).toBeTruthy();
  await fireEvent.change(
    screen.getByRole("combobox", { name: "Filter todos by project" }),
    { target: { value: "" } },
  );
  expect(window.location.search).not.toContain("project=");
  expect(screen.getByRole("checkbox", { name: "Parent" }).disabled).toBe(false);
  expect(fetch.mock.calls.some(([, options]) => options.method === "PUT")).toBe(
    false,
  );
});
test("a filtered child bookmark cannot add an item until the full list is shown", async () => {
  window.history.replaceState(
    null,
    "",
    "/todos?project=silkboard&todo=1&mode=child",
  );
  window.dispatchEvent(new Event("popstate"));
  await start({
    items: [{ ...initial[0], text: "Parent @silkboard" }, initial[1]],
  });
  await screen.findByText(/linked todo is unavailable/);
  expect(screen.queryByRole("checkbox", { name: "Empty todo" })).toBeNull();
  await fireEvent.click(button("Show full list"));
  await screen.findByRole("checkbox", { name: "Empty todo" });
});
test("checkbox edits save without replacing the editor and leave descendants independent", async () => {
  const { fetch } = await start();
  const editor = screen.getByRole("textbox", { name: "Todo document" });
  await fireEvent.click(screen.getByRole("checkbox", { name: "Parent" }));
  expect(screen.getByRole("checkbox", { name: "Child" }).checked).toBe(false);
  await waitFor(() =>
    expect(fetch.mock.calls.some(([, o]) => o.method === "PUT")).toBe(true),
  );
  await waitFor(() =>
    expect(screen.getByText("All changes saved")).toBeTruthy(),
  );
  expect(screen.getByRole("textbox", { name: "Todo document" })).toBe(editor);
  expect(
    screen.getByRole("button", { name: "Undo", exact: true }).disabled,
  ).toBe(false);
});
test("provides keyboard help and leaves document focus on Escape", async () => {
  await start();
  await fireEvent.click(
    screen.getByRole("button", { name: "Keyboard help", exact: true }),
  );
  expect(
    screen.getByRole("dialog", { name: "Writing your list" }),
  ).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("button", { name: "Close keyboard help" }),
  );
  const editor = screen.getByRole("textbox", { name: "Todo document" });
  editor.focus();
  await fireEvent.keyDown(editor, { key: "Escape" });
  expect(document.activeElement).not.toBe(editor);
});
test("keeps the draft visible when saving conflicts and exposes explicit recovery", async () => {
  await start({ save: () => response({}, 409) });
  await fireEvent.click(screen.getByRole("checkbox", { name: "Parent" }));
  await screen.findByRole("alert");
  expect(screen.getByRole("checkbox", { name: "Parent" }).checked).toBe(true);
  expect(
    within(screen.getByRole("alert")).getByRole("button", {
      name: "Review saved list",
    }),
  ).toBeTruthy();
  expect(screen.getByRole("button", { name: "Copy my draft" })).toBeTruthy();
});
test("empty lists start with a reachable document and a shared new-task action", async () => {
  await start({ items: [] });
  expect(screen.getByRole("textbox", { name: "Todo document" })).toBeTruthy();
  expect(screen.getByText(/^0 tasks/)).toBeTruthy();
  expect(
    screen.getByRole("button", { name: "New task", exact: true }),
  ).toBeTruthy();
});
const button = (name) => screen.getByRole("button", { name, exact: true });
function selectText(value) {
  const element = [...document.querySelectorAll(".todo-task-text")].find(
    (n) => n.textContent === value,
  );
  const editor = screen.getByRole("textbox", { name: "Todo document" });
  editor.focus();
  const range = document.createRange();
  range.selectNodeContents(element);
  range.collapse(false);
  const selection = getSelection();
  selection.removeAllRanges();
  selection.addRange(range);
  document.dispatchEvent(new Event("selectionchange"));
  return editor;
}
test("shared tools edit nesting and history with keyboard toolbar navigation", async () => {
  await start();
  const editor = selectText("Child");
  await waitFor(() => expect(window.location.search).toContain("todo=2"));
  await fireEvent.click(button("Outdent"));
  expect(document.querySelectorAll(".todo-task")[1].dataset.depth).toBe("1");
  await fireEvent.click(button("Indent"));
  expect(document.querySelectorAll(".todo-task")[1].dataset.depth).toBe("2");
  await fireEvent.click(button("Undo"));
  await fireEvent.click(button("Redo"));
  await fireEvent.click(button("New task"));
  expect(screen.getByRole("checkbox", { name: "Empty todo" })).toBeTruthy();
  await fireEvent.keyDown(editor, { key: "Escape" });
  const help = button("Help");
  help.focus();
  await fireEvent.keyDown(help, { key: "Home" });
  expect(document.activeElement).toBe(button("New task"));
  await fireEvent.keyDown(button("New task"), { key: "ArrowRight" });
  expect(document.activeElement).toBe(button("Outdent"));
  await fireEvent.keyDown(button("Outdent"), { key: "ArrowLeft" });
  expect(document.activeElement).toBe(button("New task"));
  await fireEvent.keyDown(button("New task"), { key: "End" });
  expect(document.activeElement).toBe(help);
  await fireEvent.keyDown(help, { key: "a" });
  const toolbar = screen.getByRole("toolbar");
  expect(await fireEvent.pointerDown(button("New task"))).toBe(false);
  await fireEvent.pointerDown(toolbar);
  await fireEvent.click(help);
  expect(screen.getByRole("dialog")).toBeTruthy();
  await fireEvent.click(button("Close keyboard help"));
});
test("review, copy, retry and confirmed replacement preserve local work until requested", async () => {
  let remote = false;
  const clipboard = { writeText: vi.fn().mockResolvedValue() };
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: clipboard,
  });
  await start({
    save: () => response({}, 503),
    load: () =>
      response({
        revision: remote ? "2" : "0",
        items: remote ? [{ ...initial[0], text: "Remote" }] : initial,
      }),
  });
  await fireEvent.click(screen.getByRole("checkbox", { name: "Parent" }));
  await screen.findByRole("alert");
  await fireEvent.click(button("Copy my draft"));
  expect(clipboard.writeText).toHaveBeenCalledWith("[x] Parent\n\t[ ] Child");
  await screen.findByText("Draft copied.");
  clipboard.writeText.mockRejectedValue(new Error("blocked"));
  await fireEvent.click(button("Copy my draft"));
  expect(
    (await screen.findByRole("textbox", { name: "Copy your draft" })).value,
  ).toContain("Parent");
  await fireEvent.click(
    within(screen.getByRole("alert")).getByRole("button", {
      name: "Review saved list",
    }),
  );
  await screen.findByRole("button", { name: "Retry save" });
  await fireEvent.click(button("Retry save"));
  await screen.findByText(/Couldn't confirm/);
  remote = true;
  await fireEvent.click(
    within(screen.getByRole("alert")).getByRole("button", {
      name: "Review saved list",
    }),
  );
  await screen.findByRole("textbox", { name: "Latest saved list" });
  const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
  await fireEvent.click(button("Use saved list"));
  expect(screen.getByRole("checkbox", { name: "Parent" }).checked).toBe(true);
  confirm.mockReturnValue(true);
  await fireEvent.click(button("Use saved list"));
  await screen.findByRole("checkbox", { name: "Remote" });
  expect(screen.queryByRole("textbox", { name: "Your draft" })).toBeNull();
});
test("copy selected links and menu review never include draft text", async () => {
  const clipboard = { writeText: vi.fn().mockResolvedValue() };
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: clipboard,
  });
  await start();
  selectText("Parent");
  await waitFor(() =>
    expect(button("Copy selected task link").disabled).toBe(false),
  );
  await fireEvent.click(button("Copy selected task link"));
  await screen.findByText("Task link copied.");
  expect(clipboard.writeText.mock.calls[0][0]).toMatch(/\/todos\?todo=1$/);
  clipboard.writeText.mockRejectedValue(new Error("blocked"));
  await fireEvent.click(button("Copy selected task link"));
  await screen.findByText(/address bar/);
  await fireEvent.click(button("Review saved list"));
});
test("pending drafts warn on unload and only explicit sign-out discard removes retained memory", async () => {
  let guard;
  const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
  await start({
    onregister: (g) => {
      guard = g;
    },
  });
  expect(guard.confirm()).toBe(true);
  const clean = new Event("beforeunload", { cancelable: true });
  window.dispatchEvent(clean);
  expect(clean.defaultPrevented).toBe(false);
  await fireEvent.click(screen.getByRole("checkbox", { name: "Parent" }));
  expect(guard.confirm()).toBe(false);
  expect(confirm).toHaveBeenCalledOnce();
  const dirty = new Event("beforeunload", { cancelable: true });
  window.dispatchEvent(dirty);
  expect(dirty.defaultPrevented).toBe(true);
  guard.keep();
  guard.discard();
});
test.each([401, 503])(
  "load failures (%s) expire or offer a working retry",
  async (status) => {
    let code = status;
    const expire = vi.fn();
    vi.stubGlobal(
      "fetch",
      vi.fn(async () =>
        response(code === 200 ? { revision: "0", items: initial } : {}, code),
      ),
    );
    render(Todos, { onexpire: expire });
    if (status === 401) {
      await waitFor(() => expect(expire).toHaveBeenCalledOnce());
      return;
    }
    await screen.findByText("Unable to load Todos. Try again.");
    code = 200;
    await fireEvent.click(button("Try again"));
    await screen.findByRole("checkbox", { name: "Parent" });
  },
);
test("missing bookmarked targets can be cleared and phone Done leaves editing", async () => {
  await start();
  window.history.replaceState(null, "", "/todos?todo=999");
  window.dispatchEvent(new Event("popstate"));
  await screen.findByText(/linked todo is unavailable/);
  await fireEvent.click(button("Clear todo selection"));
  expect(window.location.search).toBe("");
  await fireEvent.click(button("Edit list"));
  await screen.findByRole("button", { name: "Done" });
  await fireEvent.click(button("Done"));
  expect(document.activeElement).toBe(button("Help"));
});
test("phone viewport changes keep the caret above the shared tools", async () => {
  const viewport = new EventTarget();
  Object.assign(viewport, { height: 400, offsetTop: 20 });
  Object.defineProperty(window, "visualViewport", {
    configurable: true,
    value: viewport,
  });
  const scroll = vi.spyOn(window, "scrollBy");
  vi.stubGlobal(
    "matchMedia",
    vi.fn(() => ({ matches: true })),
  );
  Range.prototype.getBoundingClientRect = () => ({
    top: 590,
    bottom: 610,
    left: 10,
    right: 10,
    width: 0,
    height: 20,
  });
  const rect = vi
    .spyOn(Element.prototype, "getBoundingClientRect")
    .mockReturnValue({
      top: 500,
      bottom: 540,
      left: 0,
      right: 320,
      width: 320,
      height: 40,
    });
  await start();
  selectText("Parent");
  viewport.dispatchEvent(new Event("resize"));
  await waitFor(() =>
    expect(
      document
        .querySelector(".todos-editor")
        .style.getPropertyValue("--keyboard-inset"),
    ).toBe(`${Math.max(0, innerHeight - 420)}px`),
  );
  await waitFor(() => expect(scroll).toHaveBeenCalled());
  viewport.dispatchEvent(new Event("scroll"));
  rect.mockRestore();
  cleanup();
  Object.defineProperty(window, "visualViewport", {
    configurable: true,
    value: undefined,
  });
});
test("an earlier retained draft stays available when the recovery load fails", async () => {
  const { TodoDocumentSession } = await import("./todo-document-session");
  const session = new TodoDocumentSession(
    { revision: "0", items: initial },
    { change: () => {}, reset: () => {}, expire: () => {} },
  );
  session.update(
    session.rows.map((r, i) => (i ? r : { ...r, text: "Earlier draft" })),
  );
  session.keep();
  session.dispose();
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response({}, 503)),
  );
  render(Todos, { onexpire: vi.fn() });
  await screen.findByText(/earlier draft is kept/);
});
test("deep child links respect five levels without writing invalid drafts", async () => {
  const items = Array.from({ length: 5 }, (_, i) => ({
    id: String(i + 1),
    parent_id: i ? String(i) : null,
    text: `Depth ${i + 1}`,
    completed: false,
  }));
  const { fetch } = await start({ items });
  window.history.replaceState(null, "", "/todos?todo=5&mode=child");
  window.dispatchEvent(new Event("popstate"));
  await screen.findByText(/linked todo is unavailable/);
  expect(
    fetch.mock.calls.every(([, o]) => !o.method || o.method === "GET"),
  ).toBe(true);
});
