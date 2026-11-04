export interface ReceivedRange {
  from?: number;
  to?: number;
}
export type Period = "all" | "hour" | "day" | "week";
export const periods: { id: Period; label: string }[] = [
  { id: "all", label: "All time" },
  { id: "hour", label: "Last hour" },
  { id: "day", label: "Last 24 hours" },
  { id: "week", label: "Last 7 days" },
];
export function localFields(value: number) {
  const date = new Date(value);
  const pad = (n: number) => String(n).padStart(2, "0");
  return {
    date: `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`,
    time: `${pad(date.getHours())}:${pad(date.getMinutes())}`,
  };
}
export function parseLocalEndpoint(
  date: string,
  time: string,
): number | undefined {
  if (!date && !time) return undefined;
  if (
    !/^\d{4}-\d{2}-\d{2}$/.test(date) ||
    !/^([01]\d|2[0-3]):[0-5]\d$/.test(time)
  )
    throw new Error("Invalid endpoint");
  const value = new Date(`${date}T${time}:00`).getTime();
  const fields = localFields(value);
  if (
    !Number.isSafeInteger(value) ||
    value < 0 ||
    fields.date !== date ||
    fields.time !== time
  )
    throw new Error("Invalid local time");
  return value;
}
export function presetRange(period: Period, now: number): ReceivedRange {
  if (period === "all") return {};
  const duration = { hour: 3600000, day: 86400000, week: 604800000 }[period];
  return { from: Math.max(0, now - duration), to: now };
}
