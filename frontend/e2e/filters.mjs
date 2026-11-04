import { expect } from "@playwright/test";
const noOverflow = async (page) =>
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
export async function chooseFilter(page, kind, search, value) {
  await page.getByRole("button", { name: new RegExp(`^${kind}:`) }).click();
  const field = page.getByLabel(
    `Search ${kind === "Project" ? "projects" : "agents"}`,
  );
  await expect(field).toBeFocused();
  await field.fill(search);
  const option = page
    .getByRole("list", {
      name: `Available ${kind === "Project" ? "projects" : "agents"}`,
    })
    .getByRole("button", { name: value, exact: true });
  await expect(option).toBeVisible();
  await noOverflow(page);
  await option.focus();
  await page.keyboard.press("Enter");
}
export async function checkFilters(page, workflows) {
  await page.setViewportSize({ width: 320, height: 720 });
  await expect(
    page.getByRole("group", {
      name: workflows ? "Workflow filters" : "Event filters",
    }),
  ).toBeVisible();
  const endpoint = workflows ? "/api/owner/plans" : "/api/owner/events";
  const selected = page.waitForResponse(
    (r) =>
      new URL(r.url()).pathname === endpoint &&
      new URL(r.url()).searchParams.get("project") === "silkboard",
  );
  await chooseFilter(page, "Project", "silk", "silkboard");
  expect((await selected).status()).toBe(200);
  await expect(page.getByRole("button", { name: "Apply filters" })).toHaveCount(
    0,
  );
  if (workflows) {
    const status = page.waitForResponse(
      (r) =>
        new URL(r.url()).pathname === endpoint &&
        new URL(r.url()).searchParams.get("state") === "blocked",
    );
    await page.getByRole("button", { name: "Blocked", exact: true }).click();
    expect((await status).status()).toBe(200);
    await expect(
      page.getByRole("button", { name: "Blocked", exact: true }),
    ).toHaveAttribute("aria-pressed", "true");
    await page
      .getByRole("button", { name: "All statuses", exact: true })
      .click();
  } else {
    const timed = page.waitForResponse(
      (r) =>
        new URL(r.url()).pathname === endpoint &&
        new URL(r.url()).searchParams.has("from"),
    );
    await page.getByRole("button", { name: /^Received time:/ }).click();
    await page.getByLabel("From date", { exact: true }).fill("2099-01-01");
    await page.getByLabel("From time", { exact: true }).fill("00:00");
    await page.getByRole("button", { name: "Apply range" }).click();
    expect((await timed).status()).toBe(200);
    await expect(
      page.getByText("No events match these filters on this page."),
    ).toBeVisible();
  }
  await noOverflow(page);
  const cleared = page.waitForResponse(
    (r) => new URL(r.url()).pathname === endpoint && !new URL(r.url()).search,
  );
  await page.getByRole("button", { name: "Clear filters" }).click();
  expect((await cleared).status()).toBe(200);
  await expect(
    page.getByRole("button", { name: "Project: All projects" }),
  ).toBeVisible();
  if (!workflows) {
    await chooseFilter(page, "Project", "silk", "silkboard");
    await expect(page.locator("tr.event-row")).toHaveCount(20);
    await page.getByRole("button", { name: "Earlier events" }).click();
    await expect
      .poll(() => page.locator("tr.event-row").count())
      .toBeLessThan(20);
    await chooseFilter(page, "Agent", "claude", "claude · development-vm");
    await expect(page.locator("tr.event-row")).toHaveCount(1);
    await expect(
      page.getByRole("button", { name: "Latest events" }),
    ).toHaveCount(0);
  }
}
