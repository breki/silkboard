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
  window.history.replaceState(null, "", "/");
});
async function submit() {
  const input = await screen.findByLabelText("Owner password");
  await fireEvent.input(input, {
    target: { value: "a private test password" },
  });
  await fireEvent.submit(input.closest("form"));
}

test("keeps release status off the starting view and links to its own page", async () => {
  vi.stubGlobal("fetch", vi.fn(routes()));
  render(App, { eventsPage: true });
  await screen.findByLabelText("Owner password");
  expect(screen.queryByLabelText("Application status")).toBeNull();
  await fireEvent.click(
    screen.getByRole("button", { name: "Navigation menu" }),
  );
  expect(
    screen
      .getByRole("link", { name: "Application status" })
      .getAttribute("href"),
  ).toBe("/status");
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
  render(OwnerFeed, { eventsPage: true });
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
    render(OwnerFeed, { eventsPage: true });
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
  render(OwnerFeed, { eventsPage: true });
  await submit();
  await screen.findByText("Fix the tests");
  await fireEvent.click(
    screen.getByRole("button", { name: "Navigation menu" }),
  );
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
  render(OwnerFeed, { eventsPage: true });
  await screen.findByText("Fix the tests");
  revoked = true;
  await fireEvent.click(
    screen.getByRole("button", { name: "Navigation menu" }),
  );
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
  render(OwnerFeed, { eventsPage: true });
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
  const view = render(OwnerFeed, { eventsPage: true });
  await screen.findByText("Loading events…");
  await waitFor(() => expect(resolve).toBeTypeOf("function"));
  await fireEvent.click(
    screen.getByRole("button", { name: "Navigation menu" }),
  );
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
  const view = render(OwnerFeed, { eventsPage: true });
  expect(screen.getByRole("status").textContent).toContain("Checking");
  view.unmount();
  expect(signal.aborted).toBe(true);
});

test("status is separate and keeps logout recovery", async () => {
  const fetch = vi.fn(
    routes({
      "/api/owner/session": response({ authenticated: true, csrf_token: csrf }),
      "/api/owner/logout": response({}, 503),
    }),
  );
  vi.stubGlobal("fetch", fetch);
  render(App, { statusPage: true });
  await screen.findByText("Healthy");
  expect(screen.queryByRole("table")).toBeNull();
  expect(fetch.mock.calls.some(([path]) => path === "/api/owner/events")).toBe(
    false,
  );
  await fireEvent.click(
    screen.getByRole("button", { name: "Navigation menu" }),
  );
  await fireEvent.click(
    await screen.findByRole("button", { name: "Sign out" }),
  );
  await screen.findByRole("button", { name: "Retry sign out" });
  expect(screen.getByText("Healthy")).toBeTruthy();
  expect(screen.queryByText("Agent progress")).toBeNull();
});
test("signed-out status needs no password and links back to events", async () => {
  vi.stubGlobal("fetch", vi.fn(routes()));
  render(App, { statusPage: true });
  await screen.findByText("Healthy");
  expect(screen.queryByLabelText("Owner password")).toBeNull();
  await fireEvent.click(
    screen.getByRole("button", { name: "Navigation menu" }),
  );
  expect(
    screen.getByRole("link", { name: "Events" }).getAttribute("href"),
  ).toBe("/events");
  expect(screen.queryByRole("button", { name: "Sign out" })).toBeNull();
});

test("credential expiry on status clears owner actions and offers sign-in navigation", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(
      routes({
        "/api/owner/session": response({
          authenticated: true,
          csrf_token: csrf,
        }),
        "/api/owner/projects/silkboard/publishers": response({}, 401),
      }),
    ),
  );
  render(App, { statusPage: true });
  await screen.findByText("Healthy");
  await fireEvent.click(
    screen.getByRole("button", { name: "Navigation menu" }),
  );
  await fireEvent.click(
    await screen.findByRole("button", { name: "Manage publisher credentials" }),
  );
  await fireEvent.input(screen.getByLabelText("Credential project"), {
    target: { value: "silkboard" },
  });
  await fireEvent.click(
    screen.getByRole("button", { name: "Load credentials" }),
  );
  await screen.findByText("Return to sign in");
  expect(screen.queryByLabelText("Credential project")).toBeNull();
  expect(screen.getByText("Healthy")).toBeTruthy();
  await fireEvent.click(
    screen.getByRole("button", { name: "Navigation menu" }),
  );
  expect(screen.queryByRole("button", { name: "Sign out" })).toBeNull();
});

test.each(["status", "invalid plan"])(
  "idle %s session revocation removes a disclosed token without another action",
  async (view) => {
    vi.useFakeTimers();
    let revoked = false;
    const token = "b".repeat(64);
    const fetch = vi.fn(
      routes({
        "/api/owner/session": () =>
          response(
            { authenticated: true, csrf_token: csrf },
            revoked ? 401 : 200,
          ),
        "/api/owner/projects/silkboard/publishers": (options) =>
          options.method === "POST"
            ? response({ token }, 201)
            : response({
                items: [{ label: "dev", revoked: false }],
                next_cursor: null,
              }),
      }),
    );
    vi.stubGlobal("fetch", fetch);
    if (view === "invalid plan")
      window.history.replaceState(null, "", "/plans?bad=1");
    render(App, {
      statusPage: view === "status",
      plansPage: view === "invalid plan",
    });
    await vi.advanceTimersByTimeAsync(0);
    await fireEvent.click(
      screen.getByRole("button", { name: "Navigation menu" }),
    );
    await fireEvent.click(
      screen.getByRole("button", { name: "Manage publisher credentials" }),
    );
    await fireEvent.input(screen.getByLabelText("Credential project"), {
      target: { value: "silkboard" },
    });
    await fireEvent.input(screen.getByLabelText("Installation label"), {
      target: { value: "dev" },
    });
    await fireEvent.submit(
      screen.getByLabelText("Installation label").closest("form"),
    );
    await vi.advanceTimersByTimeAsync(0);
    expect(screen.getByLabelText("New publisher token").value).toBe(token);
    revoked = true;
    await vi.advanceTimersByTimeAsync(10000);
    expect(screen.queryByLabelText("New publisher token")).toBeNull();
    expect(screen.queryByLabelText("Credential project")).toBeNull();
    expect(screen.getByRole("alert").textContent).toContain("session expired");
    if (view === "status") expect(screen.getByText("Healthy")).toBeTruthy();
    await fireEvent.click(
      screen.getByRole("button", { name: "Navigation menu" }),
    );
    expect(screen.queryByRole("button", { name: "Sign out" })).toBeNull();
    expect(
      fetch.mock.calls.some(([path]) => path === "/api/owner/events"),
    ).toBe(false);
  },
);

test("status session checks pause while hidden, tolerate outages and cancel on teardown", async () => {
  vi.useFakeTimers();
  const hidden = vi.spyOn(document, "hidden", "get").mockReturnValue(true);
  let count = 0,
    pendingSignal;
  const fetch = vi.fn(
    routes({
      "/api/owner/session": (options) => {
        count++;
        if (count === 4) {
          pendingSignal = options.signal;
          return new Promise(() => {});
        }
        return response(
          { authenticated: true, csrf_token: csrf },
          count === 2 ? 503 : 200,
        );
      },
    }),
  );
  vi.stubGlobal("fetch", fetch);
  const view = render(App, { statusPage: true });
  await vi.advanceTimersByTimeAsync(0);
  expect(count).toBe(1);
  await vi.advanceTimersByTimeAsync(10000);
  expect(count).toBe(1);
  hidden.mockReturnValue(false);
  document.dispatchEvent(new Event("visibilitychange"));
  await vi.advanceTimersByTimeAsync(0);
  expect(count).toBe(2);
  expect(screen.getByText("Healthy")).toBeTruthy();
  await vi.advanceTimersByTimeAsync(10000);
  expect(count).toBe(3);
  await vi.advanceTimersByTimeAsync(10000);
  expect(pendingSignal.aborted).toBe(false);
  view.unmount();
  expect(pendingSignal.aborted).toBe(true);
});

test("plan deep links survive login and private plan data clears on logout", async () => {
  const { detail, task } = await import("../test/plans");
  const value = detail();
  window.history.replaceState(null, "", `/plans/silkboard/${task}?sequence=1`);
  let signed = false;
  vi.stubGlobal(
    "fetch",
    vi.fn(
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
          signed = false;
          return response(null, 204);
        },
        [`/api/owner/projects/silkboard/plans/${task}`]: response(value),
        [`/api/owner/projects/silkboard/plans/${task}?sequence=1`]:
          response(value),
      }),
    ),
  );
  render(App, { plansPage: true });
  await submit();
  await screen.findByRole("region", { name: "Task prerequisite graph" });
  expect(window.location.search).toBe("?sequence=1");
  expect(
    screen.queryByRole("table", { name: "Events from all projects" }),
  ).toBeNull();
  await fireEvent.click(
    screen.getByRole("button", { name: "Navigation menu" }),
  );
  await fireEvent.click(
    screen.getByRole("button", { name: "Sign out", exact: true }),
  );
  await screen.findByLabelText("Owner password");
  expect(screen.queryByText(value.snapshot.title)).toBeNull();
  window.history.replaceState(null, "", "/");
});

test("ordinary owner entry starts on Agents without replacing explicit views", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(
      routes({
        "/api/owner/session": response({
          authenticated: true,
          csrf_token: csrf,
        }),
        "/api/owner/agents": response(page([])),
        "/api/owner/events": response(page([])),
      }),
    ),
  );
  render(OwnerFeed);
  expect(
    await screen.findByRole("heading", { name: "Agents", exact: true }),
  ).toBeTruthy();
  expect(
    screen
      .getByRole("link", { name: "Agents", exact: true })
      .getAttribute("href"),
  ).toBe("/");
  expect(
    screen
      .getByRole("link", { name: "Events", exact: true })
      .getAttribute("href"),
  ).toBe("/events");
});
test("Todos sign-out guards an unsaved draft and forced expiry retains it privately", async () => {
  const { clearRetainedDraft } = await import("./todo-document-session");
  clearRetainedDraft();
  Range.prototype.getClientRects = () => [];
  Range.prototype.getBoundingClientRect = () => new DOMRect();
  let expired = false;
  const fetch = vi.fn(
    routes({
      "/api/owner/session": () =>
        response(
          expired ? {} : { authenticated: true, csrf_token: csrf },
          expired ? 401 : 200,
        ),
      "/api/owner/todos": response({
        revision: "0",
        items: [
          { id: "1", parent_id: null, text: "Private task", completed: false },
        ],
      }),
      "/api/owner/todos/document": () => response({}, expired ? 401 : 503),
      "/api/owner/logout": response(null, 204),
    }),
  );
  vi.stubGlobal("fetch", fetch);
  window.history.replaceState(null, "", "/todos");
  window.dispatchEvent(new Event("popstate"));
  render(OwnerFeed, { todosPage: true });
  await screen.findByRole("checkbox", { name: "Private task" });
  await fireEvent.click(screen.getByRole("checkbox", { name: "Private task" }));
  await fireEvent.click(
    screen.getByRole("button", { name: "Navigation menu" }),
  );
  const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
  await fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
  expect(confirm).toHaveBeenCalledOnce();
  expect(screen.getByRole("checkbox", { name: "Private task" })).toBeTruthy();
  expired = true;
  await screen.findByLabelText("Owner password");
  await screen.findByText(
    "Your unsaved Todos draft is kept in this tab. Sign in to recover it.",
  );
  expect(screen.queryByText("Private task")).toBeNull();
  const event = new Event("beforeunload", { cancelable: true });
  window.dispatchEvent(event);
  expect(event.defaultPrevented).toBe(true);
  cleanup();
  clearRetainedDraft();
});
test("explicit Todos sign-out discards only after owner confirmation", async () => {
  const { clearRetainedDraft } = await import("./todo-document-session");
  clearRetainedDraft();
  Range.prototype.getClientRects = () => [];
  Range.prototype.getBoundingClientRect = () => new DOMRect();
  vi.stubGlobal(
    "fetch",
    vi.fn(
      routes({
        "/api/owner/session": response({
          authenticated: true,
          csrf_token: csrf,
        }),
        "/api/owner/todos": response({
          revision: "0",
          items: [{ id: "1", parent_id: null, text: "Task", completed: false }],
        }),
        "/api/owner/logout": response(null, 204),
      }),
    ),
  );
  render(OwnerFeed, { todosPage: true });
  await screen.findByRole("checkbox", { name: "Task" });
  await fireEvent.click(screen.getByRole("checkbox", { name: "Task" }));
  vi.spyOn(window, "confirm").mockReturnValue(true);
  await fireEvent.click(
    screen.getByRole("button", { name: "Navigation menu" }),
  );
  await fireEvent.click(screen.getByRole("button", { name: "Sign out" }));
  await screen.findByLabelText("Owner password");
  expect(screen.queryByText(/draft is kept/)).toBeNull();
  cleanup();
  clearRetainedDraft();
});
