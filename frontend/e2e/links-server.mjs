// Owned, disposable fixture using the acceptance tests' sequential port pair.
import { execFileSync, spawn } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { createServer } from "node:net";
import { createServer as httpsServer } from "node:https";
import { request } from "node:http";
import { fixturePort } from "./ports.mjs";
import { snapshots } from "./plans-fixture.mjs";
import { password } from "./fixture.mjs";
export async function startLinksServer() {
  for (const port of [fixturePort(38130), fixturePort(38131)])
    await new Promise((done, reject) => {
      const probe = createServer();
      probe.once("error", reject);
      probe.listen(port, "127.0.0.1", () => probe.close(done));
    });
  const dir = mkdtempSync(join(tmpdir(), "silkboard-links-")),
    repo = resolve("..");
  const metadata = JSON.parse(
    execFileSync(
      "cargo",
      ["metadata", "--locked", "--no-deps", "--format-version", "1"],
      { cwd: repo, encoding: "utf8" },
    ),
  );
  const binary = join(metadata.target_directory, "debug/silkboard-web"),
    env = {
      ...process.env,
      SILKBOARD_DATABASE: join(dir, "data.sqlite"),
      SILKBOARD_OWNER_ORIGIN: `https://127.0.0.1:${fixturePort(38130)}`,
      SILKBOARD_LISTEN: `127.0.0.1:${fixturePort(38131)}`,
      SILKBOARD_FRONTEND: join(repo, "frontend/dist"),
    };
  execFileSync(binary, ["owner", "set-password"], {
    env,
    input: password,
    stdio: ["pipe", "pipe", "pipe"],
  });
  execFileSync(
    binary,
    [
      "publisher",
      "issue",
      "silkboard",
      "links-browser",
      join(dir, "publisher.token"),
    ],
    { env, stdio: "pipe" },
  );
  const app = spawn(binary, [], { env, stdio: "ignore" });
  let proxy;
  const close = async () => {
    if (proxy) {
      proxy.closeAllConnections();
      await new Promise((done) => proxy.close(done));
    }
    if (app.exitCode === null && app.signalCode === null)
      await new Promise((done) => {
        app.once("exit", done);
        app.kill("SIGTERM");
      });
    rmSync(dir, { recursive: true, force: true });
  };
  try {
    let ready = false;
    for (let attempt = 0; attempt < 100; attempt++) {
      if (app.exitCode !== null || app.signalCode !== null)
        throw Error("Link backend exited");
      try {
        ready = (await fetch(`http://127.0.0.1:${fixturePort(38131)}/health`))
          .ok;
      } catch {
        /* Wait only for our child. */
      }
      if (ready) break;
      await new Promise((done) => setTimeout(done, 50));
    }
    if (!ready) throw Error("Link backend did not start");
    const token = readFileSync(join(dir, "publisher.token"), "utf8").trim();
    async function publish(path, value) {
      const result = await fetch(
        `http://127.0.0.1:${fixturePort(38131)}/api/projects/silkboard/${path}`,
        {
          method: "POST",
          headers: {
            Authorization: `Bearer ${token}`,
            "Content-Type": "application/json",
          },
          body: JSON.stringify(value),
        },
      );
      if (!result.ok) throw Error("Link fixture publication failed");
    }
    for (const snapshot of snapshots()) await publish("plans", snapshot);
    for (let sequence = 1; sequence <= 25; sequence++)
      await publish("progress", {
        event_id: `link-${sequence}`,
        hostname: "link-host",
        tool: "codex",
        session_id: "link-session",
        sequence,
        message: `Link report ${sequence}`,
      });
    const shared = process.env.SILKBOARD_E2E_DIR;
    proxy = httpsServer(
      {
        key: readFileSync(join(shared, "key.pem")),
        cert: readFileSync(join(shared, "cert.pem")),
      },
      (incoming, outgoing) => {
        const upstream = request(
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
          outgoing.writeHead(502);
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
    return close;
  } catch (error) {
    await close();
    throw error;
  }
}
