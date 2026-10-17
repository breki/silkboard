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
  await page.goto("/");
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
  await page.goto("/");
  await expect(page.getByRole("alert")).toContainText("Reload to try again");
  await expect(page.getByText("Healthy", { exact: true })).toHaveCount(0);
});
