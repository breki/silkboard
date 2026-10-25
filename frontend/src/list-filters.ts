export interface ListFilters {
  project?: string;
  agent?: string;
  from?: number;
  to?: number;
  state?: string;
}

export function filterQuery(
  before: string | null,
  filters: ListFilters,
): string {
  const entries = Object.entries({ before, ...filters }).filter(
    ([, value]) => value !== null && value !== undefined && value !== "",
  );
  return entries.length
    ? `?${entries.map(([key, value]) => `${key}=${encodeURIComponent(String(value))}`).join("&")}`
    : "";
}

// eslint-disable-next-line no-unused-vars -- Parameter describes the callback type.
export type ApplyFilters = (filters: ListFilters) => void;

// eslint-disable-next-line no-unused-vars -- Parameter describes the callback type.
export type SelectChoice = (value: string) => void;
