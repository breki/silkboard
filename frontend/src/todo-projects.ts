import { ApiError, projects } from "./feed-api";
import type { TaskRow } from "./todo-editor-model";

export interface ProjectTag {
  project: string;
  from: number;
  to: number;
}
export interface ProjectMention {
  query: string;
  from: number;
  to: number;
}
export function tagsIn(text: string, known: readonly string[]): ProjectTag[] {
  const names = new Set(known);
  return [...text.matchAll(/(?:^|[\s(])@([a-z0-9-]+)(?=$|[\s),.!?;:\]])/g)]
    .filter((match) => names.has(match[1]))
    .map((match) => ({
      project: match[1],
      from: match.index! + match[0].indexOf("@"),
      to: match.index! + match[0].length,
    }));
}
export function mentionAt(text: string, offset: number): ProjectMention | null {
  const match = /(?:^|[\s(])@([a-z0-9-]*)$/.exec(text.slice(0, offset));
  return match
    ? {
        query: match[1],
        from: offset - match[1].length - 1,
        to: offset + /^[a-z0-9-]*/.exec(text.slice(offset))![0].length,
      }
    : null;
}
export function assignProject(
  text: string,
  project: string,
  known: readonly string[],
  mention?: ProjectMention | null,
) {
  const tags = tagsIn(text, known);
  const target = mention ?? tags[0] ?? { from: text.length, to: text.length };
  const ranges = [
    ...tags.filter((tag) => tag.to <= target.from || tag.from >= target.to),
    target,
  ].sort((a, b) => a.from - b.from);
  let result = "",
    cursor = 0,
    offset = 0;
  for (const range of ranges) {
    result += text.slice(cursor, range.from);
    if (range === target) {
      if (target.from === text.length && text && !/\s$/.test(text))
        result += " ";
      result += `@${project}`;
      offset = result.length;
    }
    cursor = range.to;
  }
  return { text: result + text.slice(cursor), offset };
}
export function tagValidation(rows: TaskRow[], known: readonly string[]) {
  return rows.some((row) => tagsIn(row.text, known).length > 1)
    ? "Use one project tag per item. Remove the extra @project tags to save."
    : "";
}
export function projectRows(
  rows: TaskRow[],
  known: readonly string[],
  project: string,
) {
  const matching = new Set<string>(),
    visible = new Set<string>(),
    ancestors: string[] = [];
  for (const row of rows) {
    ancestors.length = row.depth - 1;
    if (
      !project ||
      tagsIn(row.text, known).some((tag) => tag.project === project)
    ) {
      matching.add(row.key);
      for (const key of ancestors) visible.add(key);
      visible.add(row.key);
    }
    ancestors.push(row.key);
  }
  return { matching, visible };
}
export async function loadProjectNames(signal: AbortSignal, api = projects) {
  let after: string | null = null;
  const result: string[] = [];
  do {
    const page = await api(after, signal);
    if (
      page.items.some(
        (item, index) =>
          item.id <= (index ? page.items[index - 1].id : (after ?? "")),
      ) ||
      (page.next_cursor !== null && page.next_cursor !== page.items.at(-1)?.id)
    )
      throw new ApiError(0);
    result.push(...page.items.map((item) => item.id));
    after = page.next_cursor;
  } while (after !== null);
  return result;
}
