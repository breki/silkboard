import { expect, test } from "@playwright/test";
import { fixturePort } from "./ports.mjs";
import { password } from "./fixture.mjs";
test.use({ baseURL: `https://127.0.0.1:${fixturePort(38128)}` });
test("owner checklist persists nested edits and explains promotion on a phone", async ({
  page,
  context,
}, info) => {
  await page.clock.install();
  await page.setViewportSize({ width: 320, height: 720 });
  expect((await context.request.get("/api/owner/todos")).status()).toBe(401);
  await page.goto("/todos");
  await page.getByLabel("Owner password").fill(password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Todos", exact: true }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Agents", exact: true }).click();
  await page.getByRole("link", { name: "Todos", exact: true }).focus();
  await page.keyboard.press("Enter");
  await page.getByLabel("New todo", { exact: true }).fill("Prepare release");
  await page.getByRole("button", { name: "Add todo", exact: true }).click();
  const parent = page.getByRole("checkbox", {
    name: "Prepare release",
    exact: true,
  });
  await expect(parent).toBeVisible();
  await parent.focus();
  await page.keyboard.press("Space");
  await expect(parent).toBeChecked();
  await expect(parent).toBeFocused();
  await page
    .getByRole("button", { name: "Add child to Prepare release", exact: true })
    .click();
  await expect(page.getByLabel("New child of Prepare release")).toBeFocused();
  await page.getByLabel("New child of Prepare release").fill("Check package");
  await page.getByRole("button", { name: "Add todo", exact: true }).click();
  const child = page.getByRole("checkbox", {
    name: "Check package",
    exact: true,
  });
  await expect(child).not.toBeChecked();
  expect((await child.boundingBox()).x).toBeGreaterThan(
    (await parent.boundingBox()).x,
  );
  await page
    .getByRole("button", { name: "Edit Prepare release", exact: true })
    .click();
  await page.getByLabel("Edit todo text").fill("Ship release");
  await page.getByRole("button", { name: "Save text", exact: true }).click();
  const edited = page.getByRole("checkbox", {
    name: "Ship release",
    exact: true,
  });
  await expect(edited).toBeChecked();
  await page.reload();
  await expect(edited).toBeChecked();
  await expect(child).not.toBeChecked();
  await page
    .getByRole("button", { name: "Delete Ship release", exact: true })
    .click();
  const confirm = page.getByRole("region", { name: "Confirm todo deletion" });
  await expect(confirm).toContainText(
    "children will be kept and moved up one level",
  );
  await confirm
    .getByRole("button", { name: "Confirm delete", exact: true })
    .click();
  await expect(edited).toHaveCount(0);
  await expect(child).toBeVisible();
  await page.getByLabel("New todo", { exact: true }).fill("Unsaved draft");
  await page.route("**/api/owner/todos/*", (route) =>
    route.fulfill({ status: 503, contentType: "application/json", body: "{}" }),
  );
  await child.click();
  await expect(page.getByRole("alert")).toContainText("Unable to confirm");
  await expect(child).not.toBeChecked();
  await expect(page.getByLabel("New todo", { exact: true })).toHaveValue(
    "Unsaved draft",
  );
  await page.unroute("**/api/owner/todos/*");
  await page.getByRole("button", { name: "Reload todos", exact: true }).click();
  await expect(child).toBeEnabled();
  await expect(page.getByLabel("New todo", { exact: true })).toHaveValue(
    "Unsaved draft",
  );
  for (let level = 2; level <= 5; level++) {
    const previous = level === 2 ? "Check package" : `Level ${level - 1}`;
    await page
      .getByRole("button", { name: `Add child to ${previous}`, exact: true })
      .click();
    await page
      .getByLabel(`New child of ${previous}`, { exact: true })
      .fill(`Level ${level}`);
    await page.getByRole("button", { name: "Add todo", exact: true }).click();
    await expect(
      page.getByRole("checkbox", { name: `Level ${level}`, exact: true }),
    ).toBeVisible();
  }
  await expect(
    page.getByRole("button", { name: "Add child to Level 5", exact: true }),
  ).toHaveCount(0);
  const overflow = await page.evaluate(() =>
    [...document.querySelectorAll("main *")]
      .filter((node) => node.getBoundingClientRect().right > innerWidth + 1)
      .map((node) => ({
        tag: node.tagName,
        cls: node.className,
        right: node.getBoundingClientRect().right,
        width: node.getBoundingClientRect().width,
      }))
      .slice(0, 15),
  );
  expect(overflow).toEqual([]);
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth),
  ).toBeLessThanOrEqual(320);
  await page.screenshot({
    path: info.outputPath("todos-phone.png"),
    fullPage: true,
  });
  await page.route("**/api/owner/session", (route) =>
    route.fulfill({ status: 401, body: "{}", contentType: "application/json" }),
  );
  await page.clock.fastForward(10000);
  await expect(page.getByLabel("Owner password")).toBeVisible();
  await expect(child).toHaveCount(0);
});
