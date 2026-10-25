import { expect, test } from "@playwright/test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { password } from "./fixture.mjs";
import { task, maximumTask, liveTask, publish } from "./plans-fixture.mjs";
import { detail } from "../test/plans.js";
test.use({ baseURL: "https://127.0.0.1:38128" });
let cookies;
test.beforeAll(async ({ browser }) => {
  const context = await browser.newContext({
    ignoreHTTPSErrors: true,
    baseURL: "https://127.0.0.1:38128",
  });
  const page = await context.newPage();
  await login(page);
  await expect(page.getByRole("table")).toBeVisible();
  cookies = await context.cookies();
  await context.close();
});
async function openPlan(page, path = "/plans") {
  await page.context().addCookies(cookies);
  await page.goto(path);
}
const link = (id) => `/plans/silkboard/${id}`;
async function login(page, path = "/plans") {
  await page.goto(path);
  await page.getByLabel("Owner password").fill(password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
}
const noOverflow = async (page) =>
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
test("real owner login, direct snapshots, reopening, revision pinning and safe details", async ({
  page,
  context,
}, info) => {
  expect(
    (
      await context.request.get(`/api/owner/projects/silkboard/plans/${task}`)
    ).status(),
  ).toBe(401);
  await page.setViewportSize({ width: 1280, height: 900 });
  await login(page, `${link(task)}?sequence=3`);
  await expect(
    page.getByRole("region", { name: "Task prerequisite graph" }),
  ).toBeVisible();
  expect(new URL(page.url()).search).toBe("?sequence=3");
  await page.reload();
  await expect(page.getByText(/Newer reports are available/)).toBeVisible();
  await page.getByRole("button", { name: "Build graph page, Active" }).click();
  const aside = page.getByRole("complementary", { name: "Step details" });
  await expect(
    aside.getByText("Previously marked done in received history.", {
      exact: false,
    }),
  ).toBeVisible();
  await expect(aside.getByText("Native session")).toBeVisible();
  await expect(page.locator("img")).toHaveCount(0);
  await expect(page.locator(".event-message script")).toHaveCount(0);
  expect(await page.evaluate(() => window.injected)).toBeUndefined();
  await aside.getByRole("link", { name: "View that snapshot" }).click();
  await expect(
    page.getByRole("button", { name: "Build graph page, Done" }),
  ).toBeVisible();
  expect(new URL(page.url()).search).toBe("?sequence=1");
  await page.getByRole("link", { name: "Return to Current plan" }).click();
  await expect(
    page.getByText("new-development-vm", { exact: false }).first(),
  ).toBeVisible();
  await page
    .getByRole("combobox", { name: "Choose active step" })
    .selectOption("docs");
  await expect(
    aside.getByRole("heading", { name: "Explain the graph" }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Compare with previous received report" })
    .click();
  await expect(page.getByText(/continuity changed/)).toBeVisible();
  await page
    .getByRole("button", { name: "Received history and revisions" })
    .click();
  await page
    .getByRole("button", { name: "Definition revisions", exact: true })
    .click();
  await page.getByRole("button", { name: "Revision 1", exact: true }).click();
  await expect(page).toHaveURL(`${link(task)}?sequence=3`);
  await expect(page.getByText(/Earlier snapshot/).first()).toBeVisible();
  await page
    .getByRole("button", { name: "Compare with previous received report" })
    .click();
  await expect(page.getByText(/spans missing reports/)).toBeVisible();
  await expect(page.getByText(/Done → Active/)).toBeVisible();
  await page.screenshot({
    path: info.outputPath("task-plan-desktop.png"),
    fullPage: true,
  });
  await noOverflow(page);
  await page.getByRole("button", { name: "Navigation menu" }).click();
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  await expect(page.getByLabel("Owner password")).toBeVisible();
  await expect(
    page.getByRole("region", { name: "Task prerequisite graph" }),
  ).toHaveCount(0);
});
test("phone modal, keyboard focus, resize and bounded maximum graph remain usable", async ({
  page,
}, info) => {
  await page.setViewportSize({ width: 320, height: 720 });
  await openPlan(page, link(task));
  await page.getByRole("button", { name: "Build graph page, Active" }).focus();
  await page.keyboard.press("Enter");
  const dialog = page.getByRole("dialog", { name: "Step details" });
  await expect(dialog).toBeVisible();
  await noOverflow(page);
  await expect(
    dialog.getByRole("heading", { name: "Build graph page" }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(dialog).not.toBeVisible();
  await expect(
    page.getByRole("button", { name: "Build graph page, Active" }),
  ).toBeFocused();
  await page.getByRole("button", { name: "View selected step" }).click();
  await page.keyboard.press("Shift+Tab");
  expect(
    await page.evaluate(() =>
      document.querySelector("dialog").contains(document.activeElement),
    ),
  ).toBe(true);
  await page.screenshot({
    path: info.outputPath("task-plan-phone.png"),
    fullPage: true,
  });
  await page.setViewportSize({ width: 1200, height: 900 });
  await expect(dialog).not.toBeVisible();
  await expect(page.getByRole("complementary")).toBeVisible();
  await page.goto(link(maximumTask));
  await expect(page.locator(".graph-node")).toHaveCount(128);
  await expect(page.locator(".prerequisite-edge")).toHaveCount(512);
  await page.setViewportSize({ width: 320, height: 720 });
  await noOverflow(page);
  await page.getByRole("button", { name: "Steps", exact: true }).click();
  await expect(
    page
      .getByRole("list", { name: "Steps in prerequisite order" })
      .locator(":scope > li"),
  ).toHaveCount(128);
  await noOverflow(page);
  await page.screenshot({
    path: info.outputPath("task-plan-maximum-phone.png"),
    fullPage: true,
  });
});
test("list pagination and browser Back restore the earlier list page", async ({
  page,
}) => {
  await openPlan(page);
  const table = page.getByRole("table", {
    name: "Task plans from all projects",
  });
  await expect(table.locator("tbody tr")).toHaveCount(20);
  await page.getByRole("button", { name: "Older task plans" }).click();
  await expect(
    page.getByRole("button", { name: "Latest task plans" }),
  ).toBeVisible();
  const title = await table.getByRole("link").first().textContent();
  await table.getByRole("link").first().click();
  await expect(
    page.getByRole("region", { name: "Task prerequisite graph" }),
  ).toBeVisible();
  await page.goBack();
  await expect(
    page.getByRole("button", { name: "Latest task plans" }),
  ).toBeVisible();
  await expect(table.getByRole("link").first()).toHaveText(title);
});
test("live polling preserves selection and graph positions, history stays pinned", async ({
  page,
}) => {
  test.setTimeout(45000);
  await openPlan(page, link(liveTask));
  const node = page.getByRole("button", { name: "Build graph page, Active" });
  await node.click();
  const before = await node.getAttribute("style");
  const token = readFileSync(
    join(process.env.SILKBOARD_E2E_DIR, "plans.token"),
    "utf8",
  ).trim();
  const next = detail({
    task_id: liveTask,
    title: "Live browser plan",
    sequence: "2",
  }).snapshot;
  next.steps[1].state = "done";
  next.steps[2].state = "active";
  await publish(next, token);
  const done = page.getByRole("button", { name: "Build graph page, Done" });
  await expect(done).toBeVisible({ timeout: 15000 });
  expect(await done.getAttribute("style")).toBe(before);
  await expect(
    page
      .getByRole("complementary")
      .getByRole("heading", { name: "Build graph page" }),
  ).toBeVisible();
  await page.goto(`${link(liveTask)}?sequence=1`);
  await expect(page.getByText(/Earlier snapshot/).first()).toBeVisible();
  next.sequence = "3";
  await publish(next, token);
  await expect(page.getByText(/Newer reports are available/)).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Build graph page, Active" }),
  ).toBeVisible();
});

test("expired owner sessions clear all private plan content on the next poll", async ({
  page,
  context,
}) => {
  test.setTimeout(30000);
  await openPlan(page, link(task));
  await expect(
    page.getByRole("region", { name: "Task prerequisite graph" }),
  ).toBeVisible();
  await context.clearCookies();
  await expect(page.getByLabel("Owner password")).toBeVisible({
    timeout: 15000,
  });
  await expect(
    page.getByRole("region", { name: "Task prerequisite graph" }),
  ).toHaveCount(0);
  await expect(
    page.getByText("Your session expired. Sign in again."),
  ).toBeVisible();
});
test("long unbroken reported labels and identity fit a small phone", async ({
  page,
}) => {
  const token = readFileSync(
    join(process.env.SILKBOARD_E2E_DIR, "plans.token"),
    "utf8",
  ).trim();
  const value = detail({
    task_id: "dddddddd-dddd-4ddd-8ddd-dddddddddddd",
    title: "T".repeat(160),
  }).snapshot;
  value.reporter.hostname = "h".repeat(256);
  value.steps[1].title = "N".repeat(160);
  value.steps[2].depends_on = ["contract"];
  value.steps[2].state = "active";
  await publish(value, token);
  await page.setViewportSize({ width: 320, height: 720 });
  await openPlan(page, link(value.task_id));
  await expect(
    page.getByRole("combobox", { name: "Choose active step" }),
  ).toBeVisible();
  await noOverflow(page);
});

test("removing a selected phone step closes its dialog and returns focus to the graph heading", async ({
  page,
}) => {
  const token = readFileSync(
    join(process.env.SILKBOARD_E2E_DIR, "plans.token"),
    "utf8",
  ).trim();
  const value = detail({
    task_id: "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
    title: "Removal focus",
  }).snapshot;
  await publish(value, token);
  await page.clock.install();
  await page.setViewportSize({ width: 320, height: 720 });
  await openPlan(page, link(value.task_id));
  await page.getByRole("button", { name: "Build graph page, Active" }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  value.sequence = "2";
  value.plan_revision = "2";
  value.steps = [value.steps[0]];
  await publish(value, token);
  await page.clock.fastForward(10000);
  await expect(
    page.getByText("The selected step is not in this snapshot."),
  ).toBeVisible();
  await expect(page.getByRole("dialog")).not.toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Steps and prerequisites" }),
  ).toBeFocused();
});

test("an invalid plan link still expires an idle credential manager", async ({
  page,
  context,
}) => {
  await page.clock.install();
  await openPlan(page, "/plans?bad=1");
  await expect(
    page.getByRole("heading", { name: "Invalid task-plan link" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Navigation menu" }).click();
  await page
    .getByRole("button", { name: "Manage publisher credentials" })
    .click();
  await page.getByLabel("Credential project").fill("silkboard");
  await page.getByLabel("Installation label").fill("invalid-link-fixture");
  await page.getByRole("button", { name: "Issue token", exact: true }).click();
  await expect(page.getByLabel("New publisher token")).toBeVisible();
  await context.clearCookies();
  await page.clock.fastForward(10000);
  await expect(page.getByLabel("Owner password")).toBeVisible();
  await expect(page.getByLabel("New publisher token")).toHaveCount(0);
  await page.getByRole("button", { name: "Navigation menu" }).click();
  await expect(
    page.getByRole("button", { name: "Sign out", exact: true }),
  ).toHaveCount(0);
});

test("the next full snapshot repairs a lost revision while preserving selected work and received history", async ({
  page,
}) => {
  const token = readFileSync(
    join(process.env.SILKBOARD_E2E_DIR, "plans.token"),
    "utf8",
  ).trim();
  const value = detail({
    task_id: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
    title: "Recover a missed revision",
  }).snapshot;
  await publish(value, token);
  await page.clock.install();
  await openPlan(page, link(value.task_id));
  await page.getByRole("button", { name: "Build graph page, Active" }).click();
  // Sequences 2 and 3 / definition revision 2 are deliberately never sent.
  value.sequence = "4";
  value.plan_revision = "3";
  value.steps[1].state = "done";
  value.steps[2].state = "blocked";
  value.steps[2].blocked_reason = "Awaiting owner review";
  value.steps[2].depends_on = ["contract"];
  value.steps.push({
    id: "review",
    title: "Review the repaired plan",
    description: null,
    depends_on: ["contract"],
    state: "active",
    report: "Received after the gap",
    blocked_reason: null,
  });
  await publish(value, token);
  await page.clock.fastForward(10000);
  const done = page.getByRole("button", { name: "Build graph page, Done" });
  await expect(done).toHaveAttribute("aria-pressed", "true");
  await expect(
    page.getByRole("button", { name: "Review the repaired plan, Active" }),
  ).toBeVisible();
  await expect(page.getByText(/Some reports were not received/)).toBeVisible();
  await expect(page.getByText("2 of 4 steps marked done")).toBeVisible();
  await page
    .getByRole("button", { name: "Received history and revisions" })
    .click();
  const history = page.getByRole("list", { name: "Received plan reports" });
  await expect(history.locator(":scope > li")).toHaveCount(2);
  await page
    .getByRole("button", { name: "Compare with previous received report" })
    .click();
  await expect(page.getByText("Added: Review the repaired plan")).toBeVisible();
  await expect(
    page.getByText("Changed prerequisites: Verify browser behavior"),
  ).toBeVisible();
  await page.route(
    `**/api/owner/projects/silkboard/plans/${value.task_id}`,
    (route) =>
      route.fulfill({
        status: 503,
        body: "{}",
        contentType: "application/json",
      }),
  );
  await page.clock.fastForward(10000);
  await expect(page.getByRole("alert")).toContainText("out of date");
  await expect(done).toHaveAttribute("aria-pressed", "true");
  await page
    .getByRole("button", { name: "Verify browser behavior, Blocked" })
    .click();
  await expect(page.getByText("Awaiting owner review")).toBeVisible();
  await expect(
    page.getByText(
      "These are reported states, not verified results or a live connection status.",
    ),
  ).toBeVisible();
});
