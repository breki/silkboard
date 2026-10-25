export const task = "11111111-1111-4111-8111-111111111111";
export const reporter = {
  hostname: "dev",
  tool: "codex",
  session_id: "native-1",
  subagent_id: null,
  parent_session_id: null,
  session_name: "Task graph development",
};
export function detail(overrides = {}) {
  const snapshot = {
    schema_version: 1,
    task_id: task,
    sequence: "1",
    plan_revision: "1",
    writer_generation: "1",
    reporter: structuredClone(reporter),
    title: "Build task-progress graphs",
    description: "A **private** evolving plan",
    github_issue: { repository: "breki/silkboard", number: "78" },
    steps: [
      {
        id: "contract",
        title: "Agree contract",
        description: null,
        depends_on: [],
        state: "done",
        report: null,
        blocked_reason: null,
      },
      {
        id: "ui",
        title: "Build graph page",
        description: "Graph and steps",
        depends_on: ["contract"],
        state: "active",
        report: "Implementing the owner view.",
        blocked_reason: null,
      },
      {
        id: "verify",
        title: "Verify browser behavior",
        description: null,
        depends_on: ["ui"],
        state: "pending",
        report: null,
        blocked_reason: null,
      },
    ],
    ...overrides,
  };
  const count = (state) =>
    snapshot.steps.filter((s) => s.state === state).length;
  const counts = {
    total: snapshot.steps.length,
    done: count("done"),
    active: count("active"),
    blocked: count("blocked"),
  };
  const state =
    counts.done === counts.total
      ? "completed"
      : counts.active
        ? "active"
        : counts.blocked
          ? "blocked"
          : "pending";
  return {
    snapshot,
    summary: {
      project: "silkboard",
      task_id: snapshot.task_id,
      sequence: snapshot.sequence,
      plan_revision: snapshot.plan_revision,
      writer_generation: snapshot.writer_generation,
      title: snapshot.title,
      reporter: snapshot.reporter,
      state,
      counts,
      active_step_ids: snapshot.steps
        .filter((s) => s.state === "active")
        .map((s) => s.id),
      blocked_step_ids: snapshot.steps
        .filter((s) => s.state === "blocked")
        .map((s) => s.id),
      received_at: 1792300000000,
      history_incomplete: false,
    },
    prior_completions: [],
  };
}
