import { fixturePort } from "./ports.mjs";
import { expect, test } from "@playwright/test";
import { execFileSync, spawn } from "node:child_process";
import {
  mkdtempSync,
  mkdirSync,
  copyFileSync,
  chmodSync,
  readFileSync,
  writeFileSync,
  rmSync,
} from "node:fs";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { createServer as netServer } from "node:net";
import { createServer as httpsServer } from "node:https";
import { request as upstreamRequest } from "node:http";
import { password, markdownReport } from "./fixture.mjs";

// This fixture owns its processes and files; it adds no application test API.
const origin = `https://127.0.0.1:${fixturePort(38130)}`;
test.use({ baseURL: origin });
const task = "80808080-8080-4080-8080-808080808080";
const path = `/plans/silkboard/${task}`;
let root, binary, publisher, config, env, app, proxy;
async function stopApp() {
  const child = app;
  app = null;
  if (!child || child.exitCode !== null || child.signalCode !== null) return;
  await new Promise((done, reject) => {
    const timer = setTimeout(() => {
      child.kill("SIGKILL");
      reject(new Error("Owned backend did not stop"));
    }, 5000);
    child.once("exit", () => {
      clearTimeout(timer);
      done();
    });
    child.kill("SIGTERM");
  });
}
async function startApp() {
  app = spawn(binary, [], { env, stdio: ["ignore", "ignore", "pipe"] });
  // Retain only fixed diagnostics from our owned application.
  app.stderr.on("data", () => {});
  for (let i = 0; i < 100; i++) {
    if (app.exitCode !== null || app.signalCode !== null)
      throw new Error("Owned backend exited");
    try {
      if ((await fetch(`http://127.0.0.1:${fixturePort(38131)}/health`)).ok)
        return;
    } catch {
      /* Wait for this child only. */
    }
    await new Promise((done) => setTimeout(done, 50));
  }
  throw new Error("Owned backend did not start");
}
function publish(command, value) {
  const output = execFileSync(publisher, [...command, "--config", config], {
    cwd: join(root, "workspace"),
    env: {
      ...process.env,
      CODEX_THREAD_ID: "fixture-native-acceptance",
      PATH: "/nonexistent",
    },
    input: JSON.stringify(value),
    encoding: "utf8",
  });
  expect(output.trim()).toMatch(/acknowledged$/);
}
function draft() {
  return {
    schema_version: 1,
    task_id: task,
    title: "Task-plan feature acceptance fixture",
    description: "A fixture of this same feature, not production observations.",
    github_issue: { repository: "breki/silkboard", number: "80" },
    steps: [
      {
        id: "contract",
        title: "Agree reporting contract",
        state: "done",
        depends_on: [],
      },
      {
        id: "storage",
        title: "Store private snapshots",
        state: "done",
        depends_on: ["contract"],
      },
      {
        id: "publisher",
        title: "Publish from Codex",
        state: "active",
        depends_on: ["storage"],
      },
      {
        id: "graph",
        title: "Render owner graph",
        state: "active",
        depends_on: ["storage"],
        description: markdownReport,
      },
      {
        id: "acceptance",
        title: "Verify combined feature",
        state: "blocked",
        depends_on: ["publisher", "graph"],
        blocked_reason: "Awaiting the fixture review gate",
      },
      {
        id: "release",
        title: "Release and dogfood",
        state: "pending",
        depends_on: ["acceptance"],
      },
    ],
  };
}
test.beforeAll(async () => {
  for (const port of [fixturePort(38130), fixturePort(38131)])
    await new Promise((done, reject) => {
      const probe = netServer();
      probe.once("error", reject);
      probe.listen(port, "127.0.0.1", () => probe.close(done));
    });
  root = mkdtempSync(join(tmpdir(), "silkboard-acceptance-"));
  chmodSync(root, 0o700);
  for (const dir of ["state", "workspace", "bin"])
    mkdirSync(join(root, dir), { mode: 0o700 });
  const repo = resolve("..");
  const metadata = JSON.parse(
    execFileSync(
      "cargo",
      ["metadata", "--locked", "--no-deps", "--format-version", "1"],
      { cwd: repo, encoding: "utf8" },
    ),
  );
  binary = join(metadata.target_directory, "debug", "silkboard-web");
  publisher = join(root, "bin", "silkboard-publish");
  copyFileSync(
    join(metadata.target_directory, "debug", "silkboard-publish"),
    publisher,
  );
  chmodSync(publisher, 0o700);
  env = {
    ...process.env,
    SILKBOARD_DATABASE: join(root, "data.sqlite"),
    SILKBOARD_OWNER_ORIGIN: origin,
    SILKBOARD_LISTEN: `127.0.0.1:${fixturePort(38131)}`,
    SILKBOARD_FRONTEND: join(repo, "frontend", "dist"),
  };
  execFileSync(binary, ["owner", "set-password"], {
    env,
    input: password,
    stdio: ["pipe", "pipe", "pipe"],
  });
  const tokenFile = join(root, "publisher.token");
  execFileSync(
    binary,
    ["publisher", "issue", "silkboard", "acceptance", tokenFile],
    { env, stdio: "pipe" },
  );
  config = join(root, "config.json");
  writeFileSync(
    config,
    JSON.stringify({
      endpoint: `http://127.0.0.1:${fixturePort(38131)}`,
      project: "silkboard",
      token_file: tokenFile,
      state_dir: join(root, "state"),
      codex: {
        workspace: join(root, "workspace"),
        hostname: "fixture-dev",
        plan_reporting: true,
        session_name: "Combined acceptance fixture",
      },
    }),
    { mode: 0o600 },
  );
  await startApp();
  publish(["publish"], {
    hostname: "fixture-legacy",
    tool: "codex",
    session_id: "fixture-existing-session",
    message: "Existing data before task-plan publishing",
  });
  const shared = process.env.SILKBOARD_E2E_DIR;
  proxy = httpsServer(
    {
      key: readFileSync(join(shared, "key.pem")),
      cert: readFileSync(join(shared, "cert.pem")),
    },
    (incoming, outgoing) => {
      const upstream = upstreamRequest(
        {
          hostname: "127.0.0.1",
          port: fixturePort(38131),
          path: incoming.url,
          method: incoming.method,
          headers: incoming.headers,
        },
        (response) => {
          outgoing.writeHead(response.statusCode, response.headers);
          response.pipe(outgoing);
        },
      );
      upstream.on("error", () => {
        if (!outgoing.headersSent) outgoing.writeHead(502);
        outgoing.end();
      });
      incoming.on("aborted", () => upstream.destroy());
      incoming.pipe(upstream);
    },
  );
  await new Promise((done, reject) => {
    proxy.once("error", reject);
    proxy.listen(fixturePort(38130), "127.0.0.1", done);
  });
});
test.afterAll(async () => {
  try {
    await stopApp();
  } finally {
    if (proxy) {
      proxy.closeAllConnections();
      await new Promise((done) => proxy.close(done));
    }
    if (root) rmSync(root, { recursive: true, force: true });
  }
});
test("installed Codex adapter, owner graph and existing data survive a real backend restart", async ({
  page,
  request,
}, info) => {
  const value = draft();
  publish(["codex-plan", "start"], value);
  expect(
    (await request.get(`/api/owner/projects/silkboard/plans/${task}`)).status(),
  ).toBe(401);
  const token = readFileSync(join(root, "publisher.token"), "utf8").trim();
  expect(
    (
      await request.get("/api/owner/plans", {
        headers: { Authorization: `Bearer ${token}` },
      })
    ).status(),
  ).toBe(401);
  expect(
    (
      await request.post("/api/projects/other/plans", {
        headers: { Authorization: `Bearer ${token}` },
        data: value,
      })
    ).status(),
  ).toBe(401);
  expect(
    (
      await request.post("/api/projects/silkboard/plans", {
        headers: {
          Authorization: `Bearer ${token}`,
          "Content-Type": "application/json",
        },
        data: " ".repeat(262145),
      })
    ).status(),
  ).toBe(413);
  await page.clock.install();
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto(path);
  await page.getByLabel("Owner password").fill(password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Publish from Codex, Active" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Release and dogfood, Pending" }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Render owner graph, Active" })
    .click();
  await expect(
    page
      .getByRole("complementary")
      .getByRole("heading", { name: "Markdown report" }),
  ).toBeVisible();
  await expect(
    page.locator(".event-message img,.event-message script"),
  ).toHaveCount(0);
  await page
    .getByRole("button", { name: "Verify combined feature, Blocked" })
    .click();
  await expect(
    page.getByText("Awaiting the fixture review gate"),
  ).toBeVisible();
  value.steps.push({
    id: "recovery",
    title: "Verify recovery",
    state: "active",
    depends_on: ["storage"],
  });
  value.steps[4].depends_on.push("recovery");
  publish(["codex-plan", "publish"], value);
  await page.clock.fastForward(10000);
  await expect(
    page.getByRole("button", { name: "Verify recovery, Active" }),
  ).toBeVisible();
  for (const index of [2, 3, 6]) value.steps[index].state = "done";
  value.steps[4].state = "active";
  delete value.steps[4].blocked_reason;
  publish(["codex-plan", "publish"], value);
  await page.clock.fastForward(10000);
  await expect(
    page.getByRole("button", { name: "Verify combined feature, Active" }),
  ).toHaveAttribute("aria-pressed", "true");
  const before = await page
    .context()
    .request.get(`/api/owner/projects/silkboard/plans/${task}`);
  const received = await before.json();
  expect(received.summary).toMatchObject({
    sequence: "3",
    plan_revision: "2",
    counts: { total: 7, done: 5, active: 1, blocked: 0 },
  });
  await stopApp();
  await startApp();
  await page.reload();
  await expect(
    page.getByRole("button", { name: "Verify combined feature, Active" }),
  ).toBeVisible();
  const after = await page
    .context()
    .request.get(`/api/owner/projects/silkboard/plans/${task}`);
  expect(await after.json()).toEqual(received);
  const events = await (
    await page.context().request.get("/api/owner/events")
  ).json();
  expect(events.items[0].message).toBe(
    "Existing data before task-plan publishing",
  );
  await page
    .getByRole("button", { name: "Received history and revisions" })
    .click();
  const history = page.getByRole("list", { name: "Received workflow reports" });
  await expect(history.locator(":scope > li")).toHaveCount(3);
  await history.getByRole("link").last().click();
  expect(new URL(page.url()).pathname).toBe(path);
  expect(new URL(page.url()).searchParams.get("sequence")).toBe("1");
  await expect(page.getByText(/Earlier snapshot/).first()).toBeVisible();
  await page
    .getByRole("button", { name: "Verify combined feature, Blocked" })
    .click();
  await expect(
    page.getByText("Awaiting the fixture review gate"),
  ).toBeVisible();
  await page.getByRole("link", { name: "Return to Current workflow" }).click();
  await expect(
    page.getByRole("button", { name: "Verify combined feature, Active" }),
  ).toBeVisible();
  await page.setViewportSize({ width: 320, height: 720 });
  // Wait for the application breakpoint handler before sending keyboard input.
  await expect(
    page.getByRole("complementary", { name: "Step details" }),
  ).toHaveCount(0);
  // The URL-selected details move from the desktop aside to the phone dialog.
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.keyboard.press("Escape");
  const active = page.getByRole("button", {
    name: "Verify combined feature, Active",
  });
  await active.focus();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(active).toBeFocused();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: info.outputPath("combined-acceptance-phone.png"),
    fullPage: true,
  });
  value.steps[4].state = "done";
  value.steps[5].state = "blocked";
  value.steps[5].blocked_reason =
    "Production release is a separate owner decision";
  publish(["codex-plan", "publish"], value);
  await page.clock.fastForward(10000);
  await expect(
    page.getByRole("button", { name: "Release and dogfood, Blocked" }),
  ).toBeVisible();
  expect(
    (
      await (
        await page
          .context()
          .request.get(`/api/owner/projects/silkboard/plans/${task}`)
      ).json()
    ).summary.sequence,
  ).toBe("4");
  await checkAutomaticWorkflowLink(page);
});

async function checkAutomaticWorkflowLink(page) {
  const options = {
    cwd: join(root, "workspace"),
    env: { ...process.env, PATH: "/nonexistent" },
    input: JSON.stringify({
      hook_event_name: "Stop",
      session_id: "fixture-native-acceptance",
      turn_id: "turn-acceptance",
      cwd: join(root, "workspace"),
      last_assistant_message:
        "Automatic workflow summary. Additional private fixture detail.",
    }),
    encoding: "utf8",
  };
  expect(
    execFileSync(
      publisher,
      ["codex-hook-context", "--config", config],
      options,
    ).trim(),
  ).toBe("{}");
  const output = execFileSync(
    publisher,
    ["codex-hook", "--config", config],
    options,
  );
  expect(output.trim()).toBe("{}");
  const response = await page.context().request.get("/api/owner/events");
  const reports = await response.json();
  expect(
    reports.items.find(
      (event) => event.message === "Automatic workflow summary.",
    )?.workflow_task_id,
  ).toBe(task);
  await page
    .getByRole("navigation", { name: "Primary navigation" })
    .getByRole("link", { name: "Events", exact: true })
    .click();
  const event = page
    .locator("tr.event-row")
    .filter({ hasText: "Automatic workflow summary." });
  const link = event.getByRole("link", { name: "Open workflow", exact: true });
  await expect(link).toHaveAttribute("href", path);
  await link.focus();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("button", { name: "Release and dogfood, Blocked" }),
  ).toBeVisible();
}
