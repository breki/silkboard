export interface ListFilters {
  event_id?: string;
  project?: string;
  agent?: string;
  from?: number;
  to?: number;
  state?: string;
  hostname?: string;
  tool?: string;
  session_id?: string;
  subagent_id?: string;
}

export function filterQuery(
  before: string | null,
  filters: ListFilters,
): string {
  const entries = Object.entries({ before, ...filters }).filter(
    ([key, value]) =>
      value !== null &&
      value !== undefined &&
      (value !== "" || key === "subagent_id"),
  );
  return entries.length
    ? `?${entries.map(([key, value]) => `${key}=${encodeURIComponent(String(value))}`).join("&")}`
    : "";
}

// eslint-disable-next-line no-unused-vars -- Parameter describes the callback type.
export type ApplyFilters = (filters: ListFilters) => void;

// eslint-disable-next-line no-unused-vars -- Parameter describes the callback type.
export type SelectChoice = (value: string) => void;
