import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/svelte";
import { afterEach, expect, test, vi } from "vitest";
import OwnerSetup from "./OwnerSetup.svelte";
import App from "./App.svelte";
import { takeSetupLink } from "./setup-link";
import { response } from "../test/support";

const credential = "a".repeat(64);
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
const ready = () => response({ state: "available" });
test("a direct setup page without an entry credential cannot register", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(() => Promise.resolve(ready())),
  );
  render(App, { setupPage: true });
  await screen.findByText(/Open the private setup link/);
  expect(
    screen.queryByRole("button", { name: "Create owner account" }),
  ).toBeNull();
});
test("accepts a new private link opened in the same setup tab", async () => {
  window.history.replaceState(null, "", "/setup");
  start(undefined, { takeCredential: () => "" });
  await screen.findByText(/Open the private setup link/);
  window.history.replaceState(null, "", `/setup#setup=${credential}`);
  window.dispatchEvent(new Event("hashchange"));
  await screen.findByLabelText("New owner password");
  expect(window.location.hash).toBe("");
});
function start(reply = () => response(null, 204), props = {}) {
  const fetch = vi.fn((path, options) =>
    Promise.resolve(options?.method === "POST" ? reply() : ready()),
  );
  vi.stubGlobal("fetch", fetch);
  const component = render(OwnerSetup, {
    takeCredential: () => credential,
    ...props,
  });
  return { fetch, ...component };
}
async function submit(confirm = "private setup passphrase") {
  const input = await screen.findByLabelText("New owner password");
  await fireEvent.input(input, {
    target: { value: "private setup passphrase" },
  });
  await fireEvent.input(screen.getByLabelText("Confirm password"), {
    target: { value: confirm },
  });
  await fireEvent.submit(input.closest("form"));
}
test("scrubs fragments and query before returning only a valid setup credential", () => {
  const history = { replaceState: vi.fn() };
  expect(
    takeSetupLink({ pathname: "/", hash: `#setup=${credential}` }, history),
  ).toBe("");
  expect(history.replaceState).not.toHaveBeenCalled();
  expect(
    takeSetupLink(
      { pathname: "/setup", hash: `#setup=${credential}` },
      history,
    ),
  ).toBe(credential);
  expect(history.replaceState).toHaveBeenCalledWith(null, "", "/setup");
  for (const hash of [
    "#other=secret",
    "#setup=short",
    `#setup=${credential}&extra=1`,
    `#setup=${"A".repeat(64)}`,
  ])
    expect(takeSetupLink({ pathname: "/setup", hash }, history)).toBe("");
});
test("registers without rendering or retaining the link in DOM", async () => {
  const { fetch, container } = start();
  await submit();
  await screen.findByText("Owner registered. Your setup link has been used.");
  const [path, options] = fetch.mock.calls.find(([, o]) => o.method === "POST");
  expect(path).toBe("/api/auth/setup");
  expect(JSON.parse(options.body)).toEqual({
    credential,
    password: "private setup passphrase",
  });
  expect(options.headers["X-Silkboard-Setup"]).toBe("1");
  expect(container.innerHTML).not.toContain(credential);
  expect(screen.queryByLabelText("New owner password")).toBeNull();
});
test("requires matching confirmation before sending", async () => {
  const { fetch } = start();
  await submit("different");
  expect((await screen.findByRole("alert")).textContent).toContain(
    "do not match",
  );
  expect(fetch.mock.calls.filter(([, o]) => o.method === "POST")).toHaveLength(
    0,
  );
});
test.each([
  [401, "expired"],
  [410, "already complete"],
  [429, "Wait a minute"],
  [400, "12–1,024"],
  [503, "could not be confirmed"],
])(
  "explains registration error %s and clears password fields",
  async (status, text) => {
    start(() => response({}, status));
    await submit();
    await screen.findByText(new RegExp(text));
    for (const input of document.querySelectorAll("input"))
      expect(input.value).toBe("");
  },
);
test.each(["waiting", "closed"])("explains setup state %s", async (state) => {
  vi.stubGlobal(
    "fetch",
    vi.fn(() => Promise.resolve(response({ state }))),
  );
  render(OwnerSetup, { takeCredential: () => credential });
  await screen.findByText(state === "closed" ? /already complete/ : /expired/);
});
test("missing link, page hide and remount never reuse credentials", async () => {
  const first = start();
  await screen.findByLabelText("New owner password");
  window.dispatchEvent(new Event("pagehide"));
  await screen.findByText(/Open the private setup link/);
  first.unmount();
  start(undefined, { takeCredential: () => "" });
  await screen.findByText(/Open the private setup link/);
});
test("retries unavailable status and rejects malformed responses", async () => {
  let bad = true;
  vi.stubGlobal(
    "fetch",
    vi.fn(() =>
      Promise.resolve(
        response(bad ? { state: "invalid" } : { state: "available" }),
      ),
    ),
  );
  render(App, { setupPage: true, takeSetupCredential: () => credential });
  await fireEvent.click(
    await screen.findByRole("button", { name: "Retry setup" }),
  );
  await screen.findByRole("button", { name: "Retry setup" });
  bad = false;
  await fireEvent.click(screen.getByRole("button", { name: "Retry setup" }));
  await screen.findByLabelText("New owner password");
  expect(screen.queryByText("Agent progress")).toBeNull();
});
test("unmount and pagehide abandon pending checks and submissions", async () => {
  let finish;
  vi.stubGlobal(
    "fetch",
    vi.fn(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    ),
  );
  const check = render(OwnerSetup, { takeCredential: () => credential });
  check.unmount();
  finish(response({ state: "available" }));
  await Promise.resolve();
  const pending = start(() => new Promise(() => {}));
  await submit();
  await screen.findByText("Creating your account…");
  window.dispatchEvent(new Event("pagehide"));
  await waitFor(() =>
    expect(screen.queryByText("Creating your account…")).toBeNull(),
  );
  pending.unmount();
});
