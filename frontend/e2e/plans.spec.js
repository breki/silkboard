import { fixturePort } from "./ports.mjs";
import { expect, test } from "@playwright/test";
import { checkFilters } from "./filters.mjs";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { password } from "./fixture.mjs";
import { task, maximumTask, liveTask, publish } from "./plans-fixture.mjs";
import { detail } from "../test/plans.js";
test.use({ baseURL: `https://127.0.0.1:${fixturePort(38128)}` });
let cookies;
test.beforeAll(async ({ browser }) => {
  const context = await browser.newContext({
    ignoreHTTPSErrors: true,
    baseURL: `https://127.0.0.1:${fixturePort(38128)}`,
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
  await page.getByRole("link", { name: "Return to Current workflow" }).click();
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
    name: "Workflows from all projects",
  });
  await expect(table.locator("tbody tr")).toHaveCount(20);
  await page.getByRole("button", { name: "Older workflows" }).click();
  await expect(
    page.getByRole("button", { name: "Latest workflows" }),
  ).toBeVisible();
  const title = await table.getByRole("link").first().textContent();
  await table.getByRole("link").first().click();
  await expect(
    page.getByRole("region", { name: "Task prerequisite graph" }),
  ).toBeVisible();
  await page.goBack();
  await expect(
    page.getByRole("button", { name: "Latest workflows" }),
  ).toBeVisible();
  await expect(table.getByRole("link").first()).toHaveText(title);
  await checkFilters(page, true);
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
    page.getByRole("heading", { name: "Invalid workflow link" }),
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
  const history = page.getByRole("list", { name: "Received workflow reports" });
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

test("explicit event workflow links open the existing graph by keyboard on a phone", async ({
  page,
  context,
}) => {
  const token = readFileSync(
    join(process.env.SILKBOARD_E2E_DIR, "plans.token"),
    "utf8",
  ).trim();
  const result = await context.request.post(
    "/api/projects/silkboard/progress",
    {
      headers: { Authorization: `Bearer ${token}` },
      data: {
        event_id: "linked-workflow-event",
        hostname: "link-fixture",
        tool: "codex",
        session_id: "explicit-workflow",
        sequence: 1,
        message: "Continue this workflow",
        task_id: task,
      },
    },
  );
  expect(result.ok()).toBe(true);
  await page.setViewportSize({ width: 320, height: 720 });
  await openPlan(page, "/events");
  const entry = page
    .locator("tr.event-row")
    .filter({ hasText: "Continue this workflow" });
  const workflow = entry.getByRole("link", { name: "Open workflow" });
  await expect(workflow).toHaveAttribute("href", link(task));
  await entry
    .getByRole("button", { name: "Event details: Continue this workflow" })
    .click();
  await expect(
    page.getByRole("region", { name: "Event details" }),
  ).toBeVisible();
  await noOverflow(page);
  await workflow.focus();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("region", { name: "Task prerequisite graph" }),
  ).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`${link(task)}$`));
});

test("Main agent cards link exact main events and reported workflow snapshots", async ({
  page,
  context,
}, info) => {
  await page.clock.install();
  const token = readFileSync(
    join(process.env.SILKBOARD_E2E_DIR, "plans.token"),
    "utf8",
  ).trim();
  const agentTask = "abababab-abab-4bab-8bab-abababababab";
  const first = detail({
    task_id: agentTask,
    title: "Agents handoff",
  }).snapshot;
  first.reporter.session_id = "agents-writer-one";
  await publish(first, token);
  const handoff = structuredClone(first);
  handoff.sequence = "2";
  handoff.writer_generation = "2";
  handoff.reporter.session_id = "agents-writer-two";
  await publish(handoff, token);
  for (const child of [null, "child-a", "child-b"]) {
    const seeded = await context.request.post(
      "/api/projects/silkboard/progress",
      {
        headers: { Authorization: `Bearer ${token}` },
        data: {
          event_id: `agents-${child ?? "primary"}`,
          hostname: "agents-host",
          tool: "codex",
          session_id: "agents-native",
          subagent_id: child,
          parent_session_id: child === null ? null : "agents-native",
          sequence: 1,
          session_name: `Agents ${child ?? "primary"}`,
          message: `Report ${child ?? "primary"}`,
        },
      },
    );
    expect(seeded.status()).toBe(200);
  }
  const longName = "Subagent " + "E".repeat(111);
  const longEvent = await context.request.post(
    "/api/projects/silkboard/progress",
    {
      headers: { Authorization: `Bearer ${token}` },
      data: {
        event_id: "agents-long-main",
        hostname: "event-host-" + "h".repeat(245),
        tool: "event-tool-" + "t".repeat(245),
        session_id: "event-native-" + "n".repeat(243),
        sequence: 1,
        session_name: longName,
        message: "x".repeat(2000),
      },
    },
  );
  expect(longEvent.status()).toBe(200);
  const longPlan = detail({
    task_id: "acacacac-acac-4cac-8cac-acacacacacac",
    title: "W".repeat(160),
  }).snapshot;
  longPlan.reporter = {
    ...longPlan.reporter,
    hostname: "workflow-host-" + "h".repeat(242),
    tool: "workflow-tool-" + "t".repeat(242),
    session_id: "workflow-native-" + "n".repeat(240),
    session_name: "Workflow " + "W".repeat(111),
  };
  await publish(longPlan, token);
  await page.setViewportSize({ width: 320, height: 720 });
  await openPlan(page, "/");
  const cards = page.getByRole("list", { name: "Main agents" });
  await expect(cards).toBeVisible();
  await expect(
    cards.getByRole("link", { name: "Agents child-a", exact: true }),
  ).toHaveCount(0);
  await expect(
    cards.getByRole("link", { name: "Agents child-b", exact: true }),
  ).toHaveCount(0);
  await expect(
    cards.getByRole("link", { name: longName, exact: true }),
  ).toBeVisible();
  const before = await cards
    .getByRole("article", { name: "Agents primary", exact: true })
    .textContent();
  const update = await context.request.post(
    "/api/projects/silkboard/progress",
    {
      headers: { Authorization: `Bearer ${token}` },
      data: {
        event_id: "agents-child-refresh",
        hostname: "agents-host",
        tool: "codex",
        session_id: "agents-native",
        subagent_id: "child-refresh",
        parent_session_id: "agents-native",
        sequence: 1,
        message: "New child activity",
      },
    },
  );
  expect(update.status()).toBe(200);
  await page.clock.fastForward(10000);
  await expect(
    cards.getByRole("article", { name: "Agents primary", exact: true }),
  ).toHaveText(before);
  await expect(
    cards.getByText("Event: New child activity", { exact: true }),
  ).toHaveCount(0);
  for (const width of [320, 390, 1280]) {
    await page.setViewportSize({ width, height: 800 });
    await noOverflow(page);
    expect(
      await cards
        .getByRole("article")
        .evaluateAll((nodes) =>
          nodes.every((node) => node.scrollWidth <= node.clientWidth),
        ),
    ).toBe(true);
    const columns = await cards
      .getByRole("article")
      .evaluateAll(
        (nodes) =>
          new Set(
            nodes.map((node) => Math.round(node.getBoundingClientRect().x)),
          ).size,
      );
    if (width < 600) expect(columns).toBe(1);
    else expect(columns).toBeGreaterThan(1);
    const received = cards
      .getByRole("article", { name: "Agents primary", exact: true })
      .locator("time");
    await expect(received).toHaveText(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/);
    await page.screenshot({
      path: info.outputPath(`agents-cards-${width}.png`),
      fullPage: true,
    });
  }
  await page.setViewportSize({ width: 320, height: 720 });
  await noOverflow(page);
  await page.getByRole("link", { name: "Agents primary", exact: true }).focus();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("region", { name: "Selected agent session" }),
  ).toBeVisible();
  expect(new URL(page.url()).searchParams.get("subagent_id")).toBe("");
  await expect(
    page.getByRole("button", {
      name: "Event details: Report primary",
      exact: true,
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", {
      name: "Event details: Report child-b",
      exact: true,
    }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Session hierarchy" }).click();
  await page.reload();
  await expect(
    page.getByRole("region", { name: "Selected agent session" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Clear session filter" }).click();
  await expect(
    page.getByRole("button", { name: "Project: All projects" }),
  ).toBeVisible();
  await page.goBack();
  await expect(
    page.getByRole("region", { name: "Selected agent session" }),
  ).toBeVisible();
  await page.goForward();
  await expect(
    page.getByRole("button", { name: "Project: All projects" }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Agents", exact: true }).click();
  const links = cards.locator(`a[href^="${link(agentTask)}?sequence="]`);
  await expect(links).toHaveCount(2);
  const targets = await links.evaluateAll((nodes) =>
    nodes.map((node) => node.getAttribute("href")),
  );
  expect(targets).toContain(`${link(agentTask)}?sequence=1`);
  expect(targets).toContain(`${link(agentTask)}?sequence=2`);
  await page.screenshot({
    path: info.outputPath("agents-phone.png"),
    fullPage: true,
  });
  await cards.locator(`a[href="${link(agentTask)}?sequence=1"]`).click();
  await expect(page).toHaveURL(`${link(agentTask)}?sequence=1`);
  await expect(page.getByText(/Earlier snapshot/).first()).toBeVisible();
});

test("four primary pages keep Todos visible and mark the active route", async ({
  page,
}, info) => {
  const cases = [
    ["/", "Agents"],
    ["/events", "Events"],
    ["/plans", "Workflows"],
    ["/todos", "Todos"],
    [`/plans/silkboard/${task}?sequence=1`, "Workflows"],
  ];
  for (const [path, label] of cases) {
    await openPlan(page, path);
    const primary = page.getByRole("navigation", {
      name: "Primary navigation",
    });
    await expect(page.getByLabel("Owner password")).toHaveCount(0);
    await expect(primary.getByRole("link")).toHaveCount(4);
    const current = primary.getByRole("link", { name: label, exact: true });
    await expect(current).toHaveAttribute("aria-current", "page");
    await expect(primary.locator('[aria-current="page"]')).toHaveCount(1);
    for (const width of [320, 390, 1280]) {
      await page.setViewportSize({ width, height: 800 });
      await noOverflow(page);
      const links = primary.getByRole("link");
      for (const link of await links.all()) {
        await expect(link).toBeVisible();
        const rect = await link.boundingBox();
        expect(rect.width).toBeGreaterThanOrEqual(44);
        expect(rect.height).toBeGreaterThanOrEqual(44);
      }
      const nav = await primary.boundingBox();
      const menu = await page
        .getByRole("button", { name: "Navigation menu" })
        .boundingBox();
      if (width < 600)
        expect(nav.y).toBeGreaterThanOrEqual(menu.y + menu.height);
      await page.screenshot({
        path: info.outputPath(`primary-${label}-${width}.png`),
      });
    }
    await page.setViewportSize({ width: 320, height: 800 });
    await primary.getByRole("link", { name: "Agents", exact: true }).focus();
    for (const next of ["Events", "Workflows", "Todos"]) {
      await page.keyboard.press("Tab");
      await expect(
        primary.getByRole("link", { name: next, exact: true }),
      ).toBeFocused();
    }
    if (path === "/") {
      await page.getByRole("button", { name: "Navigation menu" }).click();
      const drawer = page.getByRole("navigation", { name: "Main navigation" });
      await expect(
        drawer.getByRole("link", { name: "Todos", exact: true }),
      ).toHaveCount(0);
      const popup = await drawer.boundingBox();
      const nav = await primary.boundingBox();
      expect(popup.y).toBeGreaterThanOrEqual(nav.y + nav.height);
      await primary.getByRole("link", { name: "Todos", exact: true }).click();
    } else await page.keyboard.press("Enter");
    await expect(page).toHaveURL("/todos");
    await expect(
      primary.getByRole("link", { name: "Todos", exact: true }),
    ).toHaveAttribute("aria-current", "page");
    await expect(
      page.getByRole("heading", { name: "Todos", exact: true }),
    ).toBeVisible();
    await page.reload();
    await expect(
      primary.getByRole("link", { name: "Todos", exact: true }),
    ).toHaveAttribute("aria-current", "page");
    if (path !== "/todos") {
      await page.goBack();
      await expect(page).toHaveURL(path);
      await expect(
        primary.getByRole("link", { name: label, exact: true }),
      ).toHaveAttribute("aria-current", "page");
      await page.goForward();
      await expect(page).toHaveURL("/todos");
    }
  }
});
