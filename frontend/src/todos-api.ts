import { ApiError, object, ownerSession, positive, request } from "./feed-api";
export interface Todo {
  id: string;
  parent_id: string | null;
  text: string;
  completed: boolean;
}
// eslint-disable-next-line no-unused-vars -- Type-only parameter describes the item action.
export type TodoAction = (item: Todo) => void;
// eslint-disable-next-line no-unused-vars -- Type-only parameters describe checkbox focus restoration.
export type TodoCheck = (item: Todo, checkbox: HTMLInputElement) => void;
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
  let previous = BigInt(0);
  for (const item of value.items) {
    if (
      !object(item) ||
      !positive(item.id) ||
      BigInt(item.id) <= previous ||
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
    depths.set(item.id, depth);
    previous = BigInt(item.id);
  }
  return value as unknown as Checklist;
}
export async function loadTodos(signal: AbortSignal): Promise<Checklist> {
  return checklist(await request("/api/owner/todos", {}, signal));
}
export type TodoChange =
  | { kind: "add"; text: string; parent_id: string | null }
  | { kind: "edit"; id: string; text: string; completed: boolean }
  | { kind: "delete"; id: string };
export async function changeTodo(
  revision: string,
  change: TodoChange,
  signal: AbortSignal,
): Promise<Checklist> {
  const { csrf_token } = await ownerSession(signal);
  const { kind, ...fields } = change;
  const id = "id" in fields ? fields.id : undefined;
  const body =
    kind === "add"
      ? { revision, text: change.text, parent_id: change.parent_id }
      : kind === "edit"
        ? { revision, text: change.text, completed: change.completed }
        : { revision };
  return checklist(
    await request(
      `/api/owner/todos${id === undefined ? "" : `/${encodeURIComponent(id)}`}`,
      {
        method: kind === "add" ? "POST" : kind === "edit" ? "PATCH" : "DELETE",
        headers: {
          "Content-Type": "application/json",
          "X-Silkboard-CSRF": csrf_token,
        },
        body: JSON.stringify(body),
      },
      signal,
    ),
  );
}
