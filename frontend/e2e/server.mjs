import { execFileSync, spawn } from "node:child_process";
import {
  readFileSync,
  writeFileSync,
  rmSync,
  lstatSync,
  readdirSync,
} from "node:fs";
import { createServer } from "node:net";
import { createServer as httpsServer } from "node:https";
import { request } from "node:http";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { password, hostile } from "./fixture.mjs";

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
await new Promise((done, reject) => {
  const probe = createServer();
  probe.once("error", reject);
  probe.listen(38125, "127.0.0.1", () => probe.close(done));
});
const repo = resolve("..");
const env = {
  ...process.env,
  SILKBOARD_DATABASE: join(dir, "progress.sqlite"),
  SILKBOARD_OWNER_ORIGIN: "https://127.0.0.1:38124",
  SILKBOARD_LISTEN: "127.0.0.1:38125",
  SILKBOARD_FRONTEND: join(repo, "frontend/dist"),
};
execFileSync("cargo", ["build", "--locked", "-p", "silkboard-web"], {
  cwd: repo,
  stdio: "inherit",
});
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
const app = spawn(binary, [], { env, stdio: "inherit" });
let proxy;
let stopping = false;
function stop(code) {
  if (stopping) return;
  stopping = true;
  proxy?.closeAllConnections();
  proxy?.close();
  if (app.exitCode === null && app.signalCode === null) {
    app.once("exit", () => {
      rmSync(dir, { recursive: true, force: true });
      process.exit(code);
    });
    app.kill("SIGTERM");
  } else {
    rmSync(dir, { recursive: true, force: true });
    process.exit(code);
  }
}
process.on("SIGTERM", () => stop(0));
process.on("SIGINT", () => stop(0));
app.on("exit", () => {
  if (!stopping) stop(1);
});
try {
  let ready = false;
  for (let attempt = 0; attempt < 100; attempt++) {
    try {
      ready = (await fetch("http://127.0.0.1:38125/health")).ok;
    } catch {
      /* Wait for this child only. */
    }
    if (ready) break;
    await new Promise((done) => setTimeout(done, 100));
  }
  if (!ready) throw new Error("Browser backend did not start");
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
            message: sequence === 25 ? hostile : `Checking step ${sequence}`,
          }),
        },
      );
      if (!response.ok)
        throw new Error(`Fixture publish failed: ${response.status}`);
    }
  }
  proxy = httpsServer(
    {
      key: readFileSync(join(dir, "key.pem")),
      cert: readFileSync(join(dir, "cert.pem")),
    },
    (incoming, outgoing) => {
      const upstream = request(
        {
          hostname: "127.0.0.1",
          port: 38125,
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
  proxy.listen(38124, "127.0.0.1");
} catch (error) {
  console.error(error.message);
  stop(1);
}
