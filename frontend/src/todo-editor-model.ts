import { taskText, type Checklist, type DocumentEntry } from "./todos-api";
export interface TaskRow {
  key: string;
  text: string;
  depth: number;
  completed: boolean;
}
export interface TaskPoint {
  key: string;
  offset: number;
}
export interface TaskSelection {
  anchor: TaskPoint;
  head: TaskPoint;
}
export type TaskAction =
  | "enter"
  | "new"
  | "backspace"
  | "delete"
  | "indent"
  | "outdent"
  | "toggle"
  | "remove"
  | { insert: string }
  | { paste: string };
export interface TaskEdit {
  rows: TaskRow[];
  selection: TaskSelection;
  hint: string;
}
export const newTaskKey = () => `task-${crypto.randomUUID()}`;
const point = (key: string, offset: number): TaskSelection => ({
  anchor: { key, offset },
  head: { key, offset },
});
const empty = (key: () => string): TaskRow => ({
  key: key(),
  text: "",
  depth: 1,
  completed: false,
});
function end(rows: TaskRow[], index: number) {
  let next = index + 1;
  while (next < rows.length && rows[next].depth > rows[index].depth) next++;
  return next;
}
function range(rows: TaskRow[], selection: TaskSelection) {
  const locate = (p: TaskPoint) => {
    const index = Math.max(
      0,
      rows.findIndex((row) => row.key === p.key),
    );
    return {
      index,
      offset: Math.max(0, Math.min(p.offset, rows[index].text.length)),
    };
  };
  const a = locate(selection.anchor),
    b = locate(selection.head);
  return a.index < b.index || (a.index === b.index && a.offset <= b.offset)
    ? [a, b]
    : [b, a];
}
function parents(rows: TaskRow[]) {
  const result = new Map<string, string | null>(),
    stack: string[] = [];
  for (const row of rows) {
    stack.length = row.depth - 1;
    result.set(row.key, stack.at(-1) ?? null);
    stack.push(row.key);
  }
  return result;
}
function remove(rows: TaskRow[], keys: Set<string>) {
  const before = parents(rows),
    depths = new Map<string, number>();
  return rows
    .filter((row) => !keys.has(row.key))
    .map((row) => {
      let parent = before.get(row.key) ?? null;
      while (parent && keys.has(parent)) parent = before.get(parent) ?? null;
      const depth = parent ? depths.get(parent)! + 1 : 1;
      depths.set(row.key, depth);
      return depth === row.depth ? row : { ...row, depth };
    });
}
function replace(
  rows: TaskRow[],
  selection: TaskSelection,
  text: string,
  key: () => string,
): TaskEdit {
  const [a, b] = range(rows, selection);
  if (
    !text &&
    a.index === 0 &&
    a.offset === 0 &&
    b.index === rows.length - 1 &&
    b.offset === rows[b.index].text.length
  ) {
    const row = empty(key);
    return { rows: [row], selection: point(row.key, 0), hint: "" };
  }
  const first = rows[a.index],
    last = rows[b.index];
  const changed = {
    ...first,
    text: first.text.slice(0, a.offset) + text + last.text.slice(b.offset),
  };
  const removed = new Set(
    rows.slice(a.index + 1, b.index + 1).map((row) => row.key),
  );
  const result = remove(rows, removed).map((row) =>
    row.key === first.key ? changed : row,
  );
  return {
    rows: result,
    selection: point(first.key, a.offset + text.length),
    hint: "",
  };
}
function valid(rows: TaskRow[]) {
  const keys = new Set<string>();
  return (
    rows.length > 0 &&
    rows.length <= 1000 &&
    rows.every((row, index) => {
      const okay =
        !keys.has(row.key) &&
        row.depth >= 1 &&
        row.depth <= 5 &&
        row.depth <= (index ? rows[index - 1].depth + 1 : 1);
      keys.add(row.key);
      return okay;
    })
  );
}
export function act(
  rows: TaskRow[],
  selection: TaskSelection,
  action: TaskAction,
  key = newTaskKey,
): TaskEdit {
  const unchanged = (hint = ""): TaskEdit => ({ rows, selection, hint });
  const [a, b] = range(rows, selection);
  if (action === "toggle" || action === "remove") {
    const index = rows.findIndex((row) => row.key === selection.head.key);
    const row = rows[index];
    if (action === "toggle")
      return {
        rows: rows.map((item) =>
          item.key === row.key ? { ...item, completed: !item.completed } : item,
        ),
        selection,
        hint: "",
      };
    const remaining = remove(rows, new Set([row.key]));
    if (!remaining.length) remaining.push(empty(key));
    return {
      rows: remaining,
      selection: point(remaining[Math.min(index, remaining.length - 1)].key, 0),
      hint: "",
    };
  }

  if (typeof action !== "string") {
    if ("insert" in action) return replace(rows, selection, action.insert, key);
    const lines = action.paste
      .replace(/\r\n?/g, "\n")
      .split("\n")
      .map((line) => {
        const match = /^(\t*)(?:[-*] )?(?:\[([ xX])\] )?(.*)$/.exec(line)!;
        return {
          indent: match[1].length,
          completed:
            match[2] === undefined ? undefined : match[2].toLowerCase() === "x",
          text: match[3].replaceAll("\t", " "),
        };
      });
    if (rows.length - (b.index - a.index) + lines.length - 1 > 1000)
      return unchanged(
        "The list supports up to 1,000 tasks. Paste fewer tasks.",
      );
    const edited = replace(rows, selection, lines[0].text, key);
    const index = edited.rows.findIndex(
        (row) => row.key === edited.selection.head.key,
      ),
      first = edited.rows[index];
    const offset = edited.selection.head.offset,
      suffix = first.text.slice(offset);
    const inserted = lines.slice(1).map((line) => ({
      key: key(),
      depth: first.depth + line.indent - lines[0].indent,
      text: line.text,
      completed: line.completed ?? false,
    }));
    if (inserted.some((row) => row.depth < first.depth))
      return unchanged("Start pasted tasks at the current nesting level.");
    const result = [...edited.rows];
    result[index] = {
      ...first,
      text: lines.length > 1 ? first.text.slice(0, offset) : first.text,
      completed: lines[0].completed ?? first.completed,
    };
    result.splice(end(edited.rows, index), 0, ...inserted);
    if (!valid(result))
      return unchanged("Tasks support five levels. Paste a shallower list.");
    if (!inserted.length) return { ...edited, rows: result };
    const last = inserted.at(-1)!;
    const caret = last.text.length;
    last.text += suffix;
    return { rows: result, selection: point(last.key, caret), hint: "" };
  }
  if (action === "indent" || action === "outdent") {
    const index = rows.findIndex((row) => row.key === selection.head.key),
      row = rows[index];
    if (action === "indent") {
      let previous = index - 1;
      while (previous >= 0 && rows[previous].depth > row.depth) previous--;
      if (previous < 0 || rows[previous].depth !== row.depth)
        return unchanged("This task has no preceding sibling to indent under.");
      const stop = end(rows, index);
      if (rows.slice(index, stop).some((row) => row.depth >= 5))
        return unchanged(
          "Tasks support up to five levels, including children.",
        );
      return {
        rows: rows.map((row, n) =>
          n >= index && n < stop ? { ...row, depth: row.depth + 1 } : row,
        ),
        selection: point(row.key, selection.head.offset),
        hint: "",
      };
    }
    if (row.depth === 1)
      return unchanged("This task is already at the top level.");
    let parent = index - 1;
    while (rows[parent].depth >= row.depth) parent--;
    const stop = end(rows, index),
      after = end(rows, parent);
    const subtree = rows
      .slice(index, stop)
      .map((row) => ({ ...row, depth: row.depth - 1 }));
    return {
      rows: [
        ...rows.slice(0, index),
        ...rows.slice(stop, after),
        ...subtree,
        ...rows.slice(after),
      ],
      selection: point(row.key, selection.head.offset),
      hint: "",
    };
  }
  if (action === "enter" || action === "new") {
    if (rows.length >= 1000)
      return unchanged("The list supports up to 1,000 tasks.");
    const edited =
      action === "enter" && (a.index !== b.index || a.offset !== b.offset)
        ? replace(rows, selection, "", key)
        : unchanged();
    const index = edited.rows.findIndex(
        (row) => row.key === edited.selection.head.key,
      ),
      row = edited.rows[index];
    const offset =
      action === "new" ? row.text.length : edited.selection.head.offset;
    const next = {
      ...empty(key),
      depth: row.depth,
      text: row.text.slice(offset),
    };
    const result = [...edited.rows];
    result[index] = { ...row, text: row.text.slice(0, offset) };
    result.splice(end(edited.rows, index), 0, next);
    return { rows: result, selection: point(next.key, 0), hint: "" };
  }
  if (a.index !== b.index || a.offset !== b.offset)
    return replace(rows, selection, "", key);
  const index = a.index,
    row = rows[index];
  if (!row.text) {
    if (rows.length === 1) return unchanged();
    const result = remove(rows, new Set([row.key]));
    const target =
      action === "backspace" && index > 0
        ? result[index - 1]
        : result[Math.min(index, result.length - 1)];
    return {
      rows: result,
      selection: point(
        target.key,
        action === "backspace" && index > 0 ? target.text.length : 0,
      ),
      hint: "",
    };
  }
  if (action === "backspace") {
    if (a.offset > 0) {
      const size =
        a.offset > 1 &&
        /[\uDC00-\uDFFF]/.test(row.text[a.offset - 1]) &&
        /[\uD800-\uDBFF]/.test(row.text[a.offset - 2])
          ? 2
          : 1;
      return replace(
        rows,
        {
          anchor: { key: row.key, offset: a.offset - size },
          head: { key: row.key, offset: a.offset },
        },
        "",
        key,
      );
    }
    if (!index) return unchanged();
    const previous = rows[index - 1];
    return replace(
      rows,
      {
        anchor: { key: previous.key, offset: previous.text.length },
        head: { key: row.key, offset: 0 },
      },
      "",
      key,
    );
  }
  if (a.offset < row.text.length) {
    const size = row.text.codePointAt(a.offset)! > 0xffff ? 2 : 1;
    return replace(
      rows,
      {
        anchor: { key: row.key, offset: a.offset },
        head: { key: row.key, offset: a.offset + size },
      },
      "",
      key,
    );
  }
  if (index === rows.length - 1) return unchanged();
  return replace(
    rows,
    {
      anchor: { key: row.key, offset: row.text.length },
      head: { key: rows[index + 1].key, offset: 0 },
    },
    "",
    key,
  );
}
export function fromChecklist(data: Checklist) {
  const depths = new Map<string, number>(),
    ids = new Map<string, string>();
  const rows = data.items.map((item) => {
    const depth = item.parent_id ? depths.get(item.parent_id)! + 1 : 1;
    depths.set(item.id, depth);
    const key = `saved-${item.id}`;
    ids.set(key, item.id);
    return { key, text: item.text, completed: item.completed, depth };
  });
  return { rows: rows.length ? rows : [empty(newTaskKey)], ids };
}
export function serialize(
  rows: TaskRow[],
  ids: Map<string, string>,
): { items: DocumentEntry[]; error?: string } {
  if (
    rows.length === 1 &&
    !rows[0].text &&
    !rows[0].completed &&
    !ids.has(rows[0].key)
  )
    return { items: [] };
  if (!valid(rows))
    return {
      items: [],
      error: "Tasks support five levels and up to 1,000 items.",
    };
  const stack: string[] = [],
    items: DocumentEntry[] = [];
  for (const row of rows) {
    const blank = !row.text.trim();
    if (!taskText(row.text))
      return {
        items: [],
        error: !blank
          ? "Use single-line task text of at most 1,000 UTF-8 bytes, without control characters."
          : "Finish or remove empty tasks before saving.",
      };
    stack.length = row.depth - 1;
    items.push({
      key: row.key,
      id: ids.get(row.key) ?? null,
      parent_key: stack.at(-1) ?? null,
      text: row.text,
      completed: row.completed,
    });
    stack.push(row.key);
  }
  return { items };
}
export const draftText = (rows: TaskRow[]) =>
  rows
    .map(
      (row) =>
        `${"\t".repeat(row.depth - 1)}[${row.completed ? "x" : " "}] ${row.text}`,
    )
    .join("\n");
