import { fixturePort } from "./ports.mjs";
import { expect, test } from "@playwright/test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { password } from "./fixture.mjs";

test("phone registration consumes a private link before enabling authenticated publishing", async ({
  page,
  context,
}, info) => {
  const origin = `https://127.0.0.1:${fixturePort(38126)}`;
  const directory = join(process.env.SILKBOARD_E2E_DIR, "setup");
  const link = readFileSync(join(directory, "link"), "utf8").trim();
  const token = readFileSync(join(directory, "publisher.token"), "utf8").trim();
  const urls = [];
  page.on("request", (request) => urls.push(request.url()));
  const publish = () =>
    context.request.post(origin + "/api/projects/silkboard/progress", {
      headers: { Authorization: `Bearer ${token}` },
      data: {
        event_id: "setup-mobile",
        hostname: "dev",
        tool: "codex",
        session_id: "native",
        sequence: 1,
        message: "Owner registration succeeded",
      },
    });
  expect((await publish()).status()).toBe(503);
  await page.setViewportSize({ width: 375, height: 812 });
  const document = await page.goto(origin + "/setup");
  expect(document.headers()["cache-control"]).toBe("no-store");
  expect(document.headers()["referrer-policy"]).toBe("no-referrer");
  expect(document.headers()["content-security-policy"]).toContain(
    "frame-ancestors 'none'",
  );
  await expect(page.getByRole("alert")).toContainText("private setup link");
  await page.goto(origin + "/setup#setup=" + "0".repeat(64));
  await page.getByLabel("New owner password").fill(password);
  await page.getByLabel("Confirm password").fill(password);
  await page.getByRole("button", { name: "Create owner account" }).click();
  await expect(page.getByRole("alert")).toContainText("expired");
  expect((await publish()).status()).toBe(503);
  await page.goto(link);
  await expect(page.getByLabel("New owner password")).toBeVisible();
  expect(page.url()).toBe(origin + "/setup");
  expect(
    await page.evaluate(() => [
      history.state,
      localStorage.length,
      sessionStorage.length,
      document.referrer,
    ]),
  ).toEqual([null, 0, 0, ""]);
  await page.reload();
  await expect(page.getByRole("alert")).toContainText("private setup link");
  await page.goto(link);
  await page.getByLabel("New owner password").fill(password);
  await page.getByLabel("Confirm password").fill(password);
  await page.screenshot({
    path: info.outputPath("owner-setup-mobile.png"),
    fullPage: true,
  });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.getByRole("button", { name: "Create owner account" }).click();
  await expect(page.getByRole("status")).toContainText("Owner registered");
  expect((await publish()).status()).toBe(200);
  expect(
    urls.every(
      (url) => !url.includes("#") && !url.includes(link.split("#setup=")[1]),
    ),
  ).toBe(true);
  await page.getByRole("link", { name: "Sign in to Silkboard" }).click();
  await page.getByLabel("Owner password").fill(password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(
    page.getByText("Owner registration succeeded", { exact: true }),
  ).toBeVisible();
  const cookie = (await context.cookies()).find(
    (cookie) => cookie.name === "__Host-silkboard-owner",
  );
  expect(cookie).toMatchObject({
    secure: true,
    httpOnly: true,
    sameSite: "Strict",
  });
  await page.goto(link);
  await expect(page.getByRole("status")).toContainText("already complete");
  expect(await page.locator("input").count()).toBe(0);
});
