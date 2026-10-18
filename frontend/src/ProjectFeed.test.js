import { cleanup, fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import ProjectFeed from "./ProjectFeed.svelte";
import SessionList from "./SessionList.svelte";
import { page, response, routes, session } from "../test/support";
beforeEach(() => vi.spyOn(document, "hidden", "get").mockReturnValue(false));
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  vi.useRealTimers();
});

test("empty projects and sessions have clear states", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(routes({ "/api/owner/projects": response(page([])) })),
  );
  const view = render(ProjectFeed, { props: { onexpire: vi.fn() } });
  await screen.findByText(/No projects yet/);
  view.unmount();
  vi.stubGlobal(
    "fetch",
    vi.fn(
      routes({ "/api/owner/projects/silkboard/sessions": response(page([])) }),
    ),
  );
  render(SessionList, { props: { project: "silkboard", onexpire: vi.fn() } });
  await screen.findByText(/No sessions yet/);
});
test("project errors can retry and project pagination preserves the selected project", async () => {
  let fail = true;
  vi.stubGlobal(
    "fetch",
    vi.fn(
      routes({
        "/api/owner/projects": () =>
          fail
            ? response({}, 503)
            : response(page([{ id: "silkboard" }], "silkboard")),
        "/api/owner/projects?after=silkboard": response(
          page([{ id: "second" }]),
        ),
      }),
    ),
  );
  render(ProjectFeed, { props: { onexpire: vi.fn() } });
  await screen.findByRole("alert");
  fail = false;
  await fireEvent.click(screen.getByRole("button", { name: "Retry projects" }));
  await screen.findByLabelText("Project");
  await fireEvent.click(screen.getByRole("button", { name: "More projects" }));
  await screen.findByRole("option", { name: "second" });
  expect(screen.getByLabelText("Project").value).toBe("silkboard");
  await fireEvent.change(screen.getByLabelText("Project"), {
    target: { value: "second" },
  });
  await fireEvent.click(screen.getByRole("button", { name: "First projects" }));
  await screen.findByRole("option", { name: "silkboard" });
  expect(screen.getByLabelText("Project").value).toBe("second");
});
test.each(["response", "body"])(
  "teardown cancels a pending project %s without showing stale data",
  async (stage) => {
    let finish;
    const pending = new Promise((done) => {
      finish = done;
    });
    vi.stubGlobal(
      "fetch",
      vi.fn(() =>
        stage === "response"
          ? pending
          : { ok: true, status: 200, json: () => pending },
      ),
    );
    const expire = vi.fn();
    const view = render(ProjectFeed, { props: { onexpire: expire } });
    await screen.findByText("Loading projects…");
    await new Promise((done) => setTimeout(done, 0));
    view.unmount();
    finish(
      stage === "response"
        ? response(page([{ id: "silkboard" }]))
        : page([{ id: "silkboard" }]),
    );
    await new Promise((done) => setTimeout(done, 0));
    expect(screen.queryByLabelText("Project")).toBeNull();
    expect(expire).not.toHaveBeenCalled();
  },
);
test("expired project access returns control to owner login", async () => {
  const expire = vi.fn();
  vi.stubGlobal(
    "fetch",
    vi.fn(routes({ "/api/owner/projects": response({}, 401) })),
  );
  render(ProjectFeed, { props: { onexpire: expire } });
  await new Promise((done) => setTimeout(done, 0));
  expect(expire).toHaveBeenCalledOnce();
});
test("separate identities and one expanded history stay separate", async () => {
  const other = {
    ...session,
    id: "2",
    tool: "claude",
    session_id: "another-native",
    subagent_id: "child",
    parent_session_id: "parent",
  };
  vi.stubGlobal(
    "fetch",
    vi.fn(
      routes({
        "/api/owner/projects/silkboard/sessions": response(
          page([session, other]),
        ),
      }),
    ),
  );
  render(SessionList, { props: { project: "silkboard", onexpire: vi.fn() } });
  await screen.findByText("another-native");
  expect(screen.getByText("native-1")).toBeTruthy();
  expect(screen.getByText("child")).toBeTruthy();
  expect(screen.getByText("parent")).toBeTruthy();
  await fireEvent.click(
    screen.getAllByRole("button", { name: "View history" })[0],
  );
  await screen.findByRole("heading", { name: "Message history" });
  await fireEvent.click(screen.getByRole("button", { name: "View history" }));
  expect(
    screen.getAllByRole("heading", { name: "Message history" }),
  ).toHaveLength(1);
  await fireEvent.click(screen.getByRole("button", { name: "Hide history" }));
  expect(screen.queryByRole("heading", { name: "Message history" })).toBeNull();
});
test("session failures preserve displayed data and pagination stays bounded", async () => {
  let fail = true;
  vi.stubGlobal(
    "fetch",
    vi.fn(
      routes({
        "/api/owner/projects/silkboard/sessions": () =>
          fail ? response({}, 503) : response(page([session], "1")),
        "/api/owner/projects/silkboard/sessions?before=1": response(page([])),
      }),
    ),
  );
  render(SessionList, { props: { project: "silkboard", onexpire: vi.fn() } });
  await screen.findByRole("alert");
  fail = false;
  await fireEvent.click(screen.getByRole("button", { name: "Retry sessions" }));
  await screen.findByText("Fix the tests");
  await fireEvent.click(
    screen.getByRole("button", { name: "Earlier sessions" }),
  );
  await screen.findByText(/No sessions yet/);
  await fireEvent.click(
    screen.getByRole("button", { name: "Latest sessions" }),
  );
  await screen.findByText("Fix the tests");
});
test("session authentication expiry and cancelled old-project data cannot leak", async () => {
  const expire = vi.fn();
  vi.stubGlobal(
    "fetch",
    vi.fn(
      routes({ "/api/owner/projects/silkboard/sessions": response({}, 401) }),
    ),
  );
  const view = render(SessionList, {
    props: { project: "silkboard", onexpire: expire },
  });
  await new Promise((done) => setTimeout(done, 0));
  expect(expire).toHaveBeenCalledOnce();
  view.unmount();
  let resolve;
  vi.stubGlobal(
    "fetch",
    vi.fn(
      routes({
        "/api/owner/projects": response(
          page([{ id: "silkboard" }, { id: "other" }]),
        ),
        "/api/owner/projects/silkboard/sessions": () =>
          new Promise((done) => {
            resolve = done;
          }),
        "/api/owner/projects/other/sessions": response(
          page([{ ...session, session_name: "Other project" }]),
        ),
      }),
    ),
  );
  render(ProjectFeed, { props: { onexpire: vi.fn() } });
  await screen.findByLabelText("Project");
  await fireEvent.change(screen.getByLabelText("Project"), {
    target: { value: "other" },
  });
  await screen.findByText("Other project");
  resolve(response(page([{ ...session, session_name: "Old project" }])));
  await new Promise((done) => setTimeout(done, 0));
  expect(screen.queryByText("Old project")).toBeNull();
});
