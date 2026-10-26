import { resolve } from "node:path";
import { writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
const frontend = process.env.SILKBOARD_MOCKUP_FRONTEND;
const require = createRequire(
  frontend
    ? resolve(frontend, "package.json")
    : new URL("../../../frontend/package.json", import.meta.url),
);
const { chromium } = require("@playwright/test");
const root = fileURLToPath(new URL(".", import.meta.url));
const browser = await chromium.launch({ headless: true });
const measurements = [];
for (const [name, width, height, query] of [
  ["proposal-agents-desktop", 1360, 850, ""],
  ["proposal-agents-mobile", 320, 1040, ""],
  ["proposal-todos-mobile", 320, 1090, "?page=todos"],
  ["proposal-controls-light", 1280, 1000, "?page=controls"],
  ["proposal-controls-dark", 1280, 1000, "?page=controls&theme=dark"],
]) {
  const page = await browser.newPage({
    viewport: { width, height },
    deviceScaleFactor: 1,
  });
  await page.goto(new URL("proposal.html", import.meta.url).href + query);
  await page.screenshot({ path: root + "/" + name + ".png", fullPage: true });
  const check = await page.evaluate(() => ({
    width: innerWidth,
    scrollWidth: document.documentElement.scrollWidth,
    controls: [...document.querySelectorAll(".tap")].map((e) => ({
      text: e.textContent.trim(),
      height: e.getBoundingClientRect().height,
      width: e.getBoundingClientRect().width,
      faceHeight: e.querySelector(".face").getBoundingClientRect().height,
    })),
  }));
  if (check.scrollWidth > width) throw Error("Horizontal overflow " + name);
  if (
    width === 320 &&
    check.controls.some((c) => c.height < 44 || c.width < 44)
  )
    throw Error("Small mobile control " + name);
  measurements.push({ name, ...check });
  await page.close();
}
await writeFile(
  root + "/render-checks.json",
  JSON.stringify(measurements, null, 2) + "\n",
);
console.log(
  "Rendered five mockups; 320px layouts have no horizontal overflow and button targets are at least44×44px.",
);
await browser.close();
