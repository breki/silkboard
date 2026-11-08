import { expect, test } from "@playwright/test";
import { fixturePort } from "./ports.mjs";
import { password } from "./fixture.mjs";
const origin = `https://127.0.0.1:${fixturePort(38128)}`;
test.use({ baseURL: origin });
let cookies;
test.beforeAll(async ({ browser }) => {
  const context = await browser.newContext({
    baseURL: origin,
    ignoreHTTPSErrors: true,
  });
  expect(
    (
      await context.request.post("/api/auth/login", {
        headers: { origin },
        data: { password },
      })
    ).status(),
  ).toBe(204);
  cookies = await context.cookies();
  await context.close();
});
async function open(page, context, rows = []) {
  await context.addCookies(cookies);
  const csrf = (await (await context.request.get("/api/owner/session")).json())
    .csrf_token;
  const current = await (await context.request.get("/api/owner/todos")).json();
  const stack = [];
  const items = rows.map((row, n) => {
    stack.length = (row.depth ?? 1) - 1;
    const key = `seed-${n}`,
      parent_key = stack.at(-1) ?? null;
    stack.push(key);
    return {
      key,
      id: null,
      parent_key,
      text: row.text,
      completed: row.completed ?? false,
    };
  });
  const saved = await context.request.put("/api/owner/todos/document", {
    headers: { origin, "x-silkboard-csrf": csrf },
    data: { revision: current.revision, items },
  });
  expect(saved.status()).toBe(200);
  await page.goto("/todos");
  await expect(
    page.getByRole("textbox", { name: "Todo document" }),
  ).toBeVisible();
  return { csrf, data: await saved.json() };
}
const box = (page, text) =>
  page.getByRole("checkbox", { name: text, exact: true });
const text = (page, value) =>
  page
    .locator(".todo-task")
    .filter({ has: box(page, value) })
    .locator(".todo-task-text");
async function select(page, value, all = false) {
  await page.getByRole("textbox", { name: "Todo document" }).focus();
  await text(page, value).evaluate((element, all) => {
    const range = document.createRange();
    range.selectNodeContents(element);
    if (!all)
      range.setStart(
        element.firstChild ?? element,
        element.firstChild?.textContent.length ?? 0,
      );
    if (!all) range.collapse(true);
    const selection = getSelection();
    selection.removeAllRanges();
    selection.addRange(range);
    document.dispatchEvent(new Event("selectionchange"));
  }, all);
}
async function saved(page, action) {
  const response = page.waitForResponse(
    (r) =>
      r.url().endsWith("/api/owner/todos/document") &&
      r.request().method() === "PUT",
  );
  await action();
  expect((await response).status()).toBe(200);
  await expect(
    page.getByText("All changes saved", { exact: true }),
  ).toBeVisible();
}
async function depth(page, value) {
  return Number(
    await text(page, value).evaluate(
      (node) => node.closest(".todo-task").dataset.depth,
    ),
  );
}

const activeItem = (page) =>
  page.evaluate(() => {
    const node = getSelection()?.focusNode;
    return (node?.nodeType === Node.ELEMENT_NODE ? node : node?.parentElement)
      ?.closest(".todo-task")
      ?.querySelector("input")
      ?.getAttribute("aria-label");
  });
test("compact rows and keyboard commands follow document order across nesting levels", async ({
  page,
  context,
}) => {
  await open(page, context, [
    { text: "Parent" },
    { text: "First child", depth: 2 },
    { text: "Nested child", depth: 3 },
    { text: "Second child", depth: 2 },
    { text: "Next root" },
  ]);
  expect(
    (await text(page, "Parent").locator("..").boundingBox()).height,
  ).toBeLessThan(35);
  await select(page, "First child");
  for (const item of ["Nested child", "Second child", "Next root"]) {
    await page.keyboard.press("ArrowDown");
    await expect.poll(() => activeItem(page)).toBe(item);
  }
  for (const item of ["Second child", "Nested child", "First child"]) {
    await page.keyboard.press("ArrowUp");
    await expect.poll(() => activeItem(page)).toBe(item);
  }
  await saved(page, () => page.keyboard.press("ControlOrMeta+Enter"));
  await expect(box(page, "First child")).toBeChecked();
  await expect(box(page, "Parent")).not.toBeChecked();
  await select(page, "Parent");
  await saved(page, () => page.keyboard.press("ControlOrMeta+Shift+Backspace"));
  await expect(box(page, "Parent")).toHaveCount(0);
  expect(await depth(page, "First child")).toBe(1);
  expect(await depth(page, "Nested child")).toBe(2);
  await saved(page, () => page.keyboard.press("ControlOrMeta+z"));
  expect(await depth(page, "Nested child")).toBe(3);
  await page.screenshot({
    path: "/tmp/silkboard152-todos-desktop.png",
    fullPage: true,
  });
  await page.setViewportSize({ width: 390, height: 844 });
  const row = await text(page, "Parent").locator("..").boundingBox();
  expect(row.height).toBeGreaterThanOrEqual(44);
  expect(row.height).toBeLessThan(48);
  await page.screenshot({
    path: "/tmp/silkboard152-todos-phone.png",
    fullPage: true,
  });
});

test("project autocomplete and filtered edits preserve the entire persisted checklist", async ({
  page,
  context,
}) => {
  await open(page, context, [
    { text: "Parent" },
    { text: "Match @silkboard", depth: 2 },
    { text: "Other child", depth: 2, completed: true },
    { text: "Hidden" },
  ]);
  const filter = page.getByRole("combobox", {
    name: "Filter todos by project",
  });
  await expect(filter).toBeEnabled();
  await filter.selectOption("silkboard");
  await expect(page).toHaveURL(/project=silkboard/);
  await expect(box(page, "Parent")).toBeVisible();
  await expect(box(page, "Parent")).toBeDisabled();
  await expect(box(page, "Other child")).toBeHidden();
  await expect(box(page, "Hidden")).toBeHidden();
  await expect(
    page.getByRole("button", { name: "New task", exact: true }),
  ).toBeDisabled();
  await select(page, "Match @silkboard");
  await page.keyboard.press("Escape"); // Dismiss project suggestions, if open.
  await select(page, "Match @silkboard");
  await saved(page, () => page.keyboard.press("ControlOrMeta+Enter"));
  await expect(box(page, "Match @silkboard")).toBeChecked();
  await page.keyboard.press("Enter");
  await page.keyboard.press("ControlOrMeta+Shift+Backspace");
  await expect(box(page, "Match @silkboard")).toBeVisible();
  await page.getByRole("button", { name: "Show full list" }).click();
  await expect(
    page.getByText(
      "Show the full list to add, delete, move items or edit parent context.",
    ),
  ).toHaveCount(0);
  await expect(box(page, "Other child")).toBeChecked();
  await expect(box(page, "Hidden")).toBeVisible();
  await page.setViewportSize({ width: 390, height: 844 });
  await select(page, "Hidden");
  await page.keyboard.type(" @si");
  await expect(
    page.getByRole("listbox", { name: "Project suggestions" }),
  ).toBeVisible();
  const menu = await page
    .getByRole("listbox", { name: "Project suggestions" })
    .boundingBox();
  expect(menu.x).toBeGreaterThanOrEqual(0);
  expect(menu.x + menu.width).toBeLessThanOrEqual(390);
  expect(menu.y + menu.height).toBeLessThanOrEqual(844);
  await page.screenshot({
    path: "/tmp/silkboard152-project-menu-phone.png",
    fullPage: true,
  });
  await saved(page, () => page.keyboard.press("Enter"));
  await expect(box(page, "Hidden @silkboard")).toBeVisible();
  await page.reload();
  await expect(box(page, "Hidden @silkboard")).toBeVisible();
  await expect(box(page, "Match @silkboard")).toBeChecked();
  expect(await depth(page, "Other child")).toBe(2);
  await filter.selectOption("silkboard");
  await saved(page, async () => {
    await select(page, "Match @silkboard", true);
    await page.keyboard.type("Untagged while editing");
  });
  await expect(box(page, "Untagged while editing")).toBeVisible();
  await select(page, "Hidden @silkboard");
  await expect(box(page, "Untagged while editing")).toBeHidden();
  await page.getByRole("button", { name: "Show full list" }).click();
  await expect(box(page, "Untagged while editing")).toBeChecked();
});

test("document keyboard edits preserve subtrees, checkboxes, undo and saved order", async ({
  page,
  context,
}, info) => {
  await open(page, context);
  await saved(page, async () => {
    await page.getByRole("textbox", { name: "Todo document" }).click();
    await page.keyboard.type("Prepare release");
  });
  await saved(page, async () => {
    await page.keyboard.press("Enter");
    await page.keyboard.type("Check package");
    await page.keyboard.press("Tab");
  });
  await saved(page, async () => {
    await page.keyboard.press("Enter");
    await page.keyboard.type("Nested check");
    await page.keyboard.press("Tab");
  });
  expect(await depth(page, "Check package")).toBe(2);
  expect(await depth(page, "Nested check")).toBe(3);
  await page.screenshot({
    path: info.outputPath("todos-editor-desktop.png"),
    fullPage: true,
  });
  await saved(page, async () => {
    await box(page, "Prepare release").click();
  });
  await expect(box(page, "Prepare release")).toBeChecked();
  await expect(box(page, "Check package")).not.toBeChecked();
  await expect(box(page, "Prepare release")).toBeFocused();
  await saved(page, async () => {
    await select(page, "Prepare release", true);
    await page.keyboard.type("Ship release");
  });
  await expect(box(page, "Ship release")).toBeChecked();
  await saved(page, async () => {
    await select(page, "Ship release", true);
    await page.keyboard.press("Backspace");
    await page.keyboard.press("Backspace");
  });
  await expect(box(page, "Ship release")).toHaveCount(0);
  expect(await depth(page, "Check package")).toBe(1);
  expect(await depth(page, "Nested check")).toBe(2);
  await saved(page, async () => {
    await page.keyboard.press("ControlOrMeta+z");
    await page.keyboard.press("ControlOrMeta+z");
  });
  await expect(box(page, "Ship release")).toBeChecked();
  expect(await depth(page, "Check package")).toBe(2);
  await page.reload();
  await expect(box(page, "Ship release")).toBeChecked();
  expect(await depth(page, "Nested check")).toBe(3);
  await select(page, "Nested check");
  await page.keyboard.press("Escape");
  await expect(
    page.getByRole("textbox", { name: "Todo document" }),
  ).not.toBeFocused();
  await page.keyboard.press("Tab");
  await expect(
    page.getByRole("textbox", { name: "Todo document" }),
  ).not.toBeFocused();
  await expect(
    page.getByRole("button", { name: "Edit Ship release" }),
  ).toHaveCount(0);
});

test("arrows follow wrapped text and cross-task selections retain unselected children", async ({
  page,
  context,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  const long =
    "Review the keyboard editor carefully across a task that wraps over several visual lines without losing the caret position";
  await open(page, context, [
    { text: long },
    { text: "Next task" },
    { text: "Retained child", depth: 2, completed: true },
  ]);
  await select(page, long);
  const top = () =>
    page.evaluate(() => {
      const s = getSelection(),
        r = s.getRangeAt(0).cloneRange();
      r.collapse(true);
      return r.getBoundingClientRect().top;
    });
  const previous = await top();
  await page.keyboard.press("ArrowUp");
  await expect.poll(top).toBeLessThan(previous);
  await page.keyboard.press("Shift+ArrowDown");
  expect(
    await page.evaluate(() => getSelection().toString().length),
  ).toBeGreaterThan(0);
  await select(page, long);
  await page.keyboard.press("ArrowDown");
  await expect
    .poll(() =>
      page.evaluate(() =>
        getSelection()
          .anchorNode.parentElement.closest(".todo-task")
          .querySelector("input")
          .getAttribute("aria-label"),
      ),
    )
    .toBe("Next task");
  await select(page, long);
  await page.keyboard.press("Shift+ArrowDown");
  expect(await page.evaluate(() => getSelection().toString())).toContain("\n");
  await saved(page, () => page.keyboard.type("replacement"));
  await expect(box(page, "Retained child")).toBeChecked();
  expect(await depth(page, "Retained child")).toBe(1);
  await expect(page.locator(".todo-task")).toHaveCount(2);
  await open(page, context, [
    { text: "Alpha" },
    { text: "Beta" },
    { text: "IME child", depth: 2, completed: true },
  ]);
  await select(page, "Alpha");
  await page.evaluate(() => {
    const texts = document.querySelectorAll(".todo-task-text");
    const range = document.createRange();
    range.setStart(texts[0].firstChild, 5);
    range.setEnd(texts[1].firstChild, 2);
    const selection = getSelection();
    selection.removeAllRanges();
    selection.addRange(range);
  });
  const input = await context.newCDPSession(page);
  await input.send("Input.imeSetComposition", {
    text: "日",
    selectionStart: 1,
    selectionEnd: 1,
  });
  await saved(page, () => input.send("Input.insertText", { text: "日本" }));
  await expect(box(page, "Alpha日本ta")).toBeVisible();
  await expect(box(page, "IME child")).toBeChecked();
  expect(await depth(page, "IME child")).toBe(1);
  await input.detach();
});

test("typing stays active during a delayed save and acknowledgement preserves caret", async ({
  page,
  context,
}) => {
  await open(page, context, [{ text: "Original" }]);
  let held;
  let first = true;
  const bodies = [];
  await page.route("**/api/owner/todos/document", (route) => {
    bodies.push(route.request().postDataJSON());
    if (first) {
      first = false;
      held = route;
    } else void route.continue();
  });
  await select(page, "Original");
  await page.keyboard.type(" first");
  await expect.poll(() => Boolean(held)).toBe(true);
  await page.keyboard.type(" second");
  await expect(box(page, "Original first second")).toBeVisible();
  const editor = await page
    .getByRole("textbox", { name: "Todo document" })
    .elementHandle();
  await held.continue();
  await expect(
    page.getByText("All changes saved", { exact: true }),
  ).toBeVisible();
  expect(bodies.at(-1).items[0].text).toBe("Original first second");
  expect(await editor.evaluate((node) => node.isConnected)).toBe(true);
  expect(
    await page.evaluate(() => {
      const s = getSelection(),
        text = s.anchorNode.parentElement.closest(".todo-task-text"),
        r = document.createRange();
      r.selectNodeContents(text);
      r.setEnd(s.anchorNode, s.anchorOffset);
      return {
        text: text.textContent,
        offset: r.toString().length,
        collapsed: s.isCollapsed,
      };
    }),
  ).toEqual({ text: "Original first second", offset: 21, collapsed: true });
  await page.unroute("**/api/owner/todos/document");
});

test("phone tools, real stale-tab conflicts and expiry preserve a recoverable draft", async ({
  page,
  context,
}, info) => {
  await page.setViewportSize({ width: 320, height: 720 });
  const { csrf } = await open(page, context, [
    { text: "Parent" },
    { text: "Child", depth: 2 },
  ]);
  let held;
  await page.route("**/api/owner/todos/document", (route) => {
    held = route;
  });
  await select(page, "Child");
  await page.keyboard.type(" local");
  await expect.poll(() => Boolean(held)).toBe(true);
  const current = await (await context.request.get("/api/owner/todos")).json();
  expect(
    (
      await context.request.put("/api/owner/todos/document", {
        headers: { origin, "x-silkboard-csrf": csrf },
        data: {
          revision: current.revision,
          items: current.items.map((row, n) => ({
            key: `remote-${n}`,
            id: row.id,
            parent_key: n ? "remote-0" : null,
            text: n ? "Remote child" : row.text,
            completed: row.completed,
          })),
        },
      })
    ).status(),
  ).toBe(200);
  await held.continue();
  await expect(page.getByRole("alert")).toContainText("changed in another tab");
  await page.unroute("**/api/owner/todos/document");
  await expect(box(page, "Child local")).toBeVisible();
  await page
    .getByRole("alert")
    .getByRole("button", { name: "Review saved list", exact: true })
    .click();
  await expect(
    page.getByRole("textbox", { name: "Your draft", exact: true }),
  ).toHaveValue(/Child local/);
  await expect(
    page.getByRole("textbox", { name: "Latest saved list", exact: true }),
  ).toHaveValue(/Remote child/);
  page.once("dialog", (dialog) => dialog.dismiss());
  await page
    .getByRole("button", { name: "Use saved list", exact: true })
    .click();
  await expect(box(page, "Child local")).toBeVisible();
  page.once("dialog", (dialog) => dialog.accept());
  await page
    .getByRole("button", { name: "Use saved list", exact: true })
    .click();
  await expect(box(page, "Remote child")).toBeVisible();
  await select(page, "Remote child");
  await saved(page, async () => {
    await page.getByRole("button", { name: "Outdent", exact: true }).click();
  });
  expect(await depth(page, "Remote child")).toBe(1);
  await saved(page, async () => {
    await page.getByRole("button", { name: "Indent", exact: true }).click();
  });
  expect(await depth(page, "Remote child")).toBe(2);
  const target = await page.locator(".todo-check-target").first().boundingBox();
  expect(target.width).toBeGreaterThanOrEqual(44);
  expect(target.height).toBeGreaterThanOrEqual(44);
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth),
  ).toBeLessThanOrEqual(320);
  await page.screenshot({
    path: info.outputPath("todos-editor-phone.png"),
    fullPage: true,
  });
  await page.emulateMedia({ colorScheme: "dark" });
  await page.screenshot({
    path: info.outputPath("todos-editor-phone-dark.png"),
    fullPage: true,
  });
  await page.route("**/api/owner/session", (route) =>
    route.fulfill({ status: 401, contentType: "application/json", body: "{}" }),
  );
  await select(page, "Remote child");
  await page.keyboard.type(" unsaved");
  await expect(page.getByLabel("Owner password")).toBeVisible();
  await expect(
    page.getByText(
      "Your unsaved Todos draft is kept in this tab. Sign in to recover it.",
    ),
  ).toBeVisible();
  await expect(box(page, "Remote child unsaved")).toHaveCount(0);
  await page.unroute("**/api/owner/session");
  await page.getByLabel("Owner password").fill(password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(box(page, "Remote child unsaved")).toBeVisible();
  await page.getByRole("button", { name: "Retry save", exact: true }).click();
  await expect(
    page.getByText("All changes saved", { exact: true }),
  ).toBeVisible();
});
