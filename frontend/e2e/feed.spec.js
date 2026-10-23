import { expect, test } from "@playwright/test";
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
  await expect(table.getByRole("columnheader")).toHaveText([
    "Time",
    "Project",
    "Agent",
    "Message",
  ]);
  await expect(page.getByLabel("Project", { exact: true })).toHaveCount(0);
  await expect(table.getByText("bombyx", { exact: true })).toBeVisible();
  expect(
    await table.getByText("silkboard", { exact: true }).count(),
  ).toBeGreaterThan(0);
  expect(await page.getByText("n".repeat(256), { exact: true }).count()).toBe(
    0,
  );
  const details = page.getByRole("region", { name: "Event details" });
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
});

test("resetting the owner password removes all visible events and details on refresh", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByLabel("Owner password").fill(password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page.locator("tr.event-row")).toHaveCount(20);
  await page.locator(".event-preview").first().click();
  await expect(
    page.getByRole("region", { name: "Event details" }),
  ).toBeVisible();
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
});
