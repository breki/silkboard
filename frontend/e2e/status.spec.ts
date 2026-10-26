import { expect, test } from "@playwright/test";

test("built page reports the backend identity and works on mobile", async ({
  page,
  request,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const response = await request.get("/api/status");
  expect(response.ok()).toBeTruthy();
  const status = await response.json();
  await page.goto("/status");
  await expect(page.getByText("Healthy", { exact: true })).toBeVisible();
  await expect(page.locator("dd")).toHaveText([
    status.version,
    status.release_tag,
    status.commit,
  ]);
  await page.setViewportSize({ width: 375, height: 812 });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  expect(errors).toEqual([]);
});

test("failed status request gives a recovery action", async ({ page }) => {
  await page.route("**/api/status", (route) =>
    route.fulfill({ status: 503, body: "unavailable" }),
  );
  await page.goto("/status");
  await expect(page.getByRole("alert")).toContainText("Reload to try again");
  await expect(page.getByText("Healthy", { exact: true })).toHaveCount(0);
});

test("primary header links and compact menu support keyboards and small phones", async ({
  page,
}, info) => {
  await page.setViewportSize({ width: 320, height: 720 });
  await page.goto("/");
  await expect(page.getByLabel("Owner password")).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Agent progress", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("heading", { name: "Application status", exact: true }),
  ).toHaveCount(0);
  expect(
    await page
      .locator("header")
      .evaluate((node) => node.getBoundingClientRect().height),
  ).toBeLessThanOrEqual(104);
  const navBox = await page
    .getByRole("navigation", { name: "Primary navigation" })
    .boundingBox();
  const brandBox = await page
    .getByRole("heading", { name: "Silkboard", exact: true })
    .boundingBox();
  expect(navBox!.y).toBeGreaterThanOrEqual(brandBox!.y + brandBox!.height);
  // Exercise a wider fallback font as well as the platform's default face.
  const widerFont = await page.addStyleTag({
    content: ".masthead { font-family: monospace; }",
  });
  expect(
    await page
      .locator("header")
      .evaluate((node) => node.getBoundingClientRect().height),
  ).toBeLessThanOrEqual(104);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await widerFont.evaluate((node) => node.parentNode?.removeChild(node));
  const primary = page.getByRole("navigation", { name: "Primary navigation" });
  const events = primary.getByRole("link", { name: "Events", exact: true });
  const plans = primary.getByRole("link", { name: "Workflows", exact: true });
  await expect(events).toBeVisible();
  await expect(plans).toBeVisible();
  await events.focus();
  await page.keyboard.press("Tab");
  await expect(plans).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/\/plans$/);
  await expect(page.getByLabel("Owner password")).toBeVisible();
  await page.goBack();
  await expect(page).toHaveURL(/\/$/);
  const menu = page.getByRole("button", { name: "Navigation menu" });
  await menu.focus();
  await page.keyboard.press("Enter");
  await expect(menu).toHaveAttribute("aria-expanded", "true");
  await expect(
    page.getByRole("button", { name: "Sign out", exact: true }),
  ).toHaveCount(0);
  await page.keyboard.press("Escape");
  await expect(menu).toBeFocused();
  await expect(menu).toHaveAttribute("aria-expanded", "false");
  await page.keyboard.press("Space");
  await page.keyboard.press("Tab");
  const drawer = page.getByRole("navigation", { name: "Main navigation" });
  await expect(
    drawer.getByRole("link", { name: "Events", exact: true }),
  ).toHaveCount(0);
  await expect(
    drawer.getByRole("link", { name: "Workflows", exact: true }),
  ).toHaveCount(0);
  await expect(
    drawer.getByRole("link", { name: "Todos", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("link", { name: "Application status" }),
  ).toBeFocused();
  await page.screenshot({
    path: info.outputPath("menu-small-phone.png"),
    fullPage: true,
  });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/\/status$/);
  await expect(page.getByText("Healthy", { exact: true })).toBeVisible();
  await expect(page.getByLabel("Owner password")).toHaveCount(0);
  await page.reload();
  await expect(page.getByText("Healthy", { exact: true })).toBeVisible();
  await page.screenshot({
    path: info.outputPath("status-small-phone.png"),
    fullPage: true,
  });
  await page.goBack();
  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByLabel("Owner password")).toBeVisible();
  await menu.click();
  await page.getByRole("link", { name: "Application status" }).click();
  await events.click();
  await expect(page.getByLabel("Owner password")).toBeVisible();
  await page.setViewportSize({ width: 1280, height: 900 });
  await expect(events).toBeVisible();
  await expect(plans).toBeVisible();
  await menu.click();
  await page.screenshot({
    path: info.outputPath("menu-desktop.png"),
    fullPage: true,
  });
  await page.getByRole("heading", { name: "Silkboard", exact: true }).click();
  await expect(menu).toHaveAttribute("aria-expanded", "false");
});
