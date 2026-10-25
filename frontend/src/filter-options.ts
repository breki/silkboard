import { ApiError, page, projectId, request, type Page } from "./feed-api";
export type FilterView = "events" | "workflows";
export type ChoiceKind = "project" | "agent";
const agent = (value: unknown): value is string =>
  typeof value === "string" && /^[!-~]{1,256} [!-~]{1,256}$/.test(value);
export async function filterChoices(
  view: FilterView,
  kind: ChoiceKind,
  query: string,
  after: string | null,
  signal: AbortSignal,
): Promise<Page<string>> {
  const valid = kind === "project" ? projectId : agent;
  const params = new URLSearchParams({ view, kind, q: query });
  if (after !== null) params.set("after", after);
  const result = page(
    await request(`/api/owner/filter-options?${params}`, {}, signal),
    valid,
    valid,
  );
  if (
    result.items.some(
      (item, index) =>
        item <= (index ? result.items[index - 1] : (after ?? "")),
    ) ||
    (result.next_cursor !== null && result.next_cursor !== result.items.at(-1))
  )
    throw new ApiError(0);
  return result;
}
