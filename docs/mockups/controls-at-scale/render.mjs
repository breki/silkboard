import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { resolve } from "node:path";
import { writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const require = createRequire(
  process.env.SILKBOARD_MOCKUP_FRONTEND
    ? resolve(process.env.SILKBOARD_MOCKUP_FRONTEND, "package.json")
    : new URL("../../../frontend/package.json", import.meta.url),
);
const { chromium } = require("@playwright/test");
const { format } = require("prettier");
const browser = await chromium.launch({ headless: true });
const root = fileURLToPath(new URL(".", import.meta.url));
const measurements = [];
const scenarios = [
  { name: "agents-desktop-light", width: 1440, height: 1000 },
  { name: "agents-desktop-dark", width: 1440, height: 1000, dark: true },
  { name: "agents-phone-light", width: 390, height: 844 },
  { name: "agents-phone-320-dark", width: 320, height: 780, dark: true },
  { name: "project-chooser-phone", width: 390, height: 844, open: "project" },
  { name: "events-desktop-light", width: 1440, height: 1000, events: true },
  {
    name: "range-desktop-light",
    width: 1440,
    height: 1000,
    events: true,
    open: "range",
  },
  {
    name: "range-phone-dark",
    width: 320,
    height: 780,
    dark: true,
    events: true,
    open: "range",
  },
];
try {
  for (const scenario of scenarios) {
    const { width, height, name } = scenario;
    const page = await browser.newPage({ viewport: { width, height } });
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.goto(
      new URL(
        `proposal.html${scenario.dark ? "?theme=dark" : ""}${scenario.events ? "#events" : ""}`,
        import.meta.url,
      ).href,
    );
    await page.waitForFunction(() =>
      document.querySelector("#result-count").textContent.startsWith("20"),
    );
    assert.equal(
      await page
        .locator(scenario.events ? ".event-row" : ".agent-card")
        .count(),
      20,
    );
    if (scenario.open) await page.locator(`#${scenario.open}-trigger`).click();
    const geometry = await page.evaluate(() => ({
      width: innerWidth,
      scrollWidth: document.documentElement.scrollWidth,
      dialogOverflow: [...document.querySelectorAll("dialog[open]")].some(
        (d) => d.scrollWidth > d.clientWidth,
      ),
      smallTargets: [
        ...document.querySelectorAll("button, input, nav a, summary"),
      ]
        .filter(
          (e) =>
            e.getClientRects().length &&
            !e.closest("aside") &&
            (!document.querySelector("dialog[open]") ||
              e.closest("dialog[open]")),
        )
        .filter((e) => {
          const r = e.getBoundingClientRect();
          return r.width < 44 || r.height < 44;
        })
        .map((e) => e.textContent.trim() || e.id),
    }));
    assert.equal(geometry.scrollWidth, width, `${name}: page overflow`);
    assert.equal(geometry.dialogOverflow, false, `${name}: dialog overflow`);
    if (width <= 700)
      assert.deepEqual(
        geometry.smallTargets,
        [],
        `${name}: small phone targets`,
      );
    assert.deepEqual(errors, []);
    await page.screenshot({
      path: resolve(root, `${name}.png`),
      fullPage: !scenario.open && width > 700,
    });
    measurements.push({
      name,
      ...geometry,
      agentCount: scenario.events ? undefined : 20,
    });
    await page.close();
  }

  const keyboardPage = await browser.newPage({
    viewport: { width: 1440, height: 1000 },
  });
  await keyboardPage.goto(new URL("proposal.html", import.meta.url).href);
  const projectButton = keyboardPage.locator(
    '.project-choice[data-project="Silkboard"]',
  );
  await projectButton.focus();
  await keyboardPage.keyboard.press("Enter");
  assert.equal(await keyboardPage.locator(".agent-card").count(), 2);
  assert.equal(
    await projectButton.evaluate((e) => e === document.activeElement),
    true,
  );
  await keyboardPage.close();

  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await page.goto(new URL("proposal.html", import.meta.url).href);
  await page.locator("#project-trigger").click();
  await page.locator("#picker-search").fill("silk");
  await page.getByRole("button", { name: /Silkboard.*2 main agents/ }).click();
  assert.equal(await page.locator(".agent-card").count(), 2);
  await page.locator("#agent-trigger").click();
  await page.locator("#picker-search").fill("phone");
  await page
    .getByRole("button", { name: /Check phone filter layouts/ })
    .click();
  assert.equal(await page.locator(".agent-card").count(), 1);
  await page.locator("summary").click();
  assert.match(
    await page.locator(".identity").innerText(),
    /00000000-0000-4000-8000-000000000002/,
  );
  await page.screenshot({ path: resolve(root, "filtered-agent-phone.png") });
  await page.locator("#clear").click();
  assert.equal(await page.locator(".agent-card").count(), 20);
  await page.getByRole("link", { name: "Events", exact: true }).click();
  await page.locator("#range-trigger").click();
  await page.getByRole("button", { name: "Last hour", exact: true }).click();
  assert.equal(await page.locator(".event-row").count(), 3);
  await page.locator("#range-trigger").click();
  await page.locator("#from-date").fill("2026-02-30");
  await page.getByRole("button", { name: "Apply range" }).click();
  assert.equal(await page.locator("#range-error").isVisible(), true);
  assert.equal(await page.locator(".event-row").count(), 3);
  await page.locator("#from-date").fill("2026-11-04");
  await page.getByRole("button", { name: "Apply range" }).click();
  assert.equal(await page.locator("#range-error").isVisible(), true);
  await page.locator("#cancel-range").click();
  assert.equal(
    await page.locator("#range-trigger strong").innerText(),
    "Last hour",
  );
  await page.locator("#range-trigger").click();
  await page.locator("#from-time").fill("11:00");
  await page.getByRole("button", { name: "Apply range" }).click();
  assert.equal(await page.locator(".event-row").count(), 3);
  assert.match(
    await page.locator("#range-trigger strong").innerText(),
    /2026-11-03 11:00/,
  );
  await page.screenshot({ path: resolve(root, "custom-range-phone.png") });
  assert.equal(
    await page.evaluate(() => document.documentElement.scrollWidth),
    390,
  );
  await page.locator("#range-trigger").click();
  await page.keyboard.press("Escape");
  assert.equal(await page.locator("#range").isVisible(), false);
  assert.equal(
    await page
      .locator("#range-trigger")
      .evaluate((e) => e === document.activeElement),
    true,
  );
  await page.locator("#agent-trigger").click();
  await page.locator("#picker-search").fill("does-not-exist");
  assert.match(
    await page.locator("#picker-options").innerText(),
    /No matching choices/,
  );
  await page.keyboard.press("Escape");
  await page.locator("#clear").focus();
  await page.keyboard.press("Enter");
  assert.equal(await page.locator(".event-row").count(), 20);
  assert.equal(
    await page
      .locator("#agent-trigger")
      .evaluate((e) => e === document.activeElement),
    true,
  );
  await page.locator("#theme").click();
  assert.equal(await page.locator("html").getAttribute("data-theme"), "dark");
  await page.setViewportSize({ width: 320, height: 640 });
  await page.locator("#range-trigger").click();
  for (const name of ["From date", "From time", "To date", "To time"]) {
    assert.equal(
      await page.getByRole("textbox", { name, exact: true }).count(),
      1,
    );
  }
  await page.getByRole("button", { name: "Close time range" }).focus();
  await page.keyboard.press("Shift+Tab");
  assert.equal(
    await page
      .getByRole("button", { name: "Apply range" })
      .evaluate((e) => e === document.activeElement),
    true,
  );
  await page.locator("#to-time").fill("25:00");
  await page.getByRole("button", { name: "Apply range" }).click();
  assert.equal(await page.locator("#range-error").isVisible(), true);
  await page.screenshot({ path: resolve(root, "invalid-range-phone.png") });
  await page.close();
  await writeFile(
    resolve(root, "render-checks.json"),
    await format(
      JSON.stringify({
        scenarios: measurements,
        interactions:
          "project and agent search/selection, identity disclosure, clear, view navigation, last-hour preset, invalid dates/times and reversed range, cancel, custom range, Escape/focus return, empty search, theme switch",
      }),
      { parser: "json" },
    ),
  );
  console.log(
    "Rendered 11 proposal screenshots; scenario geometry and interaction checks passed.",
  );
} finally {
  await browser.close();
}
