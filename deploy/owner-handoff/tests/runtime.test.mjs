import assert from "node:assert/strict";
import { test } from "node:test";
import { EventEmitter, once } from "node:events";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import http from "node:http";
import { validateConfig } from "../config.mjs";
import { createHandoff } from "../server.mjs";
import {
  capture,
  preparationArgs,
  checkDatabase,
  readSetupState,
  attachProxy,
  startHandoff,
} from "../runtime.mjs";

function proxyFixture() {
  const proxy = new EventEmitter();
  proxy.signals = [];
  proxy.kill = (signal) => {
    proxy.signals.push(signal);
    queueMicrotask(() => proxy.emit("exit", 0, signal));
  };
  return proxy;
}
function directory(t) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "handoff-runtime-"));
  fs.chmodSync(dir, 0o700);
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  return dir;
}
function application(t, body, status = 200) {
  const server = http.createServer((_req, res) => {
    res.writeHead(status, { "content-type": "application/json" });
    res.end(body);
  });
  t.after(() => {
    server.close();
    server.closeAllConnections();
  });
  server.listen(0, "127.0.0.1");
  return server;
}
test("subprocess output is bounded and errors remain generic", async () => {
  assert.equal(
    await capture(process.execPath, ["-e", 'process.stdout.write("ready")']),
    "ready",
  );
  await assert.rejects(
    capture(process.execPath, [
      "-e",
      'process.stderr.write("private");process.exit(1)',
    ]),
    /^Error: Handoff preflight failed\.$/,
  );
});
test("configured application command drops identity and uses separate private environment", () => {
  const c = validateConfig({
    origin: "https://beta.example.ts.net",
    ownerLogin: "bob@example.test",
    applicationUser: "beta",
    database: "/srv/beta/db",
    applicationBinary: "/srv/beta/web",
  });
  assert.deepEqual(preparationArgs(c), [
    "-u",
    "beta",
    "--",
    "/usr/bin/env",
    "SILKBOARD_DATABASE=/srv/beta/db",
    "SILKBOARD_OWNER_ORIGIN=https://beta.example.ts.net",
    "/srv/beta/web",
    "owner",
    "prepare-setup",
  ]);
});
test("private real database files only", (t) => {
  const dir = directory(t),
    database = path.join(dir, "db");
  fs.writeFileSync(database, "fixture", { mode: 0o600 });
  checkDatabase({ database }, fs.statSync(database).uid);
  assert.throws(() =>
    checkDatabase({ database }, fs.statSync(database).uid + 1),
  );
  fs.chmodSync(database, 0o644);
  assert.throws(() => checkDatabase({ database }, fs.statSync(database).uid));
  fs.chmodSync(database, 0o600);
  fs.symlinkSync(database, path.join(dir, "link"));
  assert.throws(() =>
    checkDatabase(
      { database: path.join(dir, "link") },
      fs.statSync(database).uid,
    ),
  );
});
test("state reads reject failed, oversized and malformed responses", async (t) => {
  for (const [body, status] of [
    ['{"state":"available"}', 200],
    ["{}", 503],
    ["x".repeat(2048), 200],
    ["not-json", 200],
  ]) {
    const server = application(t, body, status);
    await once(server, "listening");
    const config = {
      applicationUrl: `http://127.0.0.1:${server.address().port}`,
    };
    if (status === 200 && body.startsWith("{"))
      assert.equal(await readSetupState(config), "available");
    else await assert.rejects(readSetupState(config));
  }
});
test("closing server terminates only its foreground proxy; signals remove listeners", async (t) => {
  const server = http.createServer();
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const proxy = proxyFixture(),
    signals = new EventEmitter();
  attachProxy(server, proxy, signals);
  const closed = once(server, "close");
  signals.emit("SIGTERM");
  await closed;
  assert.deepEqual(proxy.signals, ["SIGTERM"]);
  assert.equal(signals.listenerCount("SIGTERM"), 0);
});
test("unexpected proxy exit is reported as failure and closes backend", async () => {
  const server = http.createServer();
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const proxy = proxyFixture();
  let failed = false;
  attachProxy(server, proxy, new EventEmitter(), () => {
    failed = true;
  });
  const closed = once(server, "close");
  proxy.emit("exit", 1, null);
  await closed;
  assert.equal(failed, true);
  assert(!server.listening);
});
test("orchestration uses dynamic guest addresses and refuses identity drift before a credential", async (t) => {
  const dir = directory(t),
    signals = new EventEmitter(),
    proxy = proxyFixture();
  let captured,
    argumentsSeen,
    changed = false;
  const config = validateConfig({
    origin: "https://beta.example.ts.net",
    ownerLogin: "bob@example.test",
    handoffPort: 8444,
  });
  const tailnet = () => ({
    BackendState: "Running",
    Self: {
      DNSName: "beta.example.ts.net.",
      TailscaleIPs: [changed ? "100.64.0.9" : "100.64.0.8"],
    },
  });
  const serve = {
    TCP: { 443: { HTTPS: true } },
    Web: {
      "beta.example.ts.net:443": {
        Handlers: { "/": { Proxy: config.applicationUrl } },
      },
    },
  };
  const run = await startHandoff(config, {
    capture: async (_cmd, args) =>
      _cmd === "/usr/bin/id"
        ? "1000\n"
        : JSON.stringify(args[0] === "serve" ? serve : tailnet()),
    readState: async () => "waiting",
    checkDatabase: () => {},
    createServer: (options) => {
      captured = options;
      return createHandoff({
        ...options,
        socketPath: path.join(dir, "http.sock"),
      });
    },
    spawnProxy: (_command, args) => {
      argumentsSeen = args;
      return proxy;
    },
    signalTarget: signals,
  });
  t.after(() => {
    run.server.close();
    run.server.closeAllConnections();
  });
  assert.deepEqual(captured.selfAddresses, ["100.64.0.8"]);
  assert.equal(captured.login, "bob@example.test");
  assert.deepEqual(argumentsSeen, [
    "serve",
    "--yes",
    "--https=8444",
    "unix:/run/silkboard-owner-handoff/http.sock",
  ]);
  changed = true;
  await assert.rejects(
    captured.prepare(config.origin, new AbortController().signal),
    /Guest identity changed/,
  );
  signals.emit("SIGINT");
  await once(run.server, "close");
  assert.deepEqual(proxy.signals, ["SIGTERM"]);
});

test("a UID-zero alias cannot launch the handoff or application", async () => {
  const config = validateConfig({
    origin: "https://beta.example.ts.net",
    ownerLogin: "bob@example.test",
    applicationUser: "toor",
  });
  let launched = false;
  await assert.rejects(
    startHandoff(config, {
      capture: async () => "0\n",
      checkDatabase: () => {},
      createServer: () => {
        launched = true;
        throw new Error("must not launch");
      },
    }),
    /non-root application account/,
  );
  assert.equal(launched, false);
});
