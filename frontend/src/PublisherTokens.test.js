import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/svelte";
import { afterEach, expect, test, vi } from "vitest";
import PublisherTokens from "./PublisherTokens.svelte";
import OwnerFeed from "./OwnerFeed.svelte";
import { response } from "../test/support";
const token = "b".repeat(64),
  csrf = "c".repeat(64);
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});
function start(overrides = {}, component = PublisherTokens) {
  const onexpire = vi.fn();
  const fetch = vi.fn(async (path, options) => {
    if (path === "/api/owner/session")
      return response({ authenticated: true, csrf_token: csrf });
    if (path === "/api/owner/projects")
      return response({ items: [], next_cursor: null });
    if (options.method === "POST")
      return path.endsWith("/revoke")
        ? (overrides.revoke?.() ?? response(null, 204))
        : (overrides.issue?.() ?? response({ token }, 201));
    return (
      overrides.list?.(path) ??
      response({
        items: [{ label: "dev-vm", revoked: false }],
        next_cursor: null,
      })
    );
  });
  vi.stubGlobal("fetch", fetch);
  return {
    fetch,
    onexpire,
    ...render(component, component === OwnerFeed ? {} : { onexpire }),
  };
}
async function fields(project = "silkboard", label = "dev-vm") {
  await fireEvent.input(await screen.findByLabelText("Credential project"), {
    target: { value: project },
  });
  await fireEvent.input(screen.getByLabelText("Installation label"), {
    target: { value: label },
  });
}
async function issue() {
  await fields();
  await fireEvent.submit(
    screen.getByLabelText("Installation label").closest("form"),
  );
}
const postCalls = (fetch) =>
  fetch.mock.calls.filter(([, options]) => options.method === "POST");

test("issue discloses a token once; explicit dismissal removes it and uses CSRF", async () => {
  const { fetch } = start();
  await issue();
  expect((await screen.findByLabelText("New publisher token")).value).toBe(
    token,
  );
  expect(postCalls(fetch)[0][1].headers["X-Silkboard-CSRF"]).toBe(csrf);
  expect(JSON.parse(postCalls(fetch)[0][1].body)).toEqual({ label: "dev-vm" });
  expect(screen.getByRole("button", { name: "Issue token" }).disabled).toBe(
    true,
  );
  await fireEvent.submit(
    screen.getByLabelText("Installation label").closest("form"),
  );
  expect(postCalls(fetch)).toHaveLength(1);
  await fireEvent.click(screen.getByRole("button", { name: "I’ve saved it" }));
  expect(screen.queryByLabelText("New publisher token")).toBeNull();
  expect(document.body.textContent).not.toContain(token);
});
test("a failed metadata refresh cannot turn confirmed issuance into an uncertain outcome", async () => {
  start({ list: () => response({}, 503) });
  await issue();
  expect((await screen.findByLabelText("New publisher token")).value).toBe(
    token,
  );
  await screen.findByText(
    /Token issued, but the credential list could not refresh/,
  );
  expect(document.body.textContent).not.toContain(
    "Issuance could not be confirmed",
  );
});
test("confirmed revocation remains visible when the following metadata refresh fails", async () => {
  let revoked = false;
  start({
    list: () =>
      revoked
        ? response({}, 503)
        : response({
            items: [{ label: "dev-vm", revoked: false }],
            next_cursor: null,
          }),
    revoke: () => {
      revoked = true;
      return response(null, 204);
    },
  });
  await fields();
  await fireEvent.click(
    screen.getByRole("button", { name: "Load credentials" }),
  );
  await fireEvent.click(
    await screen.findByRole("button", { name: "Revoke dev-vm" }),
  );
  await fireEvent.click(
    screen.getByRole("button", { name: "Confirm revocation" }),
  );
  await screen.findByText(/Credential revoked, but the list could not refresh/);
  await screen.findByText(/— Revoked/);
});
test("copies only on request and explains clipboard failure", async () => {
  const writeText = vi.fn().mockResolvedValue(undefined);
  vi.stubGlobal("navigator", { clipboard: { writeText } });
  start();
  await issue();
  await screen.findByLabelText("New publisher token");
  expect(writeText).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole("button", { name: "Copy token" }));
  await screen.findByText(/Copied. Install it privately/);
  expect(writeText).toHaveBeenCalledWith(token);
  writeText.mockRejectedValue(new Error("do not show raw error"));
  await fireEvent.click(screen.getByRole("button", { name: "Copy token" }));
  await screen.findByText(/Clipboard unavailable/);
  expect(document.body.textContent).not.toContain("raw error");
});
test("downloads a fixed filename via a temporary blob and releases it", async () => {
  const createObjectURL = vi.fn(() => "blob:private-local"),
    revokeObjectURL = vi.fn();
  vi.stubGlobal("URL", { createObjectURL, revokeObjectURL });
  const click = vi
    .spyOn(HTMLAnchorElement.prototype, "click")
    .mockImplementation(() => {});
  start();
  await issue();
  await screen.findByLabelText("New publisher token");
  await fireEvent.click(
    screen.getByRole("button", { name: "Download token file" }),
  );
  expect(click.mock.instances[0].download).toBe("silkboard-publisher.token");
  expect(createObjectURL.mock.calls[0][0].type).toBe("text/plain");
  expect(revokeObjectURL).toHaveBeenCalledWith("blob:private-local");
  expect(document.body.textContent).toContain("mode 0600");
  createObjectURL.mockImplementation(() => {
    throw new Error("unsupported");
  });
  await fireEvent.click(
    screen.getByRole("button", { name: "Download token file" }),
  );
  await screen.findByText(/Download unavailable/);
});
test.each([
  [409, "already exists"],
  [503, "could not be confirmed"],
  [200, "could not be confirmed"],
])("handles issuance error/invalid response %s", async (status, text) => {
  start({ issue: () => response({}, status) });
  await issue();
  await screen.findByText(new RegExp(text));
  expect(screen.queryByLabelText("New publisher token")).toBeNull();
});
test("loads safe metadata, confirms revocation and retains text-only labels", async () => {
  let revoked = false;
  const hostile = "<script>alert(1)</script>";
  const { fetch } = start({
    list: () =>
      response({ items: [{ label: hostile, revoked }], next_cursor: null }),
    revoke: () => {
      revoked = true;
      return response(null, 204);
    },
  });
  await fields();
  await fireEvent.click(
    screen.getByRole("button", { name: "Load credentials" }),
  );
  await screen.findByText(hostile);
  expect(document.querySelector(".token-manager script")).toBeNull();
  await fireEvent.click(
    screen.getByRole("button", { name: `Revoke ${hostile}` }),
  );
  await fireEvent.click(
    screen.getByRole("button", { name: "Cancel revocation" }),
  );
  expect(postCalls(fetch)).toHaveLength(0);
  await fireEvent.click(
    screen.getByRole("button", { name: `Revoke ${hostile}` }),
  );
  await fireEvent.click(
    screen.getByRole("button", { name: "Confirm revocation" }),
  );
  await screen.findByText(/Credential revoked/);
  await screen.findByText(/— Revoked/);
  expect(postCalls(fetch)[0][0]).toContain("/publishers/revoke");
});
test("pagination keeps the displayed project despite an edited draft input", async () => {
  const { fetch } = start({
    list: (path) =>
      response({
        items: [
          {
            label: path.includes("?after=") ? "second" : "first",
            revoked: true,
          },
        ],
        next_cursor: path.includes("?after=") ? null : "first",
      }),
  });
  await fields();
  await fireEvent.click(
    screen.getByRole("button", { name: "Load credentials" }),
  );
  await fireEvent.input(screen.getByLabelText("Credential project"), {
    target: { value: "different-project" },
  });
  await fireEvent.click(
    await screen.findByRole("button", { name: "More credentials" }),
  );
  await screen.findByText("second");
  expect(fetch.mock.calls.at(-1)[0]).toBe(
    "/api/owner/projects/silkboard/publishers?after=first",
  );
  await fireEvent.click(
    screen.getByRole("button", { name: "First credentials" }),
  );
  await screen.findByText("first");
  expect(fetch.mock.calls.at(-1)[0]).toBe(
    "/api/owner/projects/silkboard/publishers",
  );
});
test("validates fields and safe list responses", async () => {
  const { fetch } = start({
    list: () =>
      response({
        items: [{ label: "dev", revoked: false, token }],
        next_cursor: null,
      }),
  });
  await fireEvent.click(
    screen.getByRole("button", { name: "Load credentials" }),
  );
  await screen.findByText(/Use a project name/);
  expect(fetch).not.toHaveBeenCalled();
  await fields("BAD", "bad label");
  await fireEvent.submit(
    screen.getByLabelText("Installation label").closest("form"),
  );
  await screen.findByText(/Enter a valid project/);
  expect(fetch).not.toHaveBeenCalled();
  await fields();
  await fireEvent.click(
    screen.getByRole("button", { name: "Load credentials" }),
  );
  await screen.findByText(/Unable to load credentials/);
  expect(document.body.textContent).not.toContain(token);
});
test("losing owner authentication clears the manager through the existing owner flow", async () => {
  const { onexpire } = start({ issue: () => response({}, 401) });
  await issue();
  await waitFor(() => expect(onexpire).toHaveBeenCalledOnce());
  expect(screen.queryByLabelText("New publisher token")).toBeNull();
});
test("failed revocation has an explicit uncertain outcome", async () => {
  start({ revoke: () => response({}, 503) });
  await fields();
  await fireEvent.click(
    screen.getByRole("button", { name: "Load credentials" }),
  );
  await fireEvent.click(
    await screen.findByRole("button", { name: "Revoke dev-vm" }),
  );
  await fireEvent.click(
    screen.getByRole("button", { name: "Confirm revocation" }),
  );
  await screen.findByText(/Revocation could not be confirmed/);
});
test("pagehide removes a disclosed secret and ignores a pending clipboard result", async () => {
  let done;
  vi.stubGlobal("navigator", {
    clipboard: {
      writeText: () =>
        new Promise((resolve) => {
          done = resolve;
        }),
    },
  });
  start();
  await issue();
  await screen.findByLabelText("New publisher token");
  await fireEvent.click(screen.getByRole("button", { name: "Copy token" }));
  window.dispatchEvent(new Event("pagehide"));
  done();
  await waitFor(() =>
    expect(screen.queryByLabelText("New publisher token")).toBeNull(),
  );
  expect(document.body.textContent).not.toContain("Copied.");
});
test("a pending issue cannot disclose its result after navigation", async () => {
  let done;
  start({
    issue: () =>
      new Promise((resolve) => {
        done = resolve;
      }),
  });
  await issue();
  await waitFor(() => expect(done).toBeTypeOf("function"));
  window.dispatchEvent(new Event("pagehide"));
  done(response({ token }, 201));
  await waitFor(() =>
    expect(screen.queryByText("Updating credentials…")).toBeNull(),
  );
  expect(screen.queryByLabelText("New publisher token")).toBeNull();
});
test("owner opens and closes the manager without persisting a disclosed credential", async () => {
  start({}, OwnerFeed);
  await fireEvent.click(
    await screen.findByRole("button", { name: "Manage publisher credentials" }),
  );
  await issue();
  await screen.findByLabelText("New publisher token");
  await fireEvent.click(
    screen.getByRole("button", { name: "Close credential manager" }),
  );
  expect(screen.queryByLabelText("New publisher token")).toBeNull();
  await fireEvent.click(
    screen.getByRole("button", { name: "Manage publisher credentials" }),
  );
  expect(screen.getByLabelText("Credential project").value).toBe("");
});
