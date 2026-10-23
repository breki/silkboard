import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import OwnerFeed from "./OwnerFeed.svelte";
import App from "./App.svelte";
import { csrf, page, response, routes, feedEvent } from "../test/support";

beforeEach(() => vi.spyOn(document, "hidden", "get").mockReturnValue(false));
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  vi.useRealTimers();
});
async function submit() {
  const input = await screen.findByLabelText("Owner password");
  await fireEvent.input(input, {
    target: { value: "a private test password" },
  });
  await fireEvent.submit(input.closest("form"));
}

test("keeps public release status alongside the owner login", async () => {
  vi.stubGlobal("fetch", vi.fn(routes()));
  render(App);
  await screen.findByLabelText("Owner password");
  await screen.findByText("Healthy");
  expect(screen.getAllByRole("heading", { name: "Silkboard" })).toHaveLength(1);
});
test.each([
  [404, "Check again"],
  [503, "Retry sign in"],
])("recovers from initial status %s", async (status, button) => {
  let code = status;
  vi.stubGlobal(
    "fetch",
    vi.fn(routes({ "/api/owner/session": () => response({}, code) })),
  );
  render(OwnerFeed);
  await screen.findByRole("button", { name: button });
  code = 401;
  await fireEvent.click(screen.getByRole("button", { name: button }));
  await screen.findByLabelText("Owner password");
});
test.each([
  [401, "Password not accepted"],
  [429, "Wait a minute"],
  [503, "Unable to sign in"],
])(
  "handles rejected login %s without retaining password",
  async (code, text) => {
    vi.stubGlobal(
      "fetch",
      vi.fn(routes({ "/api/auth/login": response({}, code) })),
    );
    render(OwnerFeed);
    await submit();
    expect((await screen.findByRole("alert")).textContent).toContain(text);
    expect(screen.getByLabelText("Owner password").value).toBe("");
  },
);
test("signs in, clears private content during logout and retries an unconfirmed logout", async () => {
  let signed = false,
    fail = true;
  const fetch = vi.fn(
    routes({
      "/api/owner/session": () =>
        response(
          signed ? { authenticated: true, csrf_token: csrf } : {},
          signed ? 200 : 401,
        ),
      "/api/auth/login": () => {
        signed = true;
        return response(null, 204);
      },
      "/api/owner/logout": () => {
        if (fail) return response({}, 503);
        signed = false;
        return response(null, 204);
      },
    }),
  );
  vi.stubGlobal("fetch", fetch);
  render(OwnerFeed);
  await submit();
  await screen.findByText("Fix the tests");
  await fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
  await screen.findByRole("button", { name: "Retry sign out" });
  expect(screen.queryByText("Fix the tests")).toBeNull();
  fail = false;
  await fireEvent.click(screen.getByRole("button", { name: "Retry sign out" }));
  await screen.findByLabelText("Owner password");
  expect(
    fetch.mock.calls.find(([path]) => path === "/api/owner/logout")[1].headers,
  ).toEqual({ "X-Silkboard-CSRF": csrf });
});
test("already revoked session is truthfully signed out", async () => {
  let revoked = false;
  vi.stubGlobal(
    "fetch",
    vi.fn(
      routes({
        "/api/owner/session": () =>
          response(
            { authenticated: true, csrf_token: csrf },
            revoked ? 401 : 200,
          ),
      }),
    ),
  );
  render(OwnerFeed);
  await screen.findByText("Fix the tests");
  revoked = true;
  await fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
  await screen.findByLabelText("Owner password");
});
test("a poll detecting expired authentication removes the feed", async () => {
  vi.useFakeTimers();
  let expired = false;
  vi.stubGlobal(
    "fetch",
    vi.fn(
      routes({
        "/api/owner/session": response({
          authenticated: true,
          csrf_token: csrf,
        }),
        "/api/owner/events": () =>
          response(page([feedEvent]), expired ? 401 : 200),
      }),
    ),
  );
  render(OwnerFeed);
  await vi.advanceTimersByTimeAsync(0);
  expect(screen.getByText("Fix the tests")).toBeTruthy();
  expired = true;
  await vi.advanceTimersByTimeAsync(10000);
  expect(screen.queryByText("Fix the tests")).toBeNull();
  expect(screen.getByRole("alert").textContent).toContain("session expired");
});
test("late data cannot reappear after logout or unmount", async () => {
  let resolve;
  vi.stubGlobal(
    "fetch",
    vi.fn(
      routes({
        "/api/owner/session": response({
          authenticated: true,
          csrf_token: csrf,
        }),
        "/api/owner/logout": response(null, 204),
        "/api/owner/events": () =>
          new Promise((done) => {
            resolve = done;
          }),
      }),
    ),
  );
  const view = render(OwnerFeed);
  await screen.findByText("Loading events…");
  await waitFor(() => expect(resolve).toBeTypeOf("function"));
  await fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
  await screen.findByLabelText("Owner password");
  resolve(response(page([{ ...feedEvent, message: "Late private content" }])));
  await new Promise((done) => setTimeout(done, 0));
  expect(screen.queryByText("Late private content")).toBeNull();
  view.unmount();
});
test("teardown aborts a stalled initial session check", async () => {
  let signal;
  vi.stubGlobal(
    "fetch",
    vi.fn((_path, options) => {
      signal = options.signal;
      return new Promise(() => {});
    }),
  );
  const view = render(OwnerFeed);
  expect(screen.getByRole("status").textContent).toContain("Checking");
  view.unmount();
  expect(signal.aborted).toBe(true);
});
