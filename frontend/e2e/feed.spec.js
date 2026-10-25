import { expect, test } from "@playwright/test";
import { checkFilters, chooseFilter } from "./filters.mjs";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { execFileSync } from "node:child_process";
import { password, hostile } from "./fixture.mjs";

test("owner login uses a secure cookie and logout revokes event access", async ({
  page,
  context,
}) => {
  expect((await context.request.get("/api/owner/events")).status()).toBe(401);
  await page.goto("/");
  await page.getByLabel("Owner password").fill("incorrect fixture password");
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("Password not accepted");
  await page.getByLabel("Owner password").fill(password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(
    page.getByRole("table", { name: "Events from all projects" }),
  ).toBeVisible();
  const cookie = (await context.cookies()).find(
    (value) => value.name === "__Host-silkboard-owner",
  );
  expect(cookie).toMatchObject({
    secure: true,
    httpOnly: true,
    sameSite: "Strict",
    path: "/",
  });
  expect(await page.evaluate(() => document.cookie)).not.toContain(
    "silkboard-owner",
  );
  const primary = page.getByRole("navigation", { name: "Primary navigation" });
  await primary.getByRole("link", { name: "Workflows", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Workflows", exact: true }),
  ).toBeVisible();
  await expect(page.getByLabel("Owner password")).toHaveCount(0);
  await primary.getByRole("link", { name: "Events", exact: true }).click();
  await expect(
    page.getByRole("table", { name: "Events from all projects" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Navigation menu" }).click();
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  await expect(page.getByLabel("Owner password")).toBeVisible();
  expect((await context.request.get("/api/owner/events")).status()).toBe(401);
});

test("one phone table shows all projects, opens safe event details, pages and refreshes", async ({
  page,
  context,
}, info) => {
  await page.setViewportSize({ width: 375, height: 812 });
  await page.goto("/");
  await page.getByLabel("Owner password").fill(password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  const table = page.getByRole("table", { name: "Events from all projects" });
  await expect(table.locator("tr.event-row")).toHaveCount(20);
  await page.setViewportSize({ width: 320, height: 720 });
  await page.getByRole("button", { name: "Navigation menu" }).click();
  await expect(
    page.getByRole("button", { name: "Manage publisher credentials" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Sign out", exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: info.outputPath("owner-menu-small-phone.png"),
    fullPage: true,
  });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.getByRole("link", { name: "Application status" }).click();
  await expect(page.getByText("Healthy", { exact: true })).toBeVisible();
  await expect(table).toHaveCount(0);
  await page.reload();
  await page.getByRole("button", { name: "Navigation menu" }).click();
  await expect(
    page.getByRole("button", { name: "Sign out", exact: true }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Events", exact: true }).click();
  await expect(table.locator("tr.event-row")).toHaveCount(20);
  await page.setViewportSize({ width: 375, height: 812 });
  await expect(table.getByRole("columnheader")).toHaveText([
    "Time",
    "Project",
    "Agent",
    "Message",
  ]);
  await expect(
    page.getByRole("button", { name: "Project: All projects" }),
  ).toBeVisible();
  await expect(table.getByText("bombyx", { exact: true })).toBeVisible();
  expect(
    await table.getByText("silkboard", { exact: true }).count(),
  ).toBeGreaterThan(0);
  expect(await page.getByText("n".repeat(256), { exact: true }).count()).toBe(
    0,
  );
  const details = page.getByRole("region", { name: "Event details" });
  const externalRequests = [];
  await page.route("https://example.invalid/**", (route) => {
    externalRequests.push(route.request().url());
    return route.abort();
  });
  const markdownButton = page.getByRole("button", {
    name: /^Event details: Markdown report Ready/,
  });
  await markdownButton.click();
  await expect(details.locator("strong")).toHaveText("Ready");
  await expect(details.locator("em")).toHaveText("reviewed");
  await expect(details.locator("li")).toHaveCount(2);
  await expect(details.locator("pre code")).toContainText("long-code-line-");
  await expect(details.locator(".event-message table td").first()).toHaveText(
    "Tests",
  );
  await expect(details.locator(".event-message code").first()).toHaveText(
    "cargo test",
  );
  await expect(
    details.getByRole("link", { name: "Reference" }),
  ).toHaveAttribute("href", "https://example.invalid/reference");
  await expect(details.locator("img,script,iframe,style")).toHaveCount(0);
  await expect(details.locator('a[href^="javascript:"]')).toHaveCount(0);
  await expect(details.getByText("Event ID", { exact: true })).toHaveCount(0);
  await expect(details.getByText("Sequence", { exact: true })).toHaveCount(0);
  await expect(details.locator("time")).toHaveText(
    /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/,
  );
  const tableTime = table.locator("tr.event-row time").first();
  await expect(tableTime.locator("span").nth(0)).toHaveText(/^\d{2}:\d{2}$/);
  await expect(tableTime.locator("span").nth(1)).toHaveText(/^\d{2}-\d{2}$/);
  await page.setViewportSize({ width: 320, height: 812 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: info.outputPath("markdown-small-phone.png"),
    fullPage: true,
  });
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.screenshot({
    path: info.outputPath("markdown-desktop.png"),
    fullPage: true,
  });
  await page.setViewportSize({ width: 375, height: 812 });
  expect(externalRequests).toEqual([]);
  expect(await page.evaluate(() => window.injected)).toBeUndefined();
  await markdownButton.click();
  const button = page.getByRole("button", {
    name: `Event details: ${hostile}`,
    exact: true,
  });
  const row = table.locator("tr.event-row").filter({ has: button });
  await button.focus();
  await page.keyboard.press("Enter");
  await expect(details).toBeVisible();
  await expect(
    details.getByText("n".repeat(256), { exact: true }),
  ).toBeVisible();
  await expect(details.getByText(hostile, { exact: true })).toBeVisible();
  expect(await page.locator(".feed img, .feed script").count()).toBe(0);
  expect(await page.evaluate(() => window.injected)).toBeUndefined();
  await page.screenshot({
    path: info.outputPath("events-mobile.png"),
    fullPage: true,
  });
  await button.press("Enter");
  await expect(details).toHaveCount(0);
  await row.locator("td").nth(1).click();
  await expect(details).toBeVisible();
  await page.setViewportSize({ width: 320, height: 812 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: info.outputPath("events-small-phone.png"),
    fullPage: true,
  });
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.screenshot({
    path: info.outputPath("events-desktop.png"),
    fullPage: true,
  });
  await page.getByRole("button", { name: "Earlier events" }).click();
  await expect(table.locator("tr.event-row")).toHaveCount(8);
  await expect(details).toHaveCount(0);
  await page.getByRole("button", { name: "Latest events" }).click();
  await expect(table.locator("tr.event-row")).toHaveCount(20);
  await button.click();
  const token = readFileSync(
    join(process.env.SILKBOARD_E2E_DIR, "publisher.token"),
    "utf8",
  ).trim();
  const result = await context.request.post(
    "/api/projects/silkboard/progress",
    {
      headers: { Authorization: `Bearer ${token}` },
      data: {
        event_id: "browser-refresh",
        hostname: "development-vm",
        tool: "codex",
        session_id: "n".repeat(256),
        sequence: 26,
        message: "New progress arrived",
      },
    },
  );
  expect(result.ok()).toBe(true);
  await expect(table.locator("tr.event-row").first()).toContainText(
    "New progress arrived",
    { timeout: 15000 },
  );
  await expect(details.getByText(hostile, { exact: true })).toBeVisible();
  await checkHierarchy(page);
  await checkFilters(page, false);
});

test("revocation clears events and a status-page token in the same owner session", async ({
  page,
  context,
}) => {
  await page.goto("/");
  await page.getByLabel("Owner password").fill(password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page.locator("tr.event-row")).toHaveCount(20);
  await page.locator(".event-preview").first().click();
  await expect(
    page.getByRole("region", { name: "Event details" }),
  ).toBeVisible();
  const statusPage = await context.newPage();
  await statusPage.goto("/status");
  await expect(statusPage.getByText("Healthy", { exact: true })).toBeVisible();
  await statusPage.getByRole("button", { name: "Navigation menu" }).click();
  await statusPage
    .getByRole("button", { name: "Manage publisher credentials" })
    .click();
  await statusPage.getByLabel("Credential project").fill("status-cleanup");
  await statusPage.getByLabel("Installation label").fill("status-expiry");
  await statusPage
    .getByRole("button", { name: "Issue token", exact: true })
    .click();
  await expect(statusPage.getByLabel("New publisher token")).toBeVisible();
  const state = JSON.parse(
    readFileSync(join(process.env.SILKBOARD_E2E_DIR, "state.json"), "utf8"),
  );
  execFileSync(state.binary, ["owner", "set-password"], {
    env: { ...process.env, SILKBOARD_DATABASE: state.database },
    input: password,
  });
  await expect(page.getByRole("alert")).toContainText("Your session expired", {
    timeout: 15000,
  });
  await expect(page.locator("tr.event-row")).toHaveCount(0);
  await expect(page.getByRole("region", { name: "Event details" })).toHaveCount(
    0,
  );
  await expect(statusPage.getByRole("alert")).toContainText(
    "Your session expired",
    { timeout: 15000 },
  );
  await expect(statusPage.getByLabel("New publisher token")).toHaveCount(0);
  await expect(statusPage.getByLabel("Credential project")).toHaveCount(0);
  await expect(statusPage.getByText("Healthy", { exact: true })).toBeVisible();
  await statusPage.getByRole("button", { name: "Navigation menu" }).click();
  await expect(
    statusPage.getByRole("button", { name: "Sign out", exact: true }),
  ).toHaveCount(0);
  await expect(
    statusPage.getByRole("button", { name: "Manage publisher credentials" }),
  ).toHaveCount(0);
  await statusPage.close();
});

// Reuse the authenticated phone scenario rather than adding sign-in attempts
// to the shared server fixture and triggering its real admission limit.
async function checkHierarchy(page) {
  await page.setViewportSize({ width: 320, height: 812 });
  await expect(page.locator("tr.event-row")).toHaveCount(20);
  await chooseFilter(page, "Project", "silk", "silkboard");
  await chooseFilter(page, "Agent", "codex", "codex · development-vm");
  await expect(page.locator("tr.event-row")).toHaveCount(20);
  const switcher = page.getByRole("button", {
    name: "Session hierarchy",
    exact: true,
  });
  await switcher.focus();
  await page.keyboard.press("Enter");
  await expect(switcher).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator(".session-group.nested")).toHaveCount(1);
  const parent = page.getByRole("button", {
    name: /Session group: codex implementation/,
    exact: true,
  });
  const child = page.getByRole("button", {
    name: /Session group: Independent review, subagent of codex implementation/,
    exact: true,
  });
  await child.focus();
  await page.keyboard.press("Enter");
  await expect(child).toHaveAttribute("aria-expanded", "false");
  await expect(parent).toHaveAttribute("aria-expanded", "true");
  await child.click();
  const event = page.locator("tr.event-row.nested button.event-preview");
  await event.click();
  await expect(
    page.getByRole("region", { name: "Event details" }),
  ).toBeVisible();
  await parent.click();
  await expect(page.locator("tr.event-row")).toHaveCount(0);
  await expect(child).toHaveCount(0);
  await parent.click();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: "test-results/events-hierarchy-phone.png",
    fullPage: true,
  });
  await page
    .getByRole("button", { name: "Earlier events", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Latest events", exact: true }),
  ).toBeVisible();
  const olderCount = await page.locator("tr.event-row").count();
  expect(olderCount).toBeGreaterThan(0);
  expect(olderCount).toBeLessThan(20);
  await page.getByRole("button", { name: "Flat list", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Project: silkboard" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Agent: codex · development-vm" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Latest events", exact: true }),
  ).toBeVisible();
  await expect(page.locator("tr.event-row")).toHaveCount(olderCount);
  await page
    .getByRole("button", { name: "Session hierarchy", exact: true })
    .click();
  await expect(page.locator("tr.event-row")).toHaveCount(olderCount);
  await expect(page.locator(".session-group.nested")).toHaveCount(0);
  await page
    .getByRole("button", { name: "Latest events", exact: true })
    .click();
  await expect(page.locator("tr.event-row")).toHaveCount(20);
  await page.getByLabel("From", { exact: true }).fill("2099-01-01T00:00");
  await expect(
    page.getByText("No events match these filters on this page."),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Session hierarchy", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  await page.getByRole("button", { name: "Clear filters" }).click();
  await page.getByRole("button", { name: "Flat list", exact: true }).click();
  await expect(page.locator("tr.event-row")).toHaveCount(20);
}
