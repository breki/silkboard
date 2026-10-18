import { expect, test } from "@playwright/test";
import { readFileSync } from "node:fs";
import { password } from "./fixture.mjs";

test("phone owner issues, saves once and revokes a project-scoped publisher token", async ({
  page,
  context,
}, info) => {
  const endpoint = "/api/owner/projects/token-ui/publishers";
  expect((await context.request.get(endpoint)).status()).toBe(401);
  await page.setViewportSize({ width: 375, height: 812 });
  const document = await page.goto("/");
  expect(document.headers()["cache-control"]).toBe("no-store");
  expect(document.headers()["x-frame-options"]).toBe("DENY");
  await page.getByLabel("Owner password").fill(password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await page
    .getByRole("button", { name: "Manage publisher credentials" })
    .click();
  await page.getByLabel("Credential project").fill("token-ui");
  await page.getByLabel("Installation label").fill("mobile-install");
  await page.getByRole("button", { name: "Issue token", exact: true }).click();
  const field = page.getByLabel("New publisher token");
  await expect(field).toBeVisible();
  const token = await field.inputValue();
  expect(token).toMatch(/^[0-9a-f]{64}$/);
  const listed = await context.request.get(endpoint);
  expect(listed.headers()["cache-control"]).toBe("no-store");
  expect(await listed.json()).toEqual({
    items: [{ label: "mobile-install", revoked: false }],
    next_cursor: null,
  });
  expect(page.url()).not.toContain(token);
  expect(
    await page.evaluate(() => [
      localStorage.length,
      sessionStorage.length,
      history.state,
    ]),
  ).toEqual([0, 0, null]);
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.getByRole("button", { name: "Copy token", exact: true }).click();
  await expect(page.getByText(/Copied. Install it privately/)).toBeVisible();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(token);
  const pending = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download token file" }).click();
  const download = await pending;
  expect(download.suggestedFilename()).toBe("silkboard-publisher.token");
  expect(readFileSync(await download.path(), "utf8")).toBe(token + "\n");
  await page.screenshot({
    path: info.outputPath("publisher-tokens-mobile.png"),
    fullPage: true,
    mask: [field],
  });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  expect(
    (
      await context.request.get(endpoint, {
        headers: { Cookie: "", Authorization: `Bearer ${token}` },
      })
    ).status(),
  ).toBe(401);
  const publish = () =>
    context.request.post("/api/projects/token-ui/progress", {
      headers: { Authorization: `Bearer ${token}` },
      data: {
        event_id: "token-ui-progress",
        hostname: "dev",
        tool: "codex",
        session_id: "token-ui",
        sequence: 1,
        message: "Saved with mobile token",
      },
    });
  expect((await publish()).status()).toBe(200);
  await page.getByRole("button", { name: "I’ve saved it" }).click();
  await expect(field).toHaveCount(0);
  await page.getByRole("button", { name: "Revoke mobile-install" }).click();
  await page.getByRole("button", { name: "Confirm revocation" }).click();
  await expect(page.getByRole("alert")).toContainText("Credential revoked");
  await expect(page.getByText(/— Revoked/)).toBeVisible();
  expect((await publish()).status()).toBe(401);
  const sessions = await context.request.get(
    "/api/owner/projects/token-ui/sessions",
  );
  expect((await sessions.json()).items[0].latest.message).toBe(
    "Saved with mobile token",
  );
  await page.reload();
  await page
    .getByRole("button", { name: "Manage publisher credentials" })
    .click();
  expect(await page.getByLabel("New publisher token").count()).toBe(0);
  await page.getByLabel("Credential project").fill("token-ui");
  await page.getByRole("button", { name: "Load credentials" }).click();
  await expect(page.getByText(/— Revoked/)).toBeVisible();
});
