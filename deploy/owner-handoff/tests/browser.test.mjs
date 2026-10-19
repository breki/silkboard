import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import http from "node:http";
import https from "node:https";
import { execFileSync } from "node:child_process";
import { once } from "node:events";
import { chromium } from "../../../frontend/node_modules/@playwright/test/index.mjs";
import { createHandoff } from "../server.mjs";

test("HTTPS mobile browser deliberately receives a fixture link; other identity is denied", async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "owner-handoff-browser-"));
  fs.chmodSync(dir, 0o700);
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
      "-keyout",
      path.join(dir, "key.pem"),
      "-out",
      path.join(dir, "cert.pem"),
    ],
    { stdio: "ignore" },
  );
  const socketPath = path.join(dir, "http.sock");
  let login = "owner@example.test";
  let calls = 0;
  let handoff;
  let origin;
  const tls = {
    key: fs.readFileSync(path.join(dir, "key.pem")),
    cert: fs.readFileSync(path.join(dir, "cert.pem")),
  };
  const setup = https.createServer(tls, (req, res) => {
    if (req.url === "/setup") {
      res.writeHead(200, {
        "content-type": "text/html",
        "cache-control": "no-store",
      });
      // Fixture only; production fragment clearing is already covered by the application's suite.
      return res.end(
        '<script>history.replaceState(null,"","/setup")</script><h1>Choose your owner password</h1>',
      );
    }
    res.writeHead(404);
    res.end();
  });
  setup.listen(0, "127.0.0.1");
  await once(setup, "listening");
  const target = `https://127.0.0.1:${setup.address().port}`;
  const proxy = https.createServer(tls, (req, res) => {
    const upstream = http.request(
      {
        socketPath,
        path: req.url,
        method: req.method,
        headers: {
          ...req.headers,
          host: "localhost",
          "x-forwarded-host": new URL(origin).host,
          "x-forwarded-proto": "https",
          "x-forwarded-for": "100.80.70.60",
          "tailscale-user-login": login,
        },
      },
      (r) => {
        res.writeHead(r.statusCode, r.headers);
        r.pipe(res);
      },
    );
    upstream.on("error", () => {
      res.writeHead(503);
      res.end();
    });
    req.pipe(upstream);
  });
  proxy.listen(0, "127.0.0.1");
  await once(proxy, "listening");
  origin = `https://127.0.0.1:${proxy.address().port}`;
  handoff = createHandoff({
    socketPath,
    origin,
    target,
    login,
    selfAddresses: ["100.64.0.1"],
    prepare: async () => {
      calls++;
      return `${target}/setup#setup=${"a".repeat(64)}`;
    },
  });
  await once(handoff, "listening");
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({
      ignoreHTTPSErrors: true,
      viewport: { width: 375, height: 812 },
      isMobile: true,
      hasTouch: true,
    });
    login = "other@example.test";
    assert.equal((await page.goto(origin)).status(), 403);
    login = "owner@example.test";
    assert.equal((await page.goto(origin)).status(), 200);
    assert.equal(calls, 0);
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      ),
      false,
    );

    await Promise.all([
      page.waitForURL(`${target}/setup`, { timeout: 3000 }),
      page.getByRole("button", { name: "Continue to setup" }).click(),
    ]);
    assert.equal(
      await page.getByRole("heading").textContent(),
      "Choose your owner password",
    );
    assert.equal(calls, 1);
    assert.equal(new URL(page.url()).hash, "");
  } finally {
    await browser.close();
    handoff.close();
    handoff.closeAllConnections();
    proxy.close();
    proxy.closeAllConnections();
    setup.close();
    setup.closeAllConnections();
    fs.rmSync(dir, { recursive: true, force: true });
  }
});
