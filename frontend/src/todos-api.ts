import { ApiError, object, ownerSession, positive, request } from "./feed-api";
export interface Todo {
  id: string;
  parent_id: string | null;
  text: string;
  completed: boolean;
}
export interface Checklist {
  revision: string;
  items: Todo[];
}
export function taskText(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.trim().length > 0 &&
    new TextEncoder().encode(value).length <= 1000 &&
    [...value].every((char) => {
      const code = char.codePointAt(0)!;
      return (
        code >= 32 &&
        !(code >= 127 && code <= 159) &&
        ![0x2028, 0x2029, 0xfeff].includes(code)
      );
    })
  );
}
export function checklist(value: unknown): Checklist {
  if (
    !object(value) ||
    !(value.revision === "0" || positive(value.revision)) ||
    !Array.isArray(value.items) ||
    value.items.length > 1000
  )
    throw new ApiError(0);
  const depths = new Map<string, number>();
  const seen = new Set<string>();
  let ancestors: string[] = [];
  for (const item of value.items) {
    if (
      !object(item) ||
      !positive(item.id) ||
      seen.has(item.id) ||
      !taskText(item.text) ||
      typeof item.completed !== "boolean" ||
      !(
        item.parent_id === null ||
        (typeof item.parent_id === "string" && depths.has(item.parent_id))
      )
    )
      throw new ApiError(0);
    const depth =
      item.parent_id === null ? 1 : depths.get(item.parent_id as string)! + 1;
    if (depth > 5) throw new ApiError(0);
    if (item.parent_id === null) ancestors = [];
    else {
      const parent = ancestors.indexOf(item.parent_id as string);
      if (parent < 0) throw new ApiError(0);
      ancestors = ancestors.slice(0, parent + 1);
    }
    ancestors.push(item.id);
    depths.set(item.id, depth);
    seen.add(item.id);
  }
  return value as unknown as Checklist;
}
export async function loadTodos(signal: AbortSignal): Promise<Checklist> {
  return checklist(await request("/api/owner/todos", {}, signal));
}
export interface DocumentEntry {
  key: string;
  id: string | null;
  parent_key: string | null;
  text: string;
  completed: boolean;
}
export interface SavedDocument extends Checklist {
  keys: Record<string, string>;
}
export async function saveTodoDocument(
  revision: string,
  items: DocumentEntry[],
  signal: AbortSignal,
): Promise<SavedDocument> {
  const { csrf_token } = await ownerSession(signal);
  const value = await request(
    "/api/owner/todos/document",
    {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
        "X-Silkboard-CSRF": csrf_token,
      },
      body: JSON.stringify({ revision, items }),
    },
    signal,
  );
  const data = checklist(value);
  if (
    !object(value) ||
    !object(value.keys) ||
    Object.keys(value.keys).length !== items.length ||
    data.items.length !== items.length ||
    BigInt(data.revision) !== BigInt(revision) + BigInt(1)
  )
    throw new ApiError(0);
  const keys = value.keys;
  for (const [index, item] of items.entries()) {
    const row = data.items[index];
    if (
      !Object.hasOwn(keys, item.key) ||
      !positive(keys[item.key]) ||
      row.id !== keys[item.key] ||
      (item.id !== null && row.id !== item.id) ||
      row.text !== item.text ||
      row.completed !== item.completed ||
      row.parent_id !==
        (item.parent_key === null ? null : keys[item.parent_key])
    )
      throw new ApiError(0);
  }
  return { ...data, keys: keys as Record<string, string> };
}
