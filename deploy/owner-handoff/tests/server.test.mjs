import assert from "node:assert/strict";
import { test } from "node:test";
import http from "node:http";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { once } from "node:events";
import { createHandoff, prepareLink } from "../server.mjs";

const origin = "https://dashboard.example.ts.net:8443";
const target = "https://dashboard.example.ts.net";
const login = "owner@example.test";
const link = `${target}/setup#setup=${"a".repeat(64)}`;
function request(socketPath, options = {}) {
  return new Promise((resolve, reject) => {
    const req = http.request(
      {
        socketPath,
        path: "/",
        method: "GET",
        ...options,
        headers: {
          host: "localhost",
          "tailscale-user-login": login,
          "x-forwarded-for": "100.80.70.60",
          "x-forwarded-host": new URL(origin).host,
          "x-forwarded-proto": "https",
          ...options.headers,
        },
      },
      (res) => {
        let body = "";
        res.on("data", (chunk) => (body += chunk));
        res.on("end", () =>
          resolve({ status: res.statusCode, headers: res.headers, body }),
        );
      },
    );
    req.on("error", reject);
    req.end(options.body);
  });
}
async function fixture(t, prepare = async () => link, extra = {}) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "owner-handoff-test-"));
  fs.chmodSync(dir, 0o700);
  const socketPath = path.join(dir, "http.sock");
  const server = createHandoff({
    socketPath,
    origin,
    target,
    login,
    selfAddresses: ["100.64.0.1", "fd7a:115c:a1e0::1"],
    prepare,
    ...extra,
  });
  await once(server, "listening");
  t.after(() => {
    server.close();
    server.closeAllConnections();
    fs.rmSync(dir, { recursive: true, force: true });
  });
  const get = await request(socketPath);
  const nonce = get.body.match(/name="nonce" value="([a-f0-9]{64})"/)?.[1];
  return {
    socketPath,
    server,
    get,
    nonce,
    post: (options = {}) =>
      request(socketPath, {
        method: "POST",
        body: `nonce=${nonce}`,
        ...options,
        headers: {
          origin,
          "content-type": "application/x-www-form-urlencoded",
          "sec-fetch-site": "same-origin",
          ...options.headers,
        },
      }),
  };
}
test("GET never prepares a link; exact owner only and protected socket", async (t) => {
  let calls = 0;
  const f = await fixture(t, async () => {
    calls++;
    return link;
  });
  assert.equal(f.get.status, 200);
  assert.equal(calls, 0);
  assert.equal(fs.statSync(f.socketPath).mode & 0o777, 0o600);
  assert.equal(f.get.headers["cache-control"], "no-store");
  assert.equal(f.get.headers["referrer-policy"], "same-origin");
  assert(!f.get.body.includes(link));
  for (const headers of [
    { "tailscale-user-login": "" },
    { "tailscale-user-login": "other@example.test" },
    { "tailscale-user-login": `${login}, ${login}` },
    { "tailscale-funnel-request": "?1" },
    { "x-forwarded-for": "100.64.0.1" },
    { "x-forwarded-for": "fd7a:115c:a1e0::1" },
    { "x-forwarded-for": "::ffff:100.64.0.1" },
    { "x-forwarded-for": "" },
    { "x-forwarded-for": "127.0.0.1" },
    { "x-forwarded-for": "100.80.70.60, 100.64.0.1" },
    { "x-forwarded-host": "evil.test" },
    { "x-forwarded-proto": "http" },
    { host: "evil.test" },
  ]) {
    assert.equal((await request(f.socketPath, { headers })).status, 403);
  }
  assert.equal(calls, 0);
});
test("explicit same-origin nonce POST delivers once and closes", async (t) => {
  let calls = 0;
  const f = await fixture(
    t,
    async () => {
      calls++;
      return link;
    },
    { closeDelayMs: 1000 },
  );
  const r = await f.post();
  assert.equal(r.status, 303);
  assert.equal(r.headers.location, link);
  assert.equal(r.headers["referrer-policy"], "no-referrer");
  assert(!r.body.includes(link));
  assert.equal(calls, 1);
  assert.equal((await f.post()).status, 410);
  assert.equal(calls, 1);
});
test("cross-origin, malformed and oversized submissions cannot prepare", async (t) => {
  let calls = 0;
  const f = await fixture(t, async () => {
    calls++;
    return link;
  });
  for (const options of [
    { headers: { origin: "https://evil.test" } },
    { headers: { origin: "" } },
    { headers: { "sec-fetch-site": "cross-site" } },
    { headers: { "content-type": "application/json" } },
    { body: "nonce=bad" },
    { body: `nonce=${f.nonce}&nonce=${f.nonce}` },
    { body: `nonce=${f.nonce}&extra=1` },
    { body: "x".repeat(1000) },
  ]) {
    assert((await f.post(options)).status >= 400);
  }
  assert.equal(calls, 0);
});
test("concurrent requests cannot generate competing links", async (t) => {
  let release;
  let calls = 0;
  const f = await fixture(t, () => {
    calls++;
    return new Promise((r) => (release = r));
  });
  const first = f.post();
  await new Promise((r) => setTimeout(r, 20));
  assert.equal((await f.post()).status, 409);
  release(link);
  assert.equal((await first).status, 303);
  assert.equal(calls, 1);
});
test("bad child output is never returned, failure permits deliberate retry", async (t) => {
  let calls = 0;
  const f = await fixture(t, async () =>
    ++calls === 1 ? "https://evil.test/secret" : link,
  );
  const r = await f.post();
  assert.equal(r.status, 503);
  assert(!r.body.includes("evil"));
  assert.equal((await f.post()).status, 303);
});
test("lifetime expiry closes even idle connections", async (t) => {
  const f = await fixture(t, undefined, { lifetimeMs: 100 });
  await once(f.server, "close");
  assert(!f.server.listening);
});
test("private child pipe validates output and suppresses stderr on errors", async () => {
  const command = process.execPath;
  assert.equal(
    await prepareLink(target, undefined, command, [
      "-e",
      `process.stdout.write(${JSON.stringify(link + "\n")})`,
    ]),
    link,
  );
  for (const code of [
    "process.stderr.write('SECRET');process.exit(1)",
    "process.stdout.write('x'.repeat(2048))",
    "setTimeout(()=>{},2000)",
    `process.stdout.write(${JSON.stringify(link + "\nextra")})`,
  ]) {
    await assert.rejects(
      prepareLink(target, undefined, command, ["-e", code], 100),
      /^Error: Setup preparation failed\.$/,
    );
  }
});
