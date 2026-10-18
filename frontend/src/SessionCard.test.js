import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, expect, test, vi } from "vitest";
import SessionCard from "./SessionCard.svelte";
import { message, session, page, response, routes } from "../test/support";
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
const props = () => ({
  project: "silkboard",
  session,
  tick: 0,
  open: true,
  ontoggle: vi.fn(),
  onexpire: vi.fn(),
});
test("renders hostile latest/history text without markup and preserves timestamps", async () => {
  const hostile = '<img src=x onerror="window.hacked=true">';
  vi.stubGlobal(
    "fetch",
    vi.fn(
      routes({
        "/api/owner/projects/silkboard/sessions/1/events": response(
          page([{ ...message, message: hostile }]),
        ),
      }),
    ),
  );
  const view = render(SessionCard, {
    props: {
      ...props(),
      session: { ...session, latest: { ...message, message: hostile } },
    },
  });
  await screen.findByRole("heading", { name: "Message history" });
  await screen.findAllByText(hostile);
  expect(view.container.querySelector("img,script")).toBeNull();
  expect(view.container.querySelector("time").getAttribute("datetime")).toBe(
    new Date(message.received_at).toISOString(),
  );
});
test("history can retry, page earlier and return to latest", async () => {
  let fail = true;
  vi.stubGlobal(
    "fetch",
    vi.fn(
      routes({
        "/api/owner/projects/silkboard/sessions/1/events": () =>
          fail ? response({}, 503) : response(page([message], "1")),
        "/api/owner/projects/silkboard/sessions/1/events?before=1": response(
          page([]),
        ),
      }),
    ),
  );
  render(SessionCard, { props: props() });
  await screen.findByRole("alert");
  fail = false;
  await fireEvent.click(screen.getByRole("button", { name: "Retry history" }));
  await screen.findByRole("button", { name: "Earlier messages" });
  await fireEvent.click(
    screen.getByRole("button", { name: "Earlier messages" }),
  );
  await screen.findByText("No messages on this page.");
  await fireEvent.click(
    screen.getByRole("button", { name: "Latest messages" }),
  );
  await screen.findByRole("button", { name: "Earlier messages" });
});
test("expired access is delegated, fallback labels are readable and closing aborts a read", async () => {
  const values = props();
  vi.stubGlobal(
    "fetch",
    vi.fn(
      routes({
        "/api/owner/projects/silkboard/sessions/1/events": response({}, 401),
      }),
    ),
  );
  const view = render(SessionCard, {
    props: { ...values, session: { ...session, session_name: null } },
  });
  await new Promise((done) => setTimeout(done, 0));
  expect(values.onexpire).toHaveBeenCalledOnce();
  expect(screen.getByRole("heading", { name: "codex session" })).toBeTruthy();
  view.unmount();
  let signal;
  vi.stubGlobal(
    "fetch",
    vi.fn((_path, options) => {
      signal = options.signal;
      return new Promise(() => {});
    }),
  );
  const pending = render(SessionCard, { props: props() });
  await screen.findByRole("status");
  await pending.rerender({ open: false });
  expect(signal.aborted).toBe(true);
});
