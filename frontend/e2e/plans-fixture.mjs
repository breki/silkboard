import { fixturePort } from "./ports.mjs";
import { detail, task } from "../test/plans.js";
import { markdownReport } from "./fixture.mjs";
export { task };
export const maximumTask = "ffffffff-ffff-4fff-8fff-ffffffffffff";
export const liveTask = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee";
export function snapshots() {
  const first = detail().snapshot;
  first.steps[1].state = "done";
  first.steps[1].description = markdownReport;
  first.steps[1].report = "Initial completion received.";
  const reopened = structuredClone(first);
  reopened.sequence = "3";
  reopened.steps[1].state = "active";
  reopened.steps[1].report = "Rechecking browser behavior.";
  reopened.steps[2].state = "blocked";
  reopened.steps[2].blocked_reason = "Waiting for the graph change.";
  const revised = structuredClone(reopened);
  revised.sequence = "4";
  revised.plan_revision = "2";
  revised.steps.push({
    id: "docs",
    title: "Explain the graph",
    description: null,
    depends_on: ["contract"],
    state: "active",
    report: null,
    blocked_reason: null,
  });
  const handoff = structuredClone(revised);
  handoff.sequence = "5";
  handoff.writer_generation = "2";
  handoff.reporter.session_id = "native-handoff";
  handoff.reporter.hostname = "new-development-vm";
  return [first, reopened, revised, handoff];
}
export function maximum() {
  const steps = Array.from({ length: 128 }, (_, i) => ({
    id: `s${i}`,
    title: `Maximum step ${i + 1}`,
    description: null,
    depends_on:
      i >= 16 && i < 48 ? Array.from({ length: 16 }, (_, j) => `s${j}`) : [],
    state: i < 16 ? "done" : "pending",
    report: null,
    blocked_reason: null,
  }));
  return detail({ task_id: maximumTask, title: "Maximum sized plan", steps })
    .snapshot;
}
export async function publish(snapshot, token) {
  const response = await fetch(
    `http://127.0.0.1:${fixturePort(38129)}/api/projects/silkboard/plans`,
    {
      method: "POST",
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify(snapshot),
    },
  );
  if (!response.ok)
    throw new Error(`Plan fixture publish failed: ${response.status}`);
}
export async function seedPlans(token) {
  for (const value of snapshots()) await publish(value, token);
  await publish(
    detail({ task_id: liveTask, title: "Live browser plan" }).snapshot,
    token,
  );
  for (let i = 1; i <= 21; i++)
    await publish(
      detail({
        task_id: `00000000-0000-4000-8000-${String(i).padStart(12, "0")}`,
        title: `Independent attempt ${i}`,
      }).snapshot,
      token,
    );
  await publish(maximum(), token);
}
