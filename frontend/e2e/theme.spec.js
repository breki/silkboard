import { expect, test } from "@playwright/test";
import { password } from "./fixture.mjs";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { publish } from "./plans-fixture.mjs";
import { detail } from "../test/plans.js";

const appearance = async (page, mode) =>
  expect
    .poll(() =>
      page.evaluate(
        () => getComputedStyle(document.documentElement).backgroundColor,
      ),
    )
    .toBe(mode === "light" ? "rgb(244, 247, 250)" : "rgb(12, 23, 30)");
async function choose(page, value) {
  if (
    !(await page.getByRole("combobox", { name: "Theme", exact: true }).count())
  )
    await page.getByRole("button", { name: "Navigation menu" }).click();
  await page
    .getByRole("combobox", { name: "Theme", exact: true })
    .selectOption(value);
  await page.keyboard.press("Escape");
}
const noOverflow = async (page) =>
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
async function contrast(
  page,
  selector,
  backdrop,
  property = "color",
  minimum = 4.5,
) {
  const ratio = await page
    .locator(selector)
    .first()
    .evaluate(
      (node, args) => {
        const luminance = (color) => {
          const c = color
            .match(/[\d.]+/g)
            .slice(0, 3)
            .map(Number)
            .map((x) => {
              const v = x / 255;
              return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
            });
          return c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722;
        };
        const a = luminance(
          getComputedStyle(node).getPropertyValue(args.property),
        );
        const b = luminance(
          getComputedStyle(document.querySelector(args.backdrop))
            .backgroundColor,
        );
        return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
      },
      { backdrop, property },
    );
  expect(ratio, `${selector} ${property} contrast`).toBeGreaterThanOrEqual(
    minimum,
  );
}

test("System follows OS changes, overrides persist, and setup exposes theme selection", async ({
  page,
}, info) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.setViewportSize({ width: 320, height: 720 });
  await page.goto("/");
  await expect(page.getByLabel("Owner password")).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "system");
  await appearance(page, "light");
  await page.emulateMedia({ colorScheme: "dark" });
  await appearance(page, "dark");
  await choose(page, "light");
  await appearance(page, "light");
  await page.reload();
  await appearance(page, "light");
  await page.goto("/status");
  await expect(page.getByText("Healthy", { exact: true })).toBeVisible();
  await appearance(page, "light");
  await choose(page, "dark");
  await page.emulateMedia({ colorScheme: "light" });
  await appearance(page, "dark");
  await page.goto("/setup");
  const selector = page.getByRole("combobox", { name: "Theme", exact: true });
  await expect(selector).toBeVisible();
  await expect(selector).toHaveValue("dark");
  await expect(
    page.getByRole("button", { name: "Navigation menu" }),
  ).toHaveCount(0);
  await selector.selectOption("system");
  await appearance(page, "light");
  await page.emulateMedia({ colorScheme: "dark" });
  await appearance(page, "dark");
  await noOverflow(page);
  await page.screenshot({
    path: info.outputPath("setup-theme-phone.png"),
    fullPage: true,
  });
  await page.goto("/");
  const menu = page.getByRole("button", { name: "Navigation menu" });
  await menu.focus();
  await page.keyboard.press("Enter");
  await page.keyboard.press("Tab");
  await page.keyboard.press("Tab");
  await expect(selector).toBeFocused();
  await page.keyboard.press("End");
  await expect(selector).toHaveValue("dark");
  await page.keyboard.press("Escape");
  await expect(menu).toBeFocused();
  await appearance(page, "dark");
});

test("blocked preference storage does not prevent current-tab switching", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.addInitScript(() => {
    Object.defineProperty(window, "localStorage", {
      get() {
        throw new Error("blocked fixture storage");
      },
    });
  });
  await page.goto("/");
  await appearance(page, "light");
  await choose(page, "dark");
  await appearance(page, "dark");
  await page.getByRole("button", { name: "Navigation menu" }).click();
  await expect(
    page.getByRole("combobox", { name: "Theme", exact: true }),
  ).toHaveValue("dark");
  await page.reload();
  await appearance(page, "light");
});

test("both themes cover authenticated events, credentials and status", async ({
  page,
  context,
}, info) => {
  await page.goto("/");
  await page.getByLabel("Owner password").fill(password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  for (const theme of ["light", "dark"]) {
    await choose(page, theme);
    await appearance(page, theme);
    await expect(
      page.getByRole("table", { name: "Events from all projects" }),
    ).toBeVisible();
    await contrast(page, ".event-preview", ":root");
    await contrast(page, ".muted", ":root");
    await page.getByRole("button", { name: "Navigation menu" }).click();
    await page
      .getByRole("button", { name: "Manage publisher credentials" })
      .click();
    await expect(page.getByLabel("Installation label")).toBeVisible();
    await contrast(page, "input", "input");
    await page.screenshot({
      path: info.outputPath(`credentials-${theme}.png`),
      fullPage: true,
    });
    await page.getByRole("button", { name: "Navigation menu" }).click();
    await page.getByRole("link", { name: "Application status" }).click();
    await expect(page.getByText("Healthy", { exact: true })).toBeVisible();
    await appearance(page, theme);
    await page
      .getByRole("navigation", { name: "Primary navigation" })
      .getByRole("link", { name: "Events", exact: true })
      .click();
    await expect(
      page.getByRole("table", { name: "Events from all projects" }),
    ).toBeVisible();
  }
  expect((await context.request.get("/api/owner/events")).status()).toBe(200);
});

test.describe("graph themes", () => {
  test.use({ baseURL: "https://127.0.0.1:38128" });
  test("graph states, arrows, focus and phone dialogs remain readable in both themes", async ({
    page,
  }, info) => {
    const value = detail({
      task_id: "99999999-9999-4999-8999-999999999999",
      title: "Theme presentation fixture",
    }).snapshot;
    value.steps.push({
      id: "blocked",
      title: "Blocked fixture step",
      description: null,
      depends_on: ["contract"],
      state: "blocked",
      report: null,
      blocked_reason: "Fixture state for checking theme readability",
    });
    await publish(
      value,
      readFileSync(
        join(process.env.SILKBOARD_E2E_DIR, "plans.token"),
        "utf8",
      ).trim(),
    );
    await page.goto(`/plans/silkboard/${value.task_id}`);
    await page.getByLabel("Owner password").fill(password);
    await page.getByRole("button", { name: "Sign in", exact: true }).click();
    await expect(
      page.getByRole("region", { name: "Task prerequisite graph" }),
    ).toBeVisible();
    for (const theme of ["light", "dark"]) {
      await page.setViewportSize({ width: 1280, height: 900 });
      await expect(
        page.getByRole("complementary", { name: "Step details" }),
      ).toBeVisible();
      await choose(page, theme);
      await appearance(page, theme);
      for (const state of ["pending", "active", "done", "blocked"]) {
        const node = `.graph-node[data-state="${state}"]`;
        await expect(page.locator(node).first()).toBeVisible();
        await contrast(page, node, node);
        await contrast(page, node, node, "border-top-color", 3);
      }
      await contrast(page, ".prerequisite-edge", ".graph-scroll", "stroke", 3);
      await page.keyboard.press("Tab");
      await page.locator(".graph-node").first().focus();
      expect(
        await page
          .locator(".graph-node")
          .first()
          .evaluate((node) => getComputedStyle(node).outlineStyle),
      ).toBe("solid");
      await contrast(page, ".graph-node", ".graph-scroll", "outline-color", 3);
      await noOverflow(page);
      await page.screenshot({
        path: info.outputPath(`graph-${theme}-desktop.png`),
        fullPage: true,
      });
      await page.setViewportSize({ width: 320, height: 720 });
      await expect(
        page.getByRole("complementary", { name: "Step details" }),
      ).toHaveCount(0);
      await page.locator(".graph-node").first().click();
      const dialog = page.getByRole("dialog", { name: "Step details" });
      await expect(dialog).toBeVisible();
      await contrast(page, ".step-dialog", ".step-dialog");
      await noOverflow(page);
      await page.screenshot({
        path: info.outputPath(`graph-${theme}-phone-dialog.png`),
        fullPage: true,
      });
      await page.keyboard.press("Escape");
      await expect(dialog).not.toBeVisible();
      await expect(page.locator(".graph-node").first()).toBeFocused();
    }
  });
});
