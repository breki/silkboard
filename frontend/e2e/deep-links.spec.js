import { expect, test } from "@playwright/test";
import { fixturePort } from "./ports.mjs";
import { password } from "./fixture.mjs";
import { startLinksServer } from "./links-server.mjs";
import { task } from "./plans-fixture.mjs";
const baseURL = `https://127.0.0.1:${fixturePort(38130)}`;
test.use({ baseURL });
async function login(page, path) {
  await page.goto(path);
  await page.getByLabel("Owner password").fill(password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
}
let cookies, stopServer;
test.beforeAll(async ({ browser }) => {
  stopServer = await startLinksServer();
  const context = await browser.newContext({
    baseURL,
    ignoreHTTPSErrors: true,
  });
  const page = await context.newPage();
  await login(page, "/");
  await expect(page.getByRole("list", { name: "Main agents" })).toBeVisible();
  cookies = await context.cookies();
  await context.close();
});
test.afterAll(async () => {
  await stopServer?.();
});
test("filtered event URLs survive direct login, reload, a fresh browser session and history", async ({
  page,
  browser,
}) => {
  const path =
    "/events?project=silkboard&from=0&to=8640000000000000&view=hierarchy";
  await page.setViewportSize({ width: 390, height: 844 });
  await login(page, path);
  await expect(
    page.getByRole("button", { name: "Session hierarchy" }),
  ).toHaveAttribute("aria-pressed", "true");
  await expect(
    page.getByRole("button", { name: "Project: silkboard" }),
  ).toBeVisible();
  expect(new URL(page.url()).searchParams.get("from")).toBe("0");
  await page.getByRole("button", { name: "Flat list" }).click();
  await expect(page.getByRole("button", { name: "Flat list" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await page.goBack();
  await expect(
    page.getByRole("button", { name: "Session hierarchy" }),
  ).toHaveAttribute("aria-pressed", "true");
  await page.goForward();
  await expect(page.getByRole("button", { name: "Flat list" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await page.getByRole("button", { name: /^Received time:/ }).click();
  await page.getByRole("button", { name: "Last hour", exact: true }).click();
  const hour = new URL(page.url()).searchParams.get("from");
  await page.getByRole("button", { name: /^Received time:/ }).click();
  await page
    .getByRole("button", { name: "Last 24 hours", exact: true })
    .click();
  await page.goBack();
  expect(new URL(page.url()).searchParams.get("from")).toBe(hour);
  await expect(
    page.getByRole("button", {
      name: "Received time: Last 24 hours",
      exact: true,
    }),
  ).toHaveCount(0);
  await page.goForward();
  await expect(
    page.getByRole("button", {
      name: "Received time: Last 24 hours",
      exact: true,
    }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Earlier events" }).click();
  await expect(
    page.getByRole("button", { name: "Latest events" }),
  ).toBeVisible();
  const saved = page.url();
  await page.reload();
  await expect(
    page.getByRole("button", { name: "Latest events" }),
  ).toBeVisible();
  const other = await browser.newContext({ baseURL, ignoreHTTPSErrors: true });
  const second = await other.newPage();
  await second.setViewportSize({ width: 390, height: 844 });
  await login(second, saved);
  await expect(
    second.getByRole("button", { name: "Latest events" }),
  ).toBeVisible();
  await expect(
    second.getByRole("button", { name: "Project: silkboard" }),
  ).toBeVisible();
  await other.close();
});
test("agent identities, event details and missing targets restore from bookmarks", async ({
  page,
  context,
}) => {
  await context.addCookies(cookies);
  const agents = await (await context.request.get("/api/owner/agents")).json();
  const agent = agents.items[0];
  const identity = JSON.stringify([
    agent.project,
    agent.hostname,
    agent.tool,
    agent.session_id,
    null,
  ]);
  await page.goto(
    "/?" +
      new URLSearchParams({
        project: agent.project,
        hostname: agent.hostname,
        tool: agent.tool,
        session_id: agent.session_id,
        agent_details: identity,
      }),
  );
  await expect(page.locator("details[open]")).toHaveCount(1);
  await page.reload();
  await expect(page.locator("details[open]")).toHaveCount(1);
  const events = await (await context.request.get("/api/owner/events")).json();
  const event = events.items[0];
  await page.goto(
    "/events?" +
      new URLSearchParams({
        event: JSON.stringify([event.project, event.event_id]),
      }),
  );
  await expect(
    page.getByRole("region", { name: "Event details" }),
  ).toBeVisible();
  await page.reload();
  await expect(
    page.getByRole("region", { name: "Event details" }),
  ).toBeVisible();
  await page.goto(
    "/events?" +
      new URLSearchParams({
        event: JSON.stringify([event.project, "unavailable-bookmark"]),
      }),
  );
  await expect(
    page.getByText("The linked event is unavailable.", { exact: false }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Clear linked event" }).click();
  await expect(
    page.getByRole("button", { name: "Clear linked event" }),
  ).toHaveCount(0);
  await page.goto("/events?project=BAD");
  await expect(page.getByRole("alert")).toContainText("link");
  await page.getByRole("link", { name: "Reset view" }).click();
  await expect(
    page.getByRole("table", { name: "Events from all projects" }),
  ).toBeVisible();
  const missing = await page.goto("/unavailable-feature");
  expect(missing.status()).toBe(404);
  await expect(
    page.getByRole("heading", { name: "Page not found" }),
  ).toBeVisible();
});
test("workflow filters, selected steps, history and comparison survive reload and Back", async ({
  page,
  context,
}) => {
  await context.addCookies(cookies);
  await page.goto("/plans?project=silkboard&state=active");
  await expect(
    page.getByRole("button", { name: "Active", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  await page.reload();
  await expect(
    page.getByRole("button", { name: "Active", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  const path = `/plans/silkboard/${task}?sequence=3&step=ui&presentation=steps&history=revisions&compare=previous`;
  await page.goto(path);
  await expect(
    page.getByRole("list", { name: "Steps in prerequisite order" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Build graph page", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText(/Changes between received snapshots/),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Definition revisions", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  await page.reload();
  await expect(
    page.getByText(/Changes between received snapshots/),
  ).toBeVisible();
  const snapshot = page.getByRole("link", {
    name: "View that snapshot",
    exact: true,
  });
  const href = new URL(await snapshot.getAttribute("href"), page.url());
  expect(href.searchParams.get("step")).toBe("ui");
  expect(href.searchParams.get("presentation")).toBe("steps");
  const [nativeTab] = await Promise.all([
    context.waitForEvent("page"),
    snapshot.click({ modifiers: ["ControlOrMeta"] }),
  ]);
  await expect(
    nativeTab.getByRole("list", { name: "Steps in prerequisite order" }),
  ).toBeVisible();
  await nativeTab.close();
  await page.getByRole("button", { name: "Graph", exact: true }).click();
  await expect(
    page.getByRole("region", { name: "Task prerequisite graph" }),
  ).toBeVisible();
  await page.goBack();
  await expect(
    page.getByRole("list", { name: "Steps in prerequisite order" }),
  ).toBeVisible();
  await page.setViewportSize({ width: 320, height: 720 });
  await page.reload();
  const dialog = page.getByRole("dialog", { name: "Step details" });
  await expect(dialog).toBeVisible();
  await page.keyboard.press("Escape");
  await page.reload();
  await expect(dialog).not.toBeVisible();
});
test("todo and credential links restore selected views without saving drafts or issuing tokens", async ({
  page,
  context,
}) => {
  await context.addCookies(cookies);
  const csrf = (await (await context.request.get("/api/owner/session")).json())
    .csrf_token;
  const current = await (await context.request.get("/api/owner/todos")).json();
  const created = await context.request.post("/api/owner/todos", {
    headers: { origin: baseURL, "x-silkboard-csrf": csrf },
    data: { revision: current.revision, text: "Deep-link fixture todo" },
  });
  expect(created.status()).toBe(201);
  const todo = (await created.json()).items.find(
    (item) => item.text === "Deep-link fixture todo",
  );
  await page.goto("/todos?todo=" + todo.id + "&mode=edit");
  const editor = page.getByRole("textbox", { name: "Todo document" });
  await expect(editor).toBeFocused();
  await page.clock.install();
  await page
    .locator(".todo-task-text")
    .filter({ hasText: "Deep-link fixture todo" })
    .evaluate((element) => {
      const range = document.createRange();
      range.selectNodeContents(element);
      const selection = getSelection();
      selection.removeAllRanges();
      selection.addRange(range);
      document.dispatchEvent(new Event("selectionchange"));
    });
  await page.keyboard.type("Unsaved private draft");
  expect(page.url()).not.toContain("Unsaved");
  let mutations = 0;
  page.on("request", (request) => {
    if (request.method() !== "GET") mutations++;
  });
  page.once("dialog", (dialog) => dialog.accept());
  await page.reload();
  await expect(
    page.getByRole("checkbox", { name: "Deep-link fixture todo", exact: true }),
  ).toBeVisible();
  await expect(editor).toBeFocused();
  expect(mutations).toBe(0);
  await page.goto(
    "/status?panel=credentials&credential_project=silkboard&theme=dark",
  );
  await expect(
    page.getByRole("region", { name: "Publisher credentials" }),
  ).toBeVisible();
  await expect(page.getByLabel("Credential project")).toHaveValue("silkboard");
  await expect(page.getByLabel("New publisher token")).toHaveCount(0);
  expect(mutations).toBe(0);
  await page.reload();
  await expect(page.getByLabel("Credential project")).toHaveValue("silkboard");
  expect(await page.locator("html").getAttribute("data-theme")).toBe("dark");
  expect(mutations).toBe(0);
});
