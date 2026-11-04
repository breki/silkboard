import { expect, test } from "vitest";
import { parseLocalEndpoint, localFields, presetRange } from "./time-range";

test("custom endpoints require real local dates and 24-hour times", () => {
  const stamp = new Date(2026, 10, 3, 9, 15).getTime();
  expect(parseLocalEndpoint("2026-11-03", "09:15")).toBe(stamp);
  expect(localFields(stamp)).toEqual({ date: "2026-11-03", time: "09:15" });
  expect(parseLocalEndpoint("", "")).toBeUndefined();
  for (const [date, time] of [
    ["2026-02-30", "09:00"],
    ["2026-11-03", "25:00"],
    ["2026-11-03", "9:00"],
    ["2026-11-03", ""],
    ["1969-01-01", "00:00"],
    ["2026-13-01", "09:00"],
  ]) {
    expect(() => parseLocalEndpoint(date, time)).toThrow();
  }
});
test("presets use the actual selection time with bounded elapsed durations", () => {
  const now = Date.UTC(2026, 10, 3, 12);
  expect(presetRange("all", now)).toEqual({});
  expect(presetRange("hour", now)).toEqual({ from: now - 3600000, to: now });
  expect(presetRange("day", now)).toEqual({ from: now - 86400000, to: now });
  expect(presetRange("week", now)).toEqual({ from: now - 604800000, to: now });
});
test("nonexistent local daylight-saving times are rejected rather than silently shifted", () => {
  const previous = process.env.TZ;
  try {
    process.env.TZ = "America/New_York";
    expect(() => parseLocalEndpoint("2026-03-08", "02:30")).toThrow();
    expect(localFields(parseLocalEndpoint("2026-03-08", "01:30"))).toEqual({
      date: "2026-03-08",
      time: "01:30",
    });
  } finally {
    if (previous === undefined) delete process.env.TZ;
    else process.env.TZ = previous;
  }
});
