import { expect, test } from "@playwright/test";

const paintedHeight = (control) =>
  control.evaluate((node) => {
    const face = getComputedStyle(node, "::before");
    return (
      node.getBoundingClientRect().height -
      parseFloat(face.top) -
      parseFloat(face.bottom)
    );
  });

async function faceContrast(control) {
  const ratio = await control.evaluate((node) => {
    const luminance = (color) => {
      const c = color
        .match(/[\d.]+/g)
        .slice(0, 3)
        .map(Number)
        .map((n) => {
          const v = n / 255;
          return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
        });
      return c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722;
    };
    const a = luminance(getComputedStyle(node).color);
    const b = luminance(getComputedStyle(node, "::before").backgroundColor);
    return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
  });
  expect(ratio).toBeGreaterThanOrEqual(4.5);
}

test("compact desktop actions keep generous phone targets and visible keyboard focus", async ({
  page,
}, info) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto("/");
  const signIn = page.getByRole("button", { name: "Sign in", exact: true });
  await expect(signIn).toBeVisible();
  expect((await signIn.boundingBox()).height).toBeLessThanOrEqual(33);
  expect((await signIn.boundingBox()).width).toBeLessThan(200);
  await signIn.focus();
  expect(
    await signIn.evaluate((node) => getComputedStyle(node).outlineStyle),
  ).toBe("solid");
  await page.screenshot({
    path: info.outputPath("controls-desktop.png"),
    fullPage: true,
  });
  await page.setViewportSize({ width: 320, height: 800 });
  for (const theme of ["light", "dark"]) {
    await page.emulateMedia({ colorScheme: theme });
    await faceContrast(signIn);
    const box = await signIn.boundingBox();
    expect(box.height).toBeGreaterThanOrEqual(44);
    expect(box.width).toBeGreaterThanOrEqual(44);
    expect(await paintedHeight(signIn)).toBeLessThanOrEqual(40);
    const menu = page.getByRole("button", { name: "Navigation menu" });
    await menu.click({ position: { x: 2, y: 2 } });
    const menuBox = await page
      .getByRole("navigation", { name: "Main navigation", exact: true })
      .boundingBox();
    const linksBox = await page
      .getByRole("navigation", { name: "Primary navigation", exact: true })
      .boundingBox();
    expect(menuBox.y).toBeGreaterThanOrEqual(linksBox.y + linksBox.height);
    const themeChoice = page.getByRole("combobox", {
      name: "Theme",
      exact: true,
    });
    expect((await themeChoice.boundingBox()).height).toBeGreaterThanOrEqual(44);
    await page.keyboard.press("Escape");
    await expect(menu).toBeFocused();
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    ).toBe(true);
    await page.screenshot({
      path: info.outputPath(`controls-phone-${theme}.png`),
      fullPage: true,
    });
  }
  await page.addStyleTag({ content: ":root { font-size: 200%; }" });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  for (const link of await page
    .getByRole("navigation", { name: "Primary navigation" })
    .getByRole("link")
    .all())
    await expect(link).toBeVisible();
  await page.emulateMedia({ forcedColors: "active" });
  await faceContrast(signIn);
  await signIn.focus();
  expect(
    await signIn.evaluate((node) => getComputedStyle(node).outlineStyle),
  ).toBe("solid");
});
