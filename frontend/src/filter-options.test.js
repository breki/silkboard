import { afterEach, expect, test, vi } from "vitest";
import { filterChoices } from "./filter-options";
import { ApiError } from "./feed-api";
import { page, response } from "../test/support";
afterEach(() => vi.unstubAllGlobals());
const signal = () => new AbortController().signal;
test("encodes searches and cursors and accepts bounded sorted choices", async () => {
  const fetch = vi.fn(async () =>
    response(page(["codex dev", "codex host"], "codex host")),
  );
  vi.stubGlobal("fetch", fetch);
  expect(
    (await filterChoices("events", "agent", "c & %", null, signal())).items,
  ).toHaveLength(2);
  expect(fetch.mock.calls[0][0]).toContain("q=c+%26+%25");
  fetch.mockResolvedValue(response(page(["zebra"])));
  expect(
    (await filterChoices("workflows", "project", "", "alpha", signal())).items,
  ).toEqual(["zebra"]);
  expect(fetch.mock.calls[1][0]).toContain("after=alpha");
});
test.each([
  page(["BAD"]),
  page(["bravo", "alpha"]),
  page(["alpha", "alpha"]),
  page(["alpha"], "bravo"),
  page(["alpha"], "BAD"),
])("rejects malformed or inconsistent project choices", async (value) => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response(value)),
  );
  await expect(
    filterChoices("events", "project", "", null, signal()),
  ).rejects.toBeInstanceOf(ApiError);
});
test("rejects agent metadata outside the tool-host contract and nonadvancing pages", async () => {
  const fetch = vi.fn(async () => response(page(["codex bad host"])));
  vi.stubGlobal("fetch", fetch);
  await expect(
    filterChoices("events", "agent", "", null, signal()),
  ).rejects.toBeInstanceOf(ApiError);
  fetch.mockResolvedValue(response(page(["alpha"])));
  await expect(
    filterChoices("events", "project", "", "alpha", signal()),
  ).rejects.toBeInstanceOf(ApiError);
});
