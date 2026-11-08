import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { TextSelection, AllSelection } from "prosemirror-state";
import { waitFor } from "@testing-library/svelte";
import { createTodoEditor } from "./todo-editor-view";
const a = { key: "a", text: "Alpha", depth: 1, completed: false },
  b = { key: "b", text: "Beta", depth: 2, completed: true };
const instances = [];
beforeEach(() => {
  Range.prototype.getClientRects = () => [];
  Range.prototype.getBoundingClientRect = () => new DOMRect();
  vi.stubGlobal("requestAnimationFrame", (fn) => {
    fn(0);
    return 0;
  });
  vi.spyOn(window, "scrollBy").mockImplementation(() => {});
});
afterEach(() => {
  for (const { editor, host } of instances) {
    if (!editor.view.isDestroyed) editor.destroy();
    host.remove();
  }
  instances.length = 0;
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});
function start(rows = [a, b]) {
  const host = document.createElement("div");
  document.body.append(host);
  const callbacks = {
    change: vi.fn(),
    selection: vi.fn(),
    history: vi.fn(),
    hint: vi.fn(),
    leave: vi.fn(),
    focus: vi.fn(),
    composing: vi.fn(),
    reveal: vi.fn(),
  };
  const editor = createTodoEditor(host, rows, callbacks);
  instances.push({ editor, host });
  return { editor, host, callbacks };
}
function key(editor, name, fields = {}) {
  const event = new KeyboardEvent("keydown", {
    key: name,
    bubbles: true,
    cancelable: true,
    ...fields,
  });
  editor.view.dom.dispatchEvent(event);
  return event;
}
function input(editor, inputType) {
  const event = new InputEvent("beforeinput", {
    inputType,
    bubbles: true,
    cancelable: true,
  });
  editor.view.dom.dispatchEvent(event);
  return event;
}
test("completion and removal shortcuts retain children, caret and undo history", () => {
  const { editor } = start();
  editor.focus("a", 2);
  key(editor, "Enter", { ctrlKey: true });
  expect(editor.rows()).toEqual([{ ...a, completed: true }, b]);
  key(editor, "Backspace", { ctrlKey: true, shiftKey: true });
  expect(editor.rows()).toEqual([{ ...b, depth: 1 }]);
  expect(editor.selection().head.key).toBe("b");
  editor.undo();
  expect(editor.rows()).toEqual([{ ...a, completed: true }, b]);
  editor.undo();
  expect(editor.rows()).toEqual([a, b]);
});
test("filtered editing retains hidden rows and blocks structure, parent edits and hidden-history changes", () => {
  const rows = [
    a,
    { ...b, text: "Beta @silkboard" },
    { key: "c", depth: 1, text: "Hidden @hoard", completed: false },
  ];
  const { editor, host, callbacks } = start(rows);
  editor.setProjectFilter(["silkboard", "hoard"], "silkboard");
  expect(
    host
      .querySelector("[data-todo-key=c]")
      .classList.contains("todo-project-hidden"),
  ).toBe(true);
  expect(host.querySelector("[data-todo-key=a] input").disabled).toBe(true);
  editor.focus("b", 2);
  editor.view.dispatch(editor.view.state.tr.insertText("x"));
  expect(editor.rows()[1].text).toBe("Bexta @silkboard");
  key(editor, "Enter");
  key(editor, "Tab");
  key(editor, "Backspace", { ctrlKey: true, shiftKey: true });
  expect(editor.rows()).toHaveLength(3);
  expect(editor.rows()[0]).toEqual(a);
  expect(editor.rows()[2]).toEqual(rows[2]);
  expect(callbacks.hint).toHaveBeenCalledWith(
    expect.stringMatching(/full list/i),
  );
  editor.focus("a", 0);
  editor.view.dispatch(editor.view.state.tr.insertText("no"));
  expect(editor.rows()[0]).toEqual(a);
  editor.setProjectFilter(["silkboard", "hoard"], "");
  expect(
    host
      .querySelector("[data-todo-key=c]")
      .classList.contains("todo-project-hidden"),
  ).toBe(false);
});
test("filtering permits text replacement and paste, keeps a retagged item while editing, and rejects cross-item deletion", () => {
  const { editor, host } = start([
    a,
    { ...b, text: "Beta @silkboard" },
    { key: "c", depth: 1, text: "Next @silkboard", completed: false },
  ]);
  editor.setProjectFilter(["silkboard", "hoard"], "silkboard");
  editor.focus("b", 0);
  editor.setText("b", "Now untagged", 4);
  expect(
    host
      .querySelector("[data-todo-key=b]")
      .classList.contains("todo-project-hidden"),
  ).toBe(false);
  editor.view.dispatch(
    editor.view.state.tr.setSelection(
      TextSelection.create(editor.view.state.doc, 8, 11),
    ),
  );
  key(editor, "Backspace");
  expect(editor.rows()[1].text).not.toBe("Now untagged");
  editor.action({ paste: "replacement" });
  expect(editor.rows()[1].text).toContain("replacement");
  editor.focus("c", 0);
  expect(
    host
      .querySelector("[data-todo-key=b]")
      .classList.contains("todo-project-hidden"),
  ).toBe(true);
  const before = editor.rows();
  editor.view.dispatch(
    editor.view.state.tr.setSelection(new AllSelection(editor.view.state.doc)),
  );
  key(editor, "Delete");
  expect(editor.rows()).toEqual(before);
  editor.setText("a", "Forbidden");
  expect(editor.rows()[0]).toEqual(a);
  editor.setText("missing", "Ignored");
});
test("arrows use adjacent editable rows even if coordinate hit-testing points at a parent", () => {
  const { editor } = start([
    a,
    b,
    { key: "c", text: "Next", depth: 1, completed: false },
  ]);
  editor.focus("b", 2);
  vi.spyOn(editor.view, "endOfTextblock").mockReturnValue(true);
  vi.spyOn(editor.view, "coordsAtPos").mockReturnValue({
    left: 10,
    right: 10,
    top: 10,
    bottom: 20,
  });
  vi.spyOn(editor.view, "posAtCoords").mockReturnValue({ pos: 1, inside: 0 });
  key(editor, "ArrowDown");
  expect(editor.selection().head.key).toBe("c");
  key(editor, "ArrowDown");
  expect(editor.selection().head.key).toBe("c");
  key(editor, "ArrowUp", { shiftKey: true });
  expect(editor.selection().anchor.key).toBe("c");
  expect(editor.selection().head.key).toBe("b");
  editor.focus("a", 0);
  key(editor, "ArrowUp");
  expect(editor.selection().head.key).toBe("a");
});
test("native transactions retain stable row DOM and checkboxes update independently", () => {
  const { editor, host, callbacks } = start();
  const child = host.querySelector("[data-todo-key=b]");
  editor.focus("a", 2);
  editor.view.dispatch(editor.view.state.tr.insertText("x"));
  expect(editor.rows()[0].text).toBe("Alxpha");
  expect(host.querySelector("[data-todo-key=b]")).toBe(child);
  const checkbox = host.querySelector("input");
  checkbox.checked = true;
  checkbox.dispatchEvent(new Event("change", { bubbles: true }));
  expect(editor.rows().map((r) => r.completed)).toEqual([true, true]);
  expect(callbacks.change).toHaveBeenCalled();
  checkbox.dispatchEvent(
    new KeyboardEvent("keydown", {
      key: "Escape",
      bubbles: true,
      cancelable: true,
    }),
  );
  expect(callbacks.leave).toHaveBeenCalled();
});
test("keyboard splits, subtree nesting and native history preserve document selections", () => {
  const { editor, callbacks } = start();
  editor.focus("a", 2);
  key(editor, "Enter");
  expect(editor.rows().map((r) => r.text)).toEqual(["Al", "Beta", "pha"]);
  expect(editor.selection().head.offset).toBe(0);
  key(editor, "Tab");
  expect(editor.rows().at(-1).depth).toBe(2);
  key(editor, "Tab", { shiftKey: true });
  expect(editor.rows().at(-1).depth).toBe(1);
  editor.undo();
  expect(editor.rows().at(-1).depth).toBe(2);
  editor.redo();
  expect(editor.rows().at(-1).depth).toBe(1);
  expect(callbacks.history).toHaveBeenCalledWith(true, false);
  key(editor, "Escape");
  expect(callbacks.leave).toHaveBeenCalled();
});
test("invalid nesting and malformed native structural transactions cannot corrupt the model", () => {
  const { editor, callbacks } = start();
  editor.focus("a", 0);
  key(editor, "Tab");
  expect(callbacks.hint).toHaveBeenCalled();
  expect(editor.rows()).toEqual([a, b]);
  for (const attrs of [
    { ...a, depth: 6 },
    { ...a, key: null },
    { ...a, key: "invalid key" },
    { ...a, completed: 1 },
  ])
    editor.view.dispatch(
      editor.view.state.tr.setNodeMarkup(0, undefined, attrs),
    );
  expect(editor.rows()).toEqual([a, b]);
  editor.focus("missing");
  expect(editor.rows()).toEqual([a, b]);
});
test("Backspace and Delete join only at boundaries and ordinary editing keys remain native", () => {
  const { editor } = start([a, { ...b, depth: 1 }]);
  editor.focus("b", 0);
  key(editor, "Backspace");
  expect(editor.rows()[0].text).toBe("AlphaBeta");
  editor.undo();
  editor.focus("a", 5);
  key(editor, "Delete");
  expect(editor.rows()[0].text).toBe("AlphaBeta");
  editor.undo();
  editor.focus("a", 2);
  expect(key(editor, "Backspace").defaultPrevented).toBe(false);
  expect(key(editor, "Delete").defaultPrevented).toBe(false);
  vi.spyOn(editor.view, "endOfTextblock").mockReturnValue(false);
  expect(key(editor, "ArrowDown").defaultPrevented).toBe(false);
  expect(key(editor, "Tab", { ctrlKey: true }).defaultPrevented).toBe(false);
});
test("mobile beforeinput, plain-text paste and cut use the same structural rules", () => {
  const { editor } = start([{ ...a, text: "" }]);
  const paste = new Event("paste", { bubbles: true, cancelable: true });
  Object.defineProperty(paste, "clipboardData", {
    value: {
      getData: (type) =>
        type === "text/plain" ? "[x] Parent\n\t[ ] Child" : "",
    },
  });
  editor.view.dom.dispatchEvent(paste);
  expect(editor.rows().map((r) => [r.text, r.depth, r.completed])).toEqual([
    ["Parent", 1, true],
    ["Child", 2, false],
  ]);
  editor.focus(editor.rows()[1].key, 0);
  input(editor, "deleteContentBackward");
  expect(editor.rows().map((r) => r.text)).toEqual(["ParentChild"]);
  editor.undo();
  editor.focus("a", 6);
  input(editor, "insertParagraph");
  expect(editor.rows()).toHaveLength(3);
  editor.view.dispatch(
    editor.view.state.tr.setSelection(new AllSelection(editor.view.state.doc)),
  );
  const cut = new Event("cut", { bubbles: true, cancelable: true }),
    setData = vi.fn();
  Object.defineProperty(cut, "clipboardData", { value: { setData } });
  editor.view.dom.dispatchEvent(cut);
  expect(setData).toHaveBeenCalledWith(
    "text/plain",
    expect.stringContaining("Parent"),
  );
  expect(editor.rows()).toHaveLength(1);
  expect(editor.rows()[0].text).toBe("");
});
test("cross-task text input uses the actual event range and preserves remaining children", () => {
  const { editor } = start([
    a,
    { ...b, depth: 1 },
    { key: "c", text: "Child", depth: 2, completed: true },
  ]);
  const boundary = editor.view.state.doc.child(0).nodeSize;
  expect(
    editor.view.props.handleTextInput(editor.view, 2, boundary + 2, "X"),
  ).toBe(true);
  expect(editor.rows().map((r) => [r.text, r.depth])).toEqual([
    ["AXeta", 1],
    ["Child", 1],
  ]);
  expect(editor.view.props.handleTextInput(editor.view, 1, 1, "x")).toBe(false);
});
test("composition is delegated to the browser and does not trigger structural commands", () => {
  const { editor, callbacks } = start();
  editor.view.dom.dispatchEvent(
    new CompositionEvent("compositionstart", { bubbles: true }),
  );
  expect(callbacks.composing).toHaveBeenCalledWith(true);
  key(editor, "Enter", { isComposing: true });
  expect(editor.rows()).toEqual([a, b]);
  editor.view.dom.dispatchEvent(
    new CompositionEvent("compositionend", { bubbles: true }),
  );
  expect(callbacks.composing).toHaveBeenCalledWith(false);
});
test("native DOM text changes are read without treating checkbox labels as document text", async () => {
  const { editor, host } = start();
  const text = host.querySelector(".todo-task-text");
  text.textContent = "Changed";
  editor.view.dom.dispatchEvent(
    new InputEvent("input", { bubbles: true, inputType: "insertText" }),
  );
  await waitFor(() => expect(editor.rows()[0].text).toBe("Changed"));
  expect(host.querySelector("input").getAttribute("aria-label")).toBe(
    "Changed",
  );
});
test("empty clipboard, native nonstructural input and unsupported drag keep existing tasks", () => {
  const { editor, callbacks } = start();
  const event = new Event("paste", { bubbles: true, cancelable: true });
  Object.defineProperty(event, "clipboardData", {
    value: { getData: () => "" },
  });
  editor.view.dom.dispatchEvent(event);
  expect(callbacks.hint).toHaveBeenCalledWith("Paste plain text to add tasks.");
  expect(input(editor, "formatBold").defaultPrevented).toBe(false);
  editor.view.props.handleDrop();
  expect(callbacks.hint).toHaveBeenCalledWith(
    expect.stringContaining("nesting"),
  );
  expect(editor.rows()).toEqual([a, b]);
});
test("cut without a selection and history shortcuts have normal no-op behavior", () => {
  const { editor } = start();
  editor.focus("a", 1);
  const event = new Event("cut", { bubbles: true, cancelable: true });
  Object.defineProperty(event, "clipboardData", {
    value: { setData: vi.fn() },
  });
  editor.view.dom.dispatchEvent(event);
  expect(editor.rows()).toEqual([a, b]);
  editor.view.dispatch(editor.view.state.tr.insertText("z"));
  key(editor, "z", { ctrlKey: true });
  expect(editor.rows()[0].text).toBe("Alpha");
  key(editor, "y", { ctrlKey: true });
  expect(editor.rows()[0].text).toBe("Azlpha");
  editor.view.dispatch(
    editor.view.state.tr.setSelection(
      TextSelection.create(editor.view.state.doc, 1),
    ),
  );
  expect(editor.selection().head.key).toBe("a");
});
test("browser beforeinput replacement promotes children before native DOM deletion can lose hierarchy", () => {
  const { editor } = start([
    a,
    { ...b, depth: 1 },
    { key: "c", text: "Child", completed: true, depth: 2 },
  ]);
  editor.view.dispatch(
    editor.view.state.tr.setSelection(
      TextSelection.create(editor.view.state.doc, 6, 11),
    ),
  );
  const event = new InputEvent("beforeinput", {
    bubbles: true,
    cancelable: true,
    inputType: "insertText",
    data: "replacement",
  });
  editor.view.dom.dispatchEvent(event);
  expect(event.defaultPrevented).toBe(true);
  expect(editor.rows()).toEqual([
    { ...a, text: "Alphareplacementa" },
    { key: "c", text: "Child", completed: true, depth: 1 },
  ]);
});
test("native selection is read before input even when selectionchange has not arrived", () => {
  const { editor, host } = start([
    a,
    { ...b, depth: 1 },
    { key: "c", text: "Child", depth: 2, completed: true },
  ]);
  editor.focus("a", 5);
  const texts = host.querySelectorAll(".todo-task-text");
  const range = document.createRange();
  range.setStart(texts[0].firstChild, 5);
  range.setEnd(texts[1].firstChild, 2);
  const selection = getSelection();
  selection.removeAllRanges();
  selection.addRange(range);
  const event = new InputEvent("beforeinput", {
    bubbles: true,
    cancelable: true,
    inputType: "insertText",
    data: "X",
  });
  editor.view.dom.dispatchEvent(event);
  expect(event.defaultPrevented).toBe(true);
  expect(editor.rows()).toEqual([
    { ...a, text: "AlphaXta" },
    { key: "c", text: "Child", depth: 1, completed: true },
  ]);
});
test("cut copies the fresh native range and promotes children before selectionchange", () => {
  const { editor, host } = start([
    a,
    { ...b, depth: 1 },
    { key: "c", text: "Child", depth: 2, completed: true },
  ]);
  editor.focus("a", 5);
  const texts = host.querySelectorAll(".todo-task-text");
  const range = document.createRange();
  range.setStart(texts[0].firstChild, 5);
  range.setEnd(texts[1].firstChild, 2);
  const selection = getSelection();
  selection.removeAllRanges();
  selection.addRange(range);
  const cut = new Event("cut", { bubbles: true, cancelable: true }),
    setData = vi.fn();
  Object.defineProperty(cut, "clipboardData", { value: { setData } });
  editor.view.dom.dispatchEvent(cut);
  expect(cut.defaultPrevented).toBe(true);
  expect(setData).toHaveBeenCalledWith("text/plain", "\nBe");
  expect(editor.rows()).toEqual([
    { ...a, text: "Alphata" },
    { key: "c", text: "Child", depth: 1, completed: true },
  ]);
});
test("starting IME over a cross-task native selection preserves surviving child hierarchy", () => {
  const { editor, host } = start([
    a,
    { ...b, depth: 1 },
    { key: "c", text: "Child", depth: 2, completed: true },
  ]);
  editor.focus("a", 5);
  const texts = host.querySelectorAll(".todo-task-text");
  const range = document.createRange();
  range.setStart(texts[0].firstChild, 5);
  range.setEnd(texts[1].firstChild, 2);
  const selection = getSelection();
  selection.removeAllRanges();
  selection.addRange(range);
  editor.view.dom.dispatchEvent(
    new CompositionEvent("compositionstart", { bubbles: true }),
  );
  expect(editor.rows()).toEqual([
    { ...a, text: "Alphata" },
    { key: "c", text: "Child", depth: 1, completed: true },
  ]);
  editor.view.dom.dispatchEvent(
    new CompositionEvent("compositionend", { bubbles: true, data: "日" }),
  );
});
