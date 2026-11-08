import { Schema, type Node as DocumentNode } from "prosemirror-model";
import {
  EditorState,
  Plugin,
  TextSelection,
  type Transaction,
} from "prosemirror-state";
import { EditorView, Decoration, DecorationSet } from "prosemirror-view";
import { baseKeymap } from "prosemirror-commands";
import { keymap } from "prosemirror-keymap";
import {
  history,
  undo,
  redo,
  undoDepth,
  redoDepth,
  closeHistory,
} from "prosemirror-history";
import {
  act,
  newTaskKey,
  type TaskRow,
  type TaskSelection,
  type TaskAction,
} from "./todo-editor-model";
import "prosemirror-view/style/prosemirror.css";
import {
  projectRows,
  tagsIn,
  mentionAt,
  type ProjectMention,
} from "./todo-projects";

const schema = new Schema({
  nodes: {
    doc: { content: "task+" },
    task: {
      content: "text*",
      group: "block",
      selectable: false,
      attrs: {
        key: { default: null },
        depth: { default: 1 },
        completed: { default: false },
      },
      parseDOM: [
        {
          tag: "div[data-todo-key]",
          getAttrs: (node) => ({
            key: (node as HTMLElement).dataset.todoKey,
            depth: Number((node as HTMLElement).dataset.depth),
            completed: (node as HTMLElement).dataset.completed === "true",
          }),
        },
      ],
      toDOM: (node) => ["div", { "data-depth": node.attrs.depth }, 0],
    },
    text: { group: "inline" },
  },
});
export interface TodoMention extends ProjectMention {
  key: string;
  left: number;
  top: number;
}
export interface EditorCallbacks {
  // eslint-disable-next-line no-unused-vars -- Callback parameters document the editor boundary.
  change: (rows: TaskRow[]) => void;
  // eslint-disable-next-line no-unused-vars -- Callback parameters document the editor boundary.
  selection: (key: string) => void;
  // eslint-disable-next-line no-unused-vars -- Callback parameters document the editor boundary.
  history: (canUndo: boolean, canRedo: boolean) => void;
  // eslint-disable-next-line no-unused-vars -- Callback parameters document the editor boundary.
  hint: (message: string) => void;
  leave: () => void;
  focus: () => void;
  // eslint-disable-next-line no-unused-vars -- Callback parameters document the editor boundary.
  composing: (value: boolean) => void;
  reveal?: () => void;
  // eslint-disable-next-line no-unused-vars -- Optional autocomplete boundary.
  mention?: (value: TodoMention | null) => void;
  // eslint-disable-next-line no-unused-vars -- Optional autocomplete key handling.
  keydown?: (event: KeyboardEvent) => boolean;
}
function documentOf(rows: TaskRow[]) {
  return schema.node(
    "doc",
    null,
    rows.map((row) =>
      schema.node(
        "task",
        { key: row.key, depth: row.depth, completed: row.completed },
        row.text ? schema.text(row.text) : undefined,
      ),
    ),
  );
}
export function editorRows(doc: DocumentNode): TaskRow[] {
  const rows: TaskRow[] = [];
  doc.forEach((node) =>
    rows.push({
      key: node.attrs.key,
      depth: node.attrs.depth,
      completed: node.attrs.completed,
      text: node.textContent,
    }),
  );
  return rows;
}
function selectionOf(
  state: Pick<EditorState, "doc" | "selection">,
  from = state.selection.anchor,
  to = state.selection.head,
): TaskSelection {
  const at = (position: number) => {
    let result = { key: "", offset: 0 };
    state.doc.forEach((node, offset, index) => {
      if (
        position >= offset &&
        (position < offset + node.nodeSize ||
          index === state.doc.childCount - 1)
      )
        result = {
          key: node.attrs.key,
          offset: Math.max(
            0,
            Math.min(node.textContent.length, position - offset - 1),
          ),
        };
    });
    return result;
  };
  return { anchor: at(from), head: at(to) };
}
function positionOf(doc: DocumentNode, point: { key: string; offset: number }) {
  let position = 1;
  doc.forEach((node, offset) => {
    if (node.attrs.key === point.key)
      position = offset + 1 + Math.min(point.offset, node.textContent.length);
  });
  return position;
}
const validDocument = (doc: DocumentNode) => {
  const rows = editorRows(doc),
    keys = new Set<string>();
  return (
    rows.length <= 1000 &&
    rows.every((row, index) => {
      const valid =
        typeof row.key === "string" &&
        /^[A-Za-z0-9_-]{1,64}$/.test(row.key) &&
        !keys.has(row.key) &&
        Number.isInteger(row.depth) &&
        row.depth >= 1 &&
        row.depth <= 5 &&
        row.depth <= (index ? rows[index - 1].depth + 1 : 1) &&
        typeof row.completed === "boolean";
      keys.add(row.key);
      return valid;
    })
  );
};
export function createTodoEditor(
  host: HTMLElement,
  rows: TaskRow[],
  callbacks: EditorCallbacks,
) {
  let knownProjects: readonly string[] = [],
    projectFilter = "",
    pinned = "",
    settingsKey = "";
  let cachedDoc: DocumentNode | null = null,
    cachedKey = "";
  let cachedProjection = {
    matching: new Set<string>(),
    visible: new Set<string>(),
  };
  const projection = (doc: DocumentNode) => {
    const cacheKey = settingsKey + pinned;
    if (cachedDoc === doc && cachedKey === cacheKey) return cachedProjection;
    const rows = editorRows(doc),
      result = projectRows(rows, knownProjects, projectFilter);
    if (pinned && projectFilter) {
      const ancestors: string[] = [];
      for (const row of rows) {
        ancestors.length = row.depth - 1;
        ancestors.push(row.key);
        if (row.key === pinned) {
          for (const key of ancestors) result.visible.add(key);
          break;
        }
      }
    }
    cachedDoc = doc;
    cachedKey = cacheKey;
    return (cachedProjection = result);
  };
  const canEdit = (key: string, doc: DocumentNode) =>
    !projectFilter || key === pinned || projection(doc).matching.has(key);
  const fullListHint = () =>
    callbacks.hint(
      "Show the full list to add, delete, move items or edit parent context.",
    );
  // A browser selectionchange can arrive after its next key/input event.
  // Read the native range before structural edits rather than acting on the
  // previous caret while the browser replaces a newly selected task range.
  const syncSelection = () => {
    const selection = host.ownerDocument.getSelection();
    if (
      !selection?.anchorNode ||
      !selection.focusNode ||
      !view.dom.contains(selection.anchorNode) ||
      !view.dom.contains(selection.focusNode)
    )
      return;
    const points = selectionOf(
      view.state,
      view.posAtDOM(selection.anchorNode, selection.anchorOffset),
      view.posAtDOM(selection.focusNode, selection.focusOffset),
    );
    const next = TextSelection.create(
      view.state.doc,
      positionOf(view.state.doc, points.anchor),
      positionOf(view.state.doc, points.head),
    );
    if (!next.eq(view.state.selection))
      view.dispatch(view.state.tr.setSelection(next));
  };
  const run = (action: TaskAction, selection?: TaskSelection) => {
    if (!selection) syncSelection();
    const before = editorRows(view.state.doc),
      current = selection ?? selectionOf(view.state);
    if (
      projectFilter &&
      ((!["toggle", "backspace", "delete"].includes(String(action)) &&
        !(
          typeof action === "object" &&
          !/[\r\n]/.test("insert" in action ? action.insert : action.paste)
        )) ||
        current.anchor.key !== current.head.key ||
        !canEdit(current.head.key, view.state.doc))
    ) {
      fullListHint();
      return true;
    }
    const result = act(before, current, action, newTaskKey);
    if (result.hint) callbacks.hint(result.hint);
    if (result.rows === before) return true;
    const doc = documentOf(result.rows);
    const tr = view.state.tr.replaceWith(
      0,
      view.state.doc.content.size,
      doc.content,
    );
    tr.setSelection(
      TextSelection.create(
        tr.doc,
        positionOf(tr.doc, result.selection.anchor),
        positionOf(tr.doc, result.selection.head),
      ),
    );
    view.dispatch(closeHistory(tr).scrollIntoView());
    return true;
  };
  const historyCommand = (command: typeof undo) =>
    command(view.state, (tr) => view.dispatch(tr.scrollIntoView()));
  const boundary = (backwards: boolean) => {
    const selection = selectionOf(view.state);
    const row = editorRows(view.state.doc).find(
      (row) => row.key === selection.head.key,
    )!;
    return (
      !view.state.selection.empty ||
      !row.text ||
      (backwards
        ? selection.head.offset === 0
        : selection.head.offset === row.text.length)
    );
  };
  const view: EditorView = new EditorView(host, {
    state: EditorState.create({
      doc: documentOf(rows),
      plugins: [
        history({ depth: 100, newGroupDelay: 500 }),
        keymap({
          "Mod-z": () => historyCommand(undo),
          "Shift-Mod-z": () => historyCommand(redo),
          "Mod-y": () => historyCommand(redo),
        }),
        keymap(baseKeymap),
        new Plugin({
          filterTransaction(tr, state) {
            if (tr.docChanged && !validDocument(tr.doc)) return false;
            if (projectFilter && tr.docChanged) {
              const before = editorRows(state.doc),
                after = editorRows(tr.doc);
              const allowed =
                before.length === after.length &&
                before.every((row, index) => {
                  const next = after[index];
                  return (
                    row.key === next.key &&
                    row.depth === next.depth &&
                    (canEdit(row.key, state.doc) ||
                      (row.text === next.text &&
                        row.completed === next.completed))
                  );
                });
              if (!allowed) {
                fullListHint();
                return false;
              }
              const key = selectionOf(state).head.key;
              if (canEdit(key, state.doc)) pinned = key;
            } else if (pinned && selectionOf(tr).head.key !== pinned)
              pinned = "";
            return true;
          },
          props: {
            decorations(state) {
              const decorations: Decoration[] = [],
                active = selectionOf(state).head.key;
              const scope = projection(state.doc);
              state.doc.forEach((node, offset, index) => {
                const parent =
                  index + 1 < state.doc.childCount &&
                  state.doc.child(index + 1).attrs.depth > node.attrs.depth;
                const classes = [
                  node.attrs.key === active ? "active" : "",
                  parent ? "parent" : "",
                  projectFilter && !scope.visible.has(node.attrs.key)
                    ? "todo-project-hidden"
                    : "",
                  projectFilter && !canEdit(node.attrs.key, state.doc)
                    ? "todo-project-context"
                    : "",
                ]
                  .filter(Boolean)
                  .join(" ");
                if (classes)
                  decorations.push(
                    Decoration.node(offset, offset + node.nodeSize, {
                      class: classes,
                    }),
                  );
                for (const tag of tagsIn(node.textContent, knownProjects))
                  decorations.push(
                    Decoration.inline(
                      offset + 1 + tag.from,
                      offset + 1 + tag.to,
                      { class: "todo-project-tag" },
                    ),
                  );
              });
              return DecorationSet.create(state.doc, decorations);
            },
          },
        }),
      ],
    }),
    attributes: {
      role: "textbox",
      "aria-label": "Todo document",
      "aria-multiline": "true",
      "aria-describedby": "todo-editor-instructions",
      tabindex: "0",
      spellcheck: "true",
    },
    dispatchTransaction(tr: Transaction) {
      const before = view.state;
      const result = before.applyTransaction(tr);
      view.updateState(result.state);
      if (before.doc !== result.state.doc)
        callbacks.change(editorRows(result.state.doc));
      callbacks.selection(selectionOf(result.state).head.key);
      callbacks.history(
        undoDepth(result.state) > 0,
        redoDepth(result.state) > 0,
      );
      if (
        view.hasFocus() &&
        result.state.selection.empty &&
        callbacks.mention
      ) {
        const caret = selectionOf(result.state).head;
        const row = editorRows(result.state.doc).find(
          (row) => row.key === caret.key,
        )!;
        const mention = mentionAt(row.text, caret.offset);
        const rect = view.coordsAtPos(result.state.selection.head);
        callbacks.mention(
          mention && canEdit(row.key, result.state.doc)
            ? { ...mention, key: row.key, left: rect.left, top: rect.bottom }
            : null,
        );
      } else callbacks.mention?.(null);
      if (view.hasFocus())
        requestAnimationFrame(() => {
          if (!view.isDestroyed) callbacks.reveal?.();
        });
    },
    nodeViews: {
      task(node, editor, getPos) {
        let current = node;
        const dom = document.createElement("div"),
          label = document.createElement("label"),
          input = document.createElement("input"),
          contentDOM = document.createElement("span");
        dom.className = "todo-task";
        label.className = "todo-check-target";
        label.contentEditable = "false";
        input.type = "checkbox";
        label.append(input);
        contentDOM.className = "todo-task-text";
        dom.append(label, contentDOM);
        const update = (next: DocumentNode) => {
          current = next;
          dom.dataset.todoKey = next.attrs.key;
          dom.dataset.depth = String(next.attrs.depth);
          dom.dataset.completed = String(next.attrs.completed);
          dom.dataset.empty = String(!next.textContent);
          dom.style.setProperty("--depth", String(next.attrs.depth - 1));
          input.checked = next.attrs.completed;
          const editable = canEdit(next.attrs.key, editor.state.doc);
          input.disabled = !editable;
          if (editable) dom.removeAttribute("contenteditable");
          else dom.contentEditable = "false";
          input.setAttribute("aria-label", next.textContent || "Empty todo");
          return true;
        };
        update(node);
        input.addEventListener("change", () => {
          const position = getPos();
          if (position === undefined) return;
          editor.dispatch(
            closeHistory(
              editor.state.tr.setNodeMarkup(position, undefined, {
                ...current.attrs,
                completed: input.checked,
              }),
            ),
          );
        });
        input.addEventListener("keydown", (event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            callbacks.leave();
          }
        });
        return {
          dom,
          contentDOM,
          update: (next) => next.type === current.type && update(next),
          stopEvent: (event) => label.contains(event.target as Node),
          ignoreMutation: (mutation) =>
            mutation.type !== "selection" &&
            !contentDOM.contains(mutation.target),
        };
      },
    },
    handleKeyDown(_editor, event) {
      if (event.isComposing || view.composing) return false;
      syncSelection();
      if (callbacks.keydown?.(event)) return true;
      const mod = (event.ctrlKey || event.metaKey) && !event.altKey;
      if (mod && event.key === "Enter" && !event.shiftKey) return run("toggle");
      if (mod && event.shiftKey && event.key === "Backspace")
        return run("remove");
      if (
        ["ArrowUp", "ArrowDown"].includes(event.key) &&
        !event.ctrlKey &&
        !event.metaKey &&
        !event.altKey
      ) {
        const down = event.key === "ArrowDown";
        if (view.endOfTextblock(down ? "down" : "up")) {
          const rows = editorRows(view.state.doc),
            current = selectionOf(view.state).head;
          let index =
            rows.findIndex((row) => row.key === current.key) + (down ? 1 : -1);
          while (rows[index] && !canEdit(rows[index].key, view.state.doc))
            index += down ? 1 : -1;
          const row = rows[index];
          if (!row) return true;
          const start = positionOf(view.state.doc, { key: row.key, offset: 0 }),
            end = start + row.text.length;
          const caret = view.coordsAtPos(view.state.selection.head),
            edge = view.coordsAtPos(down ? start : end);
          const dom = view.nodeDOM(start - 1) as HTMLElement;
          const rect = dom
            .querySelector(".todo-task-text")!
            .getBoundingClientRect();
          const found = view.posAtCoords({
            left: Math.max(rect.left + 1, Math.min(caret.left, rect.right - 1)),
            top: (edge.top + edge.bottom) / 2,
          });
          const target = Math.max(
            start,
            Math.min(end, found?.pos ?? (down ? start : end)),
          );
          view.dispatch(
            view.state.tr
              .setSelection(
                TextSelection.create(
                  view.state.doc,
                  event.shiftKey ? view.state.selection.anchor : target,
                  target,
                ),
              )
              .scrollIntoView(),
          );
          return true;
        }
      }
      if (event.key === "Escape") {
        event.preventDefault();
        callbacks.leave();
        return true;
      }
      if (
        event.key === "Tab" &&
        !event.ctrlKey &&
        !event.metaKey &&
        !event.altKey
      )
        return run(event.shiftKey ? "outdent" : "indent");
      if (event.key === "Enter") return run("enter");
      if (event.key === "Backspace" && boundary(true)) return run("backspace");
      if (event.key === "Delete" && boundary(false)) return run("delete");
      return false;
    },
    handleTextInput(_editor, from, to, text) {
      const selection = selectionOf(view.state, from, to);
      return selection.anchor.key !== selection.head.key
        ? run({ insert: text }, selection)
        : false;
    },
    handlePaste(_editor, event) {
      const text = event.clipboardData?.getData("text/plain");
      if (!text) {
        callbacks.hint("Paste plain text to add tasks.");
        return true;
      }
      event.preventDefault();
      return run({ paste: text });
    },
    handleDrop() {
      callbacks.hint(
        "Use Tab and Shift+Tab, or the shared tools, to change nesting.",
      );
      return true;
    },
    handleDOMEvents: {
      focus() {
        callbacks.focus();
        return false;
      },
      compositionstart() {
        callbacks.composing(true);
        syncSelection();
        const selection = selectionOf(view.state);
        if (selection.anchor.key !== selection.head.key) run("delete");
        return false;
      },
      compositionend() {
        callbacks.composing(false);
        return false;
      },
      beforeinput(_editor, event) {
        if (event.isComposing || view.composing) return false;
        syncSelection();
        let handled = false;
        const selection = selectionOf(view.state);
        if (
          ["insertText", "insertReplacementText"].includes(event.inputType) &&
          event.data !== null &&
          selection.anchor.key !== selection.head.key
        )
          handled = run({ insert: event.data });
        else if (
          ["insertParagraph", "insertLineBreak"].includes(event.inputType)
        )
          handled = run("enter");
        else if (
          ["deleteContentBackward", "deleteWordBackward"].includes(
            event.inputType,
          ) &&
          boundary(true)
        )
          handled = run("backspace");
        else if (
          ["deleteContentForward", "deleteWordForward", "deleteByCut"].includes(
            event.inputType,
          ) &&
          boundary(false)
        )
          handled = run("delete");
        if (handled) event.preventDefault();
        return handled;
      },
      cut(_editor, event) {
        syncSelection();
        if (view.state.selection.empty || !event.clipboardData) return false;
        const { from, to } = view.state.selection;
        event.clipboardData.setData(
          "text/plain",
          view.state.doc.textBetween(from, to, "\n"),
        );
        event.preventDefault();
        return run("delete");
      },
    },
  });
  return {
    view,
    action: run,
    setProjectFilter(projects: readonly string[], filter: string) {
      const next = JSON.stringify([projects, filter]);
      if (next === settingsKey) return;
      settingsKey = next;
      knownProjects = projects;
      projectFilter = filter;
      pinned = "";
      let tr = view.state.tr.setMeta("project-filter", true);
      if (
        filter &&
        !canEdit(selectionOf(view.state).head.key, view.state.doc)
      ) {
        const first = editorRows(view.state.doc).find((row) =>
          canEdit(row.key, view.state.doc),
        );
        if (first)
          tr = tr.setSelection(
            TextSelection.create(
              view.state.doc,
              positionOf(view.state.doc, { key: first.key, offset: 0 }),
            ),
          );
      }
      view.dispatch(tr);
    },
    setText(key: string, text: string, offset = text.length) {
      const row = editorRows(view.state.doc).find((row) => row.key === key);
      if (!row || !canEdit(key, view.state.doc)) return;
      const start = positionOf(view.state.doc, { key, offset: 0 });
      const tr = view.state.tr.insertText(text, start, start + row.text.length);
      view.dispatch(
        closeHistory(
          tr.setSelection(TextSelection.create(tr.doc, start + offset)),
        ),
      );
    },
    selection: () => selectionOf(view.state),
    rows: () => editorRows(view.state.doc),
    focus(key?: string, offset?: number) {
      if (key) {
        const row = editorRows(view.state.doc).find((row) => row.key === key);
        if (!row) return;
        const position = positionOf(view.state.doc, {
          key,
          offset: offset ?? row.text.length,
        });
        view.dispatch(
          view.state.tr
            .setSelection(TextSelection.create(view.state.doc, position))
            .scrollIntoView(),
        );
      }
      view.focus();
    },
    undo: () => historyCommand(undo),
    redo: () => historyCommand(redo),
    destroy: () => view.destroy(),
  };
}
export type TodoEditor = ReturnType<typeof createTodoEditor>;
