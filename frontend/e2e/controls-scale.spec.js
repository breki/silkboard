import { expect, test } from "@playwright/test";
import { agent } from "../test/agents.js";
import { password } from "./fixture.mjs";

const projects = Array.from(
  { length: 10 },
  (_, i) => `project-${String(i + 1).padStart(2, "0")}`,
);
const samples = Array.from({ length: 20 }, (_, i) => ({
  ...agent,
  project: projects[Math.floor(i / 2)],
  session_id: i === 18 ? "scale-0" : `scale-${i}`,
  session_name: `Task ${String(i + 1).padStart(2, "0")}`,
  received_at: Date.now() - i * 60000,
}));
const noOverflow = async (page) =>
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);

test("approved controls scale to ten projects and twenty agents with keyboard and phone range editing", async ({
  page,
}, info) => {
  // Browser layout fixtures complement the real API filtering and identity tests.
  await page.route("**/api/owner/agents**", async (route) => {
    const q = new URL(route.request().url()).searchParams;
    const items = samples.filter(
      (a) =>
        (!q.get("project") || a.project === q.get("project")) &&
        (!q.get("session_id") || a.session_id === q.get("session_id")) &&
        (!q.get("q") ||
          `${a.session_name} ${a.tool} ${a.hostname}`
            .toLowerCase()
            .includes(q.get("q").toLowerCase())),
    );
    await route.fulfill({ json: { items, next_cursor: null } });
  });
  await page.route("**/api/owner/filter-options?**", async (route) => {
    const q = new URL(route.request().url()).searchParams;
    if (q.get("view") !== "agents") return route.continue();
    await route.fulfill({
      json: {
        items: projects.filter((p) => p.includes(q.get("q") || "")),
        next_cursor: null,
      },
    });
  });
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto("/");
  await page.getByLabel("Owner password").fill(password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  const cards = page.getByRole("list", { name: "Main agents" });
  await expect(cards.getByRole("article")).toHaveCount(20);
  const rail = page.getByRole("complementary", { name: "Projects" });
  await expect(rail.getByRole("button")).toHaveCount(11);
  await rail.getByRole("button", { name: "project-09", exact: true }).focus();
  await page.keyboard.press("Enter");
  await expect(cards.getByRole("article")).toHaveCount(2);
  await expect(
    rail.getByRole("button", { name: "project-09", exact: true }),
  ).toBeFocused();
  await page.getByRole("button", { name: "Clear filters" }).click();
  await expect(cards.getByRole("article")).toHaveCount(20);
  await page.screenshot({
    path: info.outputPath("approved-agents-desktop.png"),
    fullPage: true,
  });

  await page.getByRole("button", { name: "Agent: All agents" }).click();
  const search = page.getByLabel("Search agents", { exact: true });
  await expect(search).toBeFocused();
  await search.fill("Task 19");
  await page
    .getByRole("list", { name: "Available agents" })
    .getByRole("button", { name: /Task 19/ })
    .click();
  await expect(cards.getByRole("article")).toHaveCount(1);
  await expect(cards.getByText("project-01", { exact: true })).toHaveCount(0);
  await cards.locator("summary").click();
  await expect(cards.locator("code")).toHaveText("scale-0");

  await page.getByRole("button", { name: "Clear filters" }).click();
  await page.setViewportSize({ width: 320, height: 780 });
  await page.getByRole("button", { name: "Navigation menu" }).click();
  await page
    .getByRole("combobox", { name: "Theme", exact: true })
    .selectOption("dark");
  await page.keyboard.press("Escape");
  await noOverflow(page);
  await expect(
    page.getByRole("button", { name: "Project: All projects" }),
  ).toBeVisible();
  await page.screenshot({
    path: info.outputPath("approved-agents-phone-dark.png"),
  });
  await page.getByRole("link", { name: "Events", exact: true }).click();
  const trigger = page.getByRole("button", { name: /^Received time:/ });
  await trigger.click();
  const dialog = page.getByRole("dialog", {
    name: "Received time",
    exact: true,
  });
  await expect(dialog).toBeVisible();
  await dialog.getByRole("button", { name: "Close received time" }).focus();
  await page.keyboard.press("Shift+Tab");
  await expect(
    dialog.getByRole("button", { name: "Apply range" }),
  ).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(
    dialog.getByRole("button", { name: "Close received time" }),
  ).toBeFocused();
  for (const label of ["From date", "From time", "To date", "To time"])
    await expect(dialog.getByLabel(label, { exact: true })).toHaveCount(1);
  await dialog.getByLabel("From date").fill("2026-02-30");
  await dialog.getByLabel("From time").fill("09:00");
  await dialog.getByRole("button", { name: "Apply range" }).click();
  await expect(dialog.getByRole("alert")).toContainText(
    "last valid time range stays active",
  );
  await noOverflow(page);
  expect(await dialog.evaluate((d) => d.scrollWidth <= d.clientWidth)).toBe(
    true,
  );
  const targets = await dialog.locator("button, input").evaluateAll((nodes) =>
    nodes.map((n) => {
      const r = n.getBoundingClientRect();
      return { width: r.width, height: r.height };
    }),
  );
  expect(targets.every((r) => r.width >= 44 && r.height >= 44)).toBe(true);
  await page.screenshot({
    path: info.outputPath("approved-range-phone-dark.png"),
  });
  await page.keyboard.press("Escape");
  await expect(trigger).toBeFocused();
  await expect(trigger).toHaveAccessibleName("Received time: All time");
  await trigger.click();
  await dialog.getByRole("button", { name: "Last hour", exact: true }).click();
  await expect(trigger).toHaveAccessibleName("Received time: Last hour");
  await page.getByRole("button", { name: "Clear filters" }).focus();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("button", { name: "Clear filters" }),
  ).toBeFocused();
  await expect(trigger).toHaveAccessibleName("Received time: All time");
  await page.evaluate(() => {
    document.documentElement.style.fontSize = "200%";
  });
  await noOverflow(page);
});
