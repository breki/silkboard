import { execFileSync, spawn } from "node:child_process";
import {
  readFileSync,
  writeFileSync,
  rmSync,
  lstatSync,
  readdirSync,
  mkdirSync,
} from "node:fs";
import { createServer } from "node:net";
import { createServer as httpsServer } from "node:https";
import { request } from "node:http";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { password, hostile, markdownReport } from "./fixture.mjs";

import { seedPlans } from "./plans-fixture.mjs";

const dir = process.env.SILKBOARD_E2E_DIR;
if (
  !dir ||
  !dir.startsWith(join(tmpdir(), "silkboard-browser-")) ||
  resolve(dir) !== dir ||
  !lstatSync(dir).isDirectory() ||
  (lstatSync(dir).mode & 0o777) !== 0o700 ||
  readdirSync(dir).length !== 0
)
  throw new Error("Expected a private browser-test directory");
// Refuse an occupied backend port before starting or seeding any service.
for (const port of [38125, 38126, 38127, 38128, 38129])
  await new Promise((done, reject) => {
    const probe = createServer();
    probe.once("error", reject);
    probe.listen(port, "127.0.0.1", () => probe.close(done));
  });
const repo = resolve("..");
const env = {
  ...process.env,
  SILKBOARD_DATABASE: join(dir, "progress.sqlite"),
  SILKBOARD_OWNER_ORIGIN: "https://127.0.0.1:38124",
  SILKBOARD_LISTEN: "127.0.0.1:38125",
  SILKBOARD_FRONTEND: join(repo, "frontend/dist"),
};
// Use the same profile as the fresh normal test binary. Cargo still checks
// all source/build inputs; standalone browser runs build it when necessary.
execFileSync(
  "cargo",
  ["build", "--locked", "--profile", "test", "-p", "silkboard-web"],
  {
    cwd: repo,
    stdio: "inherit",
  },
);
const metadata = JSON.parse(
  execFileSync(
    "cargo",
    ["metadata", "--locked", "--no-deps", "--format-version", "1"],
    { cwd: repo },
  ),
);
const binary = join(metadata.target_directory, "debug/silkboard-web");
execFileSync(binary, ["owner", "set-password"], { env, input: password });
execFileSync(
  binary,
  ["publisher", "issue", "silkboard", "browser", join(dir, "publisher.token")],
  { env },
);
execFileSync(
  binary,
  ["publisher", "issue", "bombyx", "browser", join(dir, "other.token")],
  { env },
);
const setupDir = join(dir, "setup");
mkdirSync(setupDir, { mode: 0o700 });
const setupEnv = {
  ...env,
  SILKBOARD_DATABASE: join(setupDir, "progress.sqlite"),
  SILKBOARD_OWNER_ORIGIN: "https://127.0.0.1:38126",
  SILKBOARD_LISTEN: "127.0.0.1:38127",
};
writeFileSync(
  join(setupDir, "link"),
  execFileSync(binary, ["owner", "prepare-setup"], { env: setupEnv }),
  { mode: 0o600 },
);
execFileSync(
  binary,
  [
    "publisher",
    "issue",
    "silkboard",
    "setup-browser",
    join(setupDir, "publisher.token"),
  ],
  { env: setupEnv },
);
execFileSync(
  "openssl",
  [
    "req",
    "-x509",
    "-newkey",
    "rsa:2048",
    "-nodes",
    "-days",
    "1",
    "-subj",
    "/CN=localhost",
    "-addext",
    "subjectAltName=DNS:localhost,IP:127.0.0.1",
    "-keyout",
    join(dir, "key.pem"),
    "-out",
    join(dir, "cert.pem"),
  ],
  { stdio: "ignore" },
);
writeFileSync(
  join(dir, "state.json"),
  JSON.stringify({ binary, database: env.SILKBOARD_DATABASE }),
  { mode: 0o600 },
);
// Isolate plan browser sessions from feed/setup login and revocation tests.
const plansEnv = {
  ...env,
  SILKBOARD_DATABASE: join(dir, "plans.sqlite"),
  SILKBOARD_OWNER_ORIGIN: "https://127.0.0.1:38128",
  SILKBOARD_LISTEN: "127.0.0.1:38129",
};
execFileSync(binary, ["owner", "set-password"], {
  env: plansEnv,
  input: password,
});
execFileSync(
  binary,
  ["publisher", "issue", "silkboard", "plan-browser", join(dir, "plans.token")],
  { env: plansEnv },
);
const plansApp = spawn(binary, [], { env: plansEnv, stdio: "inherit" });
const app = spawn(binary, [], { env, stdio: "inherit" });
const setupApp = spawn(binary, [], { env: setupEnv, stdio: "inherit" });
const apps = [app, setupApp, plansApp],
  proxies = [];
let stopping = false;
function stop(code) {
  if (stopping) return;
  stopping = true;
  for (const proxy of proxies) {
    proxy.closeAllConnections();
    proxy.close();
  }
  Promise.all(
    apps.map(
      (child) =>
        new Promise((done) => {
          if (child.exitCode !== null || child.signalCode !== null)
            return done();
          child.once("exit", done);
          child.kill("SIGTERM");
        }),
    ),
  ).then(() => {
    rmSync(dir, { recursive: true, force: true });
    process.exit(code);
  });
}
process.on("SIGTERM", () => stop(0));
process.on("SIGINT", () => stop(0));
for (const child of apps)
  child.on("exit", () => {
    if (!stopping) stop(1);
  });
try {
  for (const backendPort of [38125, 38127, 38129]) {
    let ready = false;
    for (let attempt = 0; attempt < 100; attempt++) {
      try {
        ready = (await fetch(`http://127.0.0.1:${backendPort}/health`)).ok;
      } catch {
        /* Wait for this child only. */
      }
      if (ready) break;
      await new Promise((done) => setTimeout(done, 100));
    }
    if (!ready) throw new Error("Browser backend did not start");
  }
  const token = readFileSync(join(dir, "publisher.token"), "utf8").trim();
  for (const [tool, child, count] of [
    ["codex", false, 25],
    ["claude", false, 1],
    ["codex", true, 1],
  ]) {
    for (let sequence = 1; sequence <= count; sequence++) {
      const response = await fetch(
        "http://127.0.0.1:38125/api/projects/silkboard/progress",
        {
          method: "POST",
          headers: {
            Authorization: `Bearer ${token}`,
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            event_id: `${tool}-${child}-${sequence}`,
            hostname: "development-vm",
            tool,
            session_id: "n".repeat(256),
            sequence,
            ...(child
              ? { subagent_id: "reviewer", parent_session_id: "n".repeat(256) }
              : {}),
            ...(sequence === 1
              ? {
                  session_name: child
                    ? "Independent review"
                    : `${tool} implementation`,
                }
              : {}),
            message:
              sequence === 25
                ? hostile
                : sequence === 24
                  ? markdownReport
                  : `Checking step ${sequence}`,
          }),
        },
      );
      if (!response.ok)
        throw new Error(`Fixture publish failed: ${response.status}`);
    }
  }
  await seedPlans(readFileSync(join(dir, "plans.token"), "utf8").trim());
  const otherToken = readFileSync(join(dir, "other.token"), "utf8").trim();
  const other = await fetch(
    "http://127.0.0.1:38125/api/projects/bombyx/progress",
    {
      method: "POST",
      headers: {
        Authorization: `Bearer ${otherToken}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        event_id: "other-project",
        hostname: "other-vm",
        tool: "codex",
        session_id: "other-native",
        sequence: 1,
        message: "Work from another project",
      }),
    },
  );
  if (!other.ok) throw new Error("Other-project fixture publish failed");
  for (const [httpsPort, backendPort] of [
    [38128, 38129],
    [38126, 38127],
    [38124, 38125],
  ]) {
    const proxy = httpsServer(
      {
        key: readFileSync(join(dir, "key.pem")),
        cert: readFileSync(join(dir, "cert.pem")),
      },
      (incoming, outgoing) => {
        const upstream = request(
          {
            hostname: "127.0.0.1",
            port: backendPort,
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
          outgoing.writeHead(502);
          outgoing.end();
        });
        incoming.on("aborted", () => upstream.destroy());
        incoming.pipe(upstream);
      },
    );
    proxy.on("error", (error) => {
      console.error(error.message);
      stop(1);
    });
    proxies.push(proxy);
    proxy.listen(httpsPort, "127.0.0.1");
  }
} catch (error) {
  console.error(error.message);
  stop(1);
}
