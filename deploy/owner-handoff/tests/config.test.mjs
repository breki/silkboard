import assert from "node:assert/strict";
import { test } from "node:test";
import { validateConfig, renderUnit, checkPreflight } from "../config.mjs";
const sample = {
  origin: "https://alpha.example.ts.net",
  ownerLogin: "alice@example.test",
};
const tailnet = {
  BackendState: "Running",
  Self: { DNSName: "alpha.example.ts.net.", TailscaleIPs: ["100.64.0.1"] },
};
const serve = {
  TCP: { 443: { HTTPS: true } },
  Web: {
    "alpha.example.ts.net:443": {
      Handlers: { "/": { Proxy: "http://127.0.0.1:3000" } },
    },
  },
};
test("two servers and application identities require configuration only", () => {
  const a = validateConfig(sample);
  const b = validateConfig({
    origin: "https://beta.example.ts.net:9443",
    ownerLogin: "bob@example.test",
    database: "/srv/beta/state/db.sqlite",
    applicationBinary: "/srv/beta/current/web",
    applicationUser: "beta",
    applicationService: "beta.service",
    applicationUrl: "http://127.0.0.1:5000",
    handoffPort: 8444,
  });
  assert.equal(a.handoffOrigin, "https://alpha.example.ts.net:8443");
  assert.equal(b.handoffOrigin, "https://beta.example.ts.net:8444");
  const unit = renderUnit(b, "/opt/node/bin/node");
  assert.match(unit, /Requires=tailscaled.service beta.service/);
  assert.match(unit, /ReadWritePaths=\/srv\/beta\/state/);
  assert.match(unit, /AmbientCapabilities=CAP_SETUID/);
  assert.match(unit, /NoNewPrivileges=yes/);
  assert(!unit.includes("[Install]"));
});
test("reject injection, unsafe origins, paths, users, ports and unknown fields", () => {
  for (const change of [
    { origin: "http://alpha.example.ts.net" },
    { origin: "https://x.test/path" },
    { origin: "https://user@x.test" },
    { origin: "https://x.test#secret" },
    { ownerLogin: "x\nExecStart=bad" },
    { applicationUser: "root" },
    { applicationUser: "user --bad" },
    { applicationService: "x.service\nBad=1" },
    { database: "/srv/data space/db" },
    { database: "/srv/%h/db" },
    { database: "/srv/../db" },
    { database: "relative" },
    { applicationBinary: "/bin/x\nBad=1" },
    { handoffPort: 443 },
    { handoffPort: 0 },
    { handoffPort: 65536 },
    { handoffPort: "8443" },
    { applicationUrl: "http://remote.test:3000" },
    { applicationUrl: "http://localhost:3000/x" },
    { typo: "ignored" },
  ])
    assert.throws(() => validateConfig({ ...sample, ...change }));
  assert.throws(() =>
    renderUnit(validateConfig(sample), "/opt/node %h/bin/node"),
  );
});
test("preflight derives self addresses and permits replacing a pending link", () => {
  const c = validateConfig(sample);
  for (const state of ["waiting", "available"])
    assert.deepEqual(checkPreflight(c, tailnet, serve, state), ["100.64.0.1"]);
  for (const state of ["closed", "unknown", undefined])
    assert.throws(() => checkPreflight(c, tailnet, serve, state));
  for (const node of [
    { ...tailnet, BackendState: "Stopped" },
    { ...tailnet, Self: { ...tailnet.Self, DNSName: "other.ts.net." } },
    { ...tailnet, Self: { ...tailnet.Self, TailscaleIPs: [] } },
  ])
    assert.throws(() => checkPreflight(c, node, serve, "waiting"));
  assert.throws(() =>
    checkPreflight(
      c,
      tailnet,
      { ...serve, TCP: { ...serve.TCP, 8443: { HTTPS: true } } },
      "waiting",
    ),
  );
  assert.throws(() =>
    checkPreflight(
      c,
      tailnet,
      { ...serve, AllowFunnel: { "alpha.example.ts.net:443": true } },
      "waiting",
    ),
  );
  assert.throws(() =>
    checkPreflight(
      c,
      tailnet,
      { ...serve, Foreground: { session: { TCP: { 8443: { HTTPS: true } } } } },
      "waiting",
    ),
  );
  assert.throws(() => checkPreflight(c, tailnet, {}, "waiting"));
});

test("install rendering keeps configuration private and refuses symlinks/overwrites", async (t) => {
  const fs = await import("node:fs"),
    os = await import("node:os"),
    path = await import("node:path");
  const { execFileSync } = await import("node:child_process");
  const { readConfig } = await import("../config.mjs");
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "handoff-config-"));
  fs.chmodSync(dir, 0o700);
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const input = path.join(dir, "input.json"),
    output = path.join(dir, "output");
  fs.mkdirSync(output, { mode: 0o700 });
  fs.writeFileSync(input, JSON.stringify(sample), { mode: 0o600 });
  execFileSync(
    process.execPath,
    [new URL("../config.mjs", import.meta.url).pathname, input, output],
    { stdio: "pipe" },
  );
  const installed = readConfig(path.join(output, "config.json"));
  assert.equal(installed.ownerLogin, sample.ownerLogin);
  assert.equal(
    fs.statSync(path.join(output, "config.json")).mode & 0o777,
    0o600,
  );
  assert.match(
    fs.readFileSync(path.join(output, "handoff.service"), "utf8"),
    /ExecStart=.*\/main.mjs/,
  );
  assert.throws(() =>
    execFileSync(
      process.execPath,
      [new URL("../config.mjs", import.meta.url).pathname, input, output],
      { stdio: "pipe" },
    ),
  );
  fs.symlinkSync(input, path.join(dir, "link"));
  assert.throws(() => readConfig(path.join(dir, "link")));
  fs.chmodSync(input, 0o644);
  assert.throws(() => readConfig(input, true));
});
