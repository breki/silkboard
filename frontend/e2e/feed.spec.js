import { expect, test } from "@playwright/test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { execFileSync } from "node:child_process";
import { password, hostile } from "./fixture.mjs";

test("owner login uses a secure cookie and logout revokes feed access", async ({
  page,
  context,
}) => {
  expect((await context.request.get("/api/owner/projects")).status()).toBe(401);
  await page.goto("/");
  await page.getByLabel("Owner password").fill("incorrect fixture password");
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("Password not accepted");
  await page.getByLabel("Owner password").fill(password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Sessions", exact: true }),
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
  expect((await context.request.get("/api/owner/projects")).status()).toBe(401);
});

test("phone feed separates identities, escapes content, pages history and refreshes", async ({
  page,
  context,
}, info) => {
  await page.setViewportSize({ width: 375, height: 812 });
  await page.goto("/");
  await page.getByLabel("Owner password").fill(password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page.locator("article.session-card")).toHaveCount(3);
  const card = page.getByRole("article").filter({
    has: page.getByRole("heading", {
      name: "codex implementation",
      exact: true,
    }),
  });
  await expect(card.getByText(hostile, { exact: true })).toBeVisible();
  expect(await page.locator(".feed img, .feed script").count()).toBe(0);
  await card.getByRole("button", { name: "View history" }).click();
  await expect(card.locator("li")).toHaveCount(20);
  await card.getByRole("button", { name: "Earlier messages" }).click();
  await expect(card.locator("li")).toHaveCount(5);
  await card.getByRole("button", { name: "Latest messages" }).click();
  await expect(card.locator("li")).toHaveCount(20);
  await page.screenshot({
    path: info.outputPath("owner-mobile.png"),
    fullPage: true,
  });
  await page.setViewportSize({ width: 320, height: 812 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
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
  await expect(card.locator(".latest-message")).toHaveText(
    "New progress arrived",
    { timeout: 15000 },
  );
  await expect(card.locator("li").first()).toContainText(
    "New progress arrived",
  );
});

test("resetting the owner password removes a visible feed on refresh", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByLabel("Owner password").fill(password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page.locator("article.session-card")).toHaveCount(3);
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
  await expect(page.locator("article.session-card")).toHaveCount(0);
});
