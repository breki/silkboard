import { afterEach, expect, test, vi } from "vitest";
import { listPlans, readPlan, planHistory } from "./plan-api";
import { detail, task } from "../test/plans";
import { page, response } from "../test/support";
afterEach(() => vi.unstubAllGlobals());
test("owner plans use encoded bounded queries and same-origin private requests", async () => {
  const value = detail();
  const fetch = vi.fn(async (path) =>
    response(
      path.includes("/history") ||
        path.includes("/revisions") ||
        path.startsWith("/api/owner/plans")
        ? page([value.summary])
        : value,
    ),
  );
  vi.stubGlobal("fetch", fetch);
  expect(
    (await listPlans(null, new AbortController().signal)).items[0].title,
  ).toBe(value.snapshot.title);
  await listPlans("opaque cursor", new AbortController().signal);
  expect(
    (await readPlan("silkboard", task, {}, new AbortController().signal))
      .snapshot.task_id,
  ).toBe(task);
  await readPlan(
    "silkboard",
    task,
    { sequence: "1" },
    new AbortController().signal,
  );
  await readPlan(
    "silkboard",
    task,
    { revision: "1" },
    new AbortController().signal,
  );
  await planHistory(
    "silkboard",
    task,
    "history",
    "2",
    new AbortController().signal,
  );
  await planHistory(
    "silkboard",
    task,
    "revisions",
    null,
    new AbortController().signal,
  );
  expect(fetch.mock.calls.map(([p]) => p)).toContain(
    "/api/owner/plans?before=opaque%20cursor",
  );
  expect(fetch.mock.calls.map(([p]) => p)).toContain(
    `/api/owner/projects/silkboard/plans/${task}/history?before=2`,
  );
  expect(fetch.mock.calls[0][1]).toMatchObject({
    credentials: "same-origin",
    cache: "no-store",
  });
});
test("malformed and cross-task snapshots fail before layout or rendering", async () => {
  for (const change of [
    (v) => (v.snapshot.task_id = "bad"),
    (v) => (v.summary.task_id = "22222222-2222-4222-8222-222222222222"),
    (v) => v.snapshot.steps.push(v.snapshot.steps[0]),
    (v) => (v.snapshot.steps[0].depends_on = ["ui"]),
    (v) => (v.summary.counts.done = 99),
    (v) =>
      (v.prior_completions = [
        { step_id: "missing", sequence: "1", received_at: 1 },
      ]),
  ]) {
    const v = detail();
    change(v);
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => response(v)),
    );
    await expect(
      readPlan("silkboard", task, {}, new AbortController().signal),
    ).rejects.toMatchObject({ status: 0 });
  }
});
test("all reported states, bounded history and prior completions retain their exact context", async () => {
  for (const state of ["pending", "active", "done", "blocked"]) {
    const value = detail({
      steps: [
        {
          id: "only",
          title: "Only step",
          state,
          depends_on: [],
          description: "Text",
          report: "Report",
          blocked_reason: state === "blocked" ? "Waiting" : null,
        },
      ],
    });
    if (state !== "done")
      value.prior_completions = [
        { step_id: "only", sequence: "1", received_at: 0 },
      ];
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => response(value)),
    );
    expect(
      (await readPlan("silkboard", task, {}, new AbortController().signal))
        .snapshot.steps[0].state,
    ).toBe(state);
  }
});
test("response families reject invalid counts, blocked reasons, graph references and identities", async () => {
  const changes = [
    (v) => (v.snapshot.reporter.tool = ""),
    (v) =>
      (v.summary.reporter = { ...v.summary.reporter, hostname: "different" }),
    (v) => (v.snapshot.reporter.parent_session_id = "orphan"),
    (v) => (v.snapshot.steps[1].blocked_reason = "unexpected"),
    (v) => (v.snapshot.steps[0].state = "pending"),
    (v) => (v.summary.active_step_ids = ["contract"]),
    (v) => (v.snapshot.description = "a".repeat(2001)),
    (v) => (v.snapshot.github_issue = { repository: "../evil", number: "1" }),
    (v) => (v.snapshot.steps[0].depends_on = ["missing"]),
    (v) => (v.snapshot.steps[0].title = "\u0000"),
    (v) =>
      (v.prior_completions = [
        { step_id: "ui", sequence: "2", received_at: 0 },
      ]),
    (v) =>
      (v.prior_completions = [
        { step_id: "contract", sequence: "1", received_at: 0 },
      ]),
  ];
  for (const change of changes) {
    const value = detail();
    change(value);
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => response(value)),
    );
    await expect(
      readPlan("silkboard", task, {}, new AbortController().signal),
    ).rejects.toMatchObject({ status: 0 });
  }
});
test("selectors, cross-scope pages, duplicate rows and mismatched snapshots are rejected", async () => {
  const signal = new AbortController().signal;
  for (const [project, id, selector] of [
    ["bad/", task, {}],
    ["ok", "bad", {}],
    ["ok", task, { sequence: "0" }],
    ["ok", task, { revision: "01" }],
    ["ok", task, { sequence: "1", revision: "1" }],
  ])
    await expect(readPlan(project, id, selector, signal)).rejects.toMatchObject(
      { status: 0 },
    );
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response(detail())),
  );
  await expect(
    readPlan("silkboard", task, { sequence: "2" }, signal),
  ).rejects.toMatchObject({ status: 0 });
  await expect(
    readPlan("silkboard", task, { revision: "2" }, signal),
  ).rejects.toMatchObject({ status: 0 });
  await expect(
    planHistory("silkboard", task, "history", "0", signal),
  ).rejects.toMatchObject({ status: 0 });
  const value = detail().summary;
  for (const items of [[value, value], [{ ...value, project: "other" }]]) {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => response(page(items))),
    );
    await expect(
      planHistory("silkboard", task, "history", null, signal),
    ).rejects.toMatchObject({ status: 0 });
  }
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => response(page([value, value]))),
  );
  await expect(listPlans(null, signal)).rejects.toMatchObject({ status: 0 });
});
