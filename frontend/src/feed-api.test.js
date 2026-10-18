import { afterEach, expect, test, vi } from "vitest";
import {
  ApiError,
  ownerSession,
  signIn,
  signOut,
  projects,
  sessions,
  history,
} from "./feed-api";
import { csrf, message, session, page, response } from "../test/support";
afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
});
const signal = () => new AbortController().signal;

test("browser credentials, cache policy, password body and fresh logout CSRF", async () => {
  const fetch = vi
    .fn()
    .mockResolvedValueOnce(response(null, 204))
    .mockResolvedValueOnce(response({ authenticated: true, csrf_token: csrf }))
    .mockResolvedValueOnce(response(null, 204));
  vi.stubGlobal("fetch", fetch);
  await signIn("a private test password");
  await signOut();
  expect(fetch.mock.calls[0][1]).toMatchObject({
    method: "POST",
    body: JSON.stringify({ password: "a private test password" }),
    credentials: "same-origin",
    cache: "no-store",
  });
  expect(fetch.mock.calls[0][1].headers.Authorization).toBeUndefined();
  expect(fetch.mock.calls[2]).toEqual([
    "/api/owner/logout",
    expect.objectContaining({
      method: "POST",
      headers: { "X-Silkboard-CSRF": csrf },
    }),
  ]);
});
test("validates exact IDs and bounded pages with encoded cursors", async () => {
  const fetch = vi
    .fn()
    .mockResolvedValueOnce(response(page([{ id: "silkboard" }], "silkboard")))
    .mockResolvedValueOnce(response(page([session], "1")))
    .mockResolvedValueOnce(
      response(page([{ ...message, sequence: "9223372036854775807" }])),
    );
  vi.stubGlobal("fetch", fetch);
  expect((await projects("other", signal())).items[0].id).toBe("silkboard");
  expect((await sessions("silkboard", "123", signal())).next_cursor).toBe("1");
  expect(
    (await history("silkboard", "1", null, signal())).items[0].sequence,
  ).toBe("9223372036854775807");
  expect(fetch.mock.calls[0][0]).toContain("?after=other");
  expect(fetch.mock.calls[1][0]).toContain("?before=123");
});
test.each([
  null,
  {},
  [],
  { items: [], next_cursor: "1" },
  { items: Array(21).fill({ id: "silkboard" }), next_cursor: null },
  page([{ id: "WRONG" }]),
  page([], "bad/slash"),
])("rejects malformed project page %j", async (body) => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(response(body)));
  await expect(projects(null, signal())).rejects.toBeInstanceOf(ApiError);
});
test.each([
  {},
  { authenticated: false, csrf_token: csrf },
  { authenticated: true, csrf_token: "invalid" },
])("rejects invalid session metadata", async (body) => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(response(body)));
  await expect(ownerSession()).rejects.toBeInstanceOf(ApiError);
});
test.each(["0", "01", "-1", "9223372036854775808", 1])(
  "rejects invalid or lossy message sequences %j",
  async (sequence) => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(response(page([{ ...message, sequence }]))),
    );
    await expect(
      history("silkboard", "1", "2", signal()),
    ).rejects.toBeInstanceOf(ApiError);
  },
);
test.each([
  { received_at: -1 },
  { received_at: 8640000000000001 },
  { message: "" },
  { event_id: "" },
])("rejects malformed history records", async (changes) => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(response(page([{ ...message, ...changes }]))),
  );
  await expect(
    history("silkboard", "1", null, signal()),
  ).rejects.toBeInstanceOf(ApiError);
});
test.each([
  { id: "x" },
  { hostname: "" },
  { tool: "" },
  { session_id: "" },
  { subagent_id: 123 },
  { parent_session_id: 123 },
  { session_name: "" },
  { latest: {} },
])("rejects malformed agent identity", async (changes) => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(response(page([{ ...session, ...changes }]))),
  );
  await expect(sessions("silkboard", null, signal())).rejects.toBeInstanceOf(
    ApiError,
  );
});
test("propagates authorization failures without reading a server error body", async () => {
  const result = response({}, 401);
  result.json = vi.fn();
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(result));
  await expect(ownerSession()).rejects.toMatchObject({ status: 401 });
  expect(result.json).not.toHaveBeenCalled();
});
test("bounds stalled requests and forwards cancellation", async () => {
  vi.useFakeTimers();
  const fetch = vi.fn(
    (_path, { signal }) =>
      new Promise((_resolve, reject) =>
        signal.addEventListener("abort", () => reject(new Error("aborted"))),
      ),
  );
  vi.stubGlobal("fetch", fetch);
  const result = ownerSession().catch((error) => error);
  await vi.advanceTimersByTimeAsync(10000);
  expect(await result).toBeInstanceOf(Error);
  const controller = new AbortController();
  const second = ownerSession(controller.signal).catch((error) => error);
  controller.abort();
  expect(await second).toBeInstanceOf(Error);
});
test("ignores late mocked responses after cancellation and supports already-aborted signals", async () => {
  const controller = new AbortController();
  controller.abort();
  vi.stubGlobal(
    "fetch",
    vi
      .fn()
      .mockResolvedValue(response({ authenticated: true, csrf_token: csrf })),
  );
  await expect(ownerSession(controller.signal)).rejects.toMatchObject({
    status: 0,
  });
});
