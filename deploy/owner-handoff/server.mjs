// Temporary operational transport; never log request data or child output.
import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { isIP } from "node:net";
import { randomBytes } from "node:crypto";
import { execFile } from "node:child_process";

const SOCKET = "/run/silkboard-owner-handoff/http.sock";

function remoteTailnetAddress(value, selfAddresses) {
  if (typeof value !== "string" || selfAddresses.includes(value)) return false;
  if (isIP(value) === 4) {
    const parts = value.split(".").map(Number);
    return parts[0] === 100 && parts[1] >= 64 && parts[1] <= 127;
  }
  // Serve emits canonical source IPs, never a client-controlled forwarding chain.
  return isIP(value) === 6 && value.startsWith("fd7a:115c:a1e0:");
}

function validLink(value, target) {
  return (
    typeof value === "string" &&
    value.startsWith(`${target}/setup#setup=`) &&
    /^[a-f0-9]{64}$/.test(value.slice(`${target}/setup#setup=`.length))
  );
}

// stdout is captured only in memory, stderr discarded, neither is logged.
export function prepareLink(target, signal, command, args, timeout = 10_000) {
  return new Promise((resolve, reject) => {
    execFile(
      command,
      args,
      {
        encoding: "utf8",
        maxBuffer: 1024,
        timeout,
        signal,
        env: { PATH: "/usr/sbin:/usr/bin:/sbin:/bin", LANG: "C.UTF-8" },
      },
      (error, stdout) => {
        const value = stdout?.endsWith("\n") ? stdout.slice(0, -1) : stdout;
        if (error || !validLink(value, target))
          reject(new Error("Setup preparation failed."));
        else resolve(value);
      },
    );
  });
}

export function createHandoff({
  socketPath = SOCKET,
  origin,
  target,
  login,
  selfAddresses,
  prepare,
  lifetimeMs = 1_200_000,
  closeDelayMs = 2000,
} = {}) {
  if (
    !Array.isArray(selfAddresses) ||
    selfAddresses.length === 0 ||
    selfAddresses.some((ip) => !isIP(ip))
  )
    throw new Error("Guest addresses required.");
  const parent = fs.lstatSync(path.dirname(socketPath));
  if (
    !parent.isDirectory() ||
    (parent.mode & 0o077) !== 0 ||
    parent.uid !== process.getuid()
  ) {
    throw new Error("Private socket directory required.");
  }
  if (fs.existsSync(socketPath)) throw new Error("Socket already exists.");
  const nonce = randomBytes(32).toString("hex");
  const controller = new AbortController();
  let busy = false;
  let delivered = false;
  let expired = false;
  const headers = {
    "content-type": "text/html; charset=utf-8",
    "cache-control": "no-store",
    "referrer-policy": "no-referrer",
    "x-content-type-options": "nosniff",
    "x-frame-options": "DENY",
    "content-security-policy": `default-src 'none'; style-src 'unsafe-inline'; form-action 'self' ${target}; frame-ancestors 'none'; base-uri 'none'`,
  };
  const page = `<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Silkboard owner setup</title><style>body{font:18px system-ui;max-width:32rem;margin:3rem auto;padding:0 1.2rem;color:#17212b;background:#f5f7fa}p{line-height:1.5}button{font:inherit;width:100%;padding:1rem;background:#203c52;color:white;border:0;border-radius:.4rem}</style><h1>Set up your Silkboard account</h1><p>This temporary page is restricted to your Tailscale account. Continue to the private registration page and choose your owner password there.</p><p>The handoff closes after delivery or 20 minutes. If registration is interrupted, contact your operator for a new handoff.</p><form method="post" action="/" autocomplete="off"><input type="hidden" name="nonce" value="${nonce}"><button type="submit">Continue to setup</button></form></html>`;
  const stop = () => {
    expired = true;
    controller.abort();
    server.close();
    server.closeAllConnections();
  };
  const server = http.createServer(async (req, res) => {
    const reply = (status, body, extra = {}) => {
      res.writeHead(status, { ...headers, ...extra });
      res.end(body);
    };
    if (
      !remoteTailnetAddress(req.headers["x-forwarded-for"], selfAddresses) ||
      req.headers.host !== "localhost" ||
      req.headers["x-forwarded-host"] !== new URL(origin).host ||
      req.headers["x-forwarded-proto"] !== "https" ||
      req.headers["tailscale-user-login"] !== login ||
      req.headers["tailscale-funnel-request"] !== undefined
    )
      return reply(403, "Access denied. Use your own Tailscale connection.");
    if (req.url !== "/") return reply(404, "Not found.");
    if (expired || delivered)
      return reply(410, "This handoff has closed. Contact your operator.");
    // Form navigation needs its same-origin Origin; this page contains no setup secret.
    if (req.method === "GET")
      return reply(200, page, { "referrer-policy": "same-origin" });
    if (req.method !== "POST")
      return reply(405, "Use the button on this page.");
    if (
      req.headers.origin !== origin ||
      (req.headers["sec-fetch-site"] !== undefined &&
        req.headers["sec-fetch-site"] !== "same-origin")
    )
      return reply(403, "Open this private page directly and try again.");
    if (req.headers["content-type"] !== "application/x-www-form-urlencoded")
      return reply(415, "Unsupported submission.");
    let body = "";
    try {
      for await (const chunk of req) {
        body += chunk.toString("utf8");
        if (Buffer.byteLength(body) > 256)
          return reply(413, "Submission too large.");
      }
    } catch {
      return;
    }
    const values = new URLSearchParams(body);
    if ([...values].length !== 1 || values.get("nonce") !== nonce)
      return reply(403, "Reload this private page and try again.");
    if (busy) return reply(409, "Setup is being prepared. Please wait.");
    busy = true;
    try {
      const link = await prepare(target, controller.signal);
      if (!validLink(link, target)) throw new Error("Invalid setup link.");
      if (expired || res.destroyed) {
        busy = false;
        return;
      }
      delivered = true;
      res.once("finish", () => setTimeout(stop, closeDelayMs).unref());
      reply(303, "Continue to the private setup page.", { location: link });
    } catch {
      busy = false;
      if (!expired && !res.destroyed)
        reply(503, "Setup could not be prepared. Return to the conversation.");
    }
  });
  server.requestTimeout = 10_000;
  server.headersTimeout = 10_000;
  server.keepAliveTimeout = 1000;
  server.maxHeadersCount = 30;
  server.maxConnections = 8;
  const expiry = setTimeout(stop, lifetimeMs);
  expiry.unref();
  server.once("close", () => {
    clearTimeout(expiry);
    controller.abort();
  });
  server.once("listening", () => fs.chmodSync(socketPath, 0o600));
  server.on("error", () => stop());
  server.listen(socketPath);
  return server;
}
