import { expect } from "@playwright/test";

export async function checkFilters(page, workflows) {
  await page.setViewportSize({ width: 320, height: 720 });
  await expect(
    page.getByRole("form", {
      name: workflows ? "Workflow filters" : "Event filters",
    }),
  ).toBeVisible();
  await page.getByLabel("Project", { exact: true }).fill("nonexistent");
  await page.getByLabel("Agent", { exact: true }).fill("codex");
  if (workflows)
    await page.getByLabel("Workflow status").selectOption("blocked");
  else await page.getByLabel("From", { exact: true }).fill("2026-01-01T00:00");
  const endpoint = workflows ? "/api/owner/plans" : "/api/owner/events";
  const pending = page.waitForResponse((r) =>
    r.url().includes(`${endpoint}?project=nonexistent&agent=codex`),
  );
  await page.getByRole("button", { name: "Apply filters" }).click();
  expect((await pending).status()).toBe(200);
  await expect(
    page.getByText(/No (events|workflows) match these filters/),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  const cleared = page.waitForResponse(
    (r) => new URL(r.url()).pathname === endpoint && !new URL(r.url()).search,
  );
  await page.getByRole("button", { name: "Clear filters" }).click();
  expect((await cleared).status()).toBe(200);
  await expect(page.getByLabel("Project", { exact: true })).toHaveValue("");
  if (!workflows) {
    await page.getByLabel("Project", { exact: true }).fill("silkboard");
    await page.getByLabel("Agent", { exact: true }).fill("codex");
    await page.getByRole("button", { name: "Apply filters" }).click();
    await expect(page.locator("tr.event-row")).toHaveCount(20);
    await page.getByRole("button", { name: "Earlier events" }).click();
    await expect
      .poll(() => page.locator("tr.event-row").count())
      .toBeGreaterThan(0);
    await expect
      .poll(() => page.locator("tr.event-row").count())
      .toBeLessThan(20);
    await page.getByLabel("Agent", { exact: true }).fill("claude");
    await page.getByRole("button", { name: "Apply filters" }).click();
    await expect(page.locator("tr.event-row")).toHaveCount(1);
    await expect(
      page.getByRole("button", { name: "Latest events" }),
    ).toHaveCount(0);
  }
}
