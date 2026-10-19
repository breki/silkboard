// Configuration is operator authority; it contains no setup credential.
import fs from "node:fs";
import path from "node:path";
import { isIP } from "node:net";
import { fileURLToPath } from "node:url";

const defaults = {
  database: "/var/lib/silkboard/progress/progress.sqlite",
  applicationBinary: "/opt/silkboard/current/silkboard-web",
  applicationUser: "silkboard",
  applicationService: "silkboard.service",
  applicationUrl: "http://127.0.0.1:3000",
  handoffPort: 8443,
};
const fail = () => {
  throw new Error("Invalid handoff configuration or preflight.");
};

export function safePath(value) {
  return (
    typeof value === "string" &&
    /^\/[A-Za-z0-9_./-]+$/.test(value) &&
    path.normalize(value) === value &&
    value !== "/"
  );
}

export function validateConfig(input) {
  if (!input || Array.isArray(input) || typeof input !== "object") fail();
  const allowed = ["origin", "ownerLogin", ...Object.keys(defaults)];
  if (Object.keys(input).some((key) => !allowed.includes(key))) fail();
  const config = { ...defaults, ...input };
  let origin, application;
  try {
    origin = new URL(config.origin);
    application = new URL(config.applicationUrl);
  } catch {
    fail();
  }
  if (
    origin.protocol !== "https:" ||
    origin.origin !== config.origin ||
    origin.username ||
    origin.password ||
    !/^[a-z0-9.-]+$/.test(origin.hostname)
  )
    fail();
  if (
    application.protocol !== "http:" ||
    application.origin !== config.applicationUrl ||
    !["127.0.0.1", "[::1]"].includes(application.hostname)
  )
    fail();
  if (
    typeof config.ownerLogin !== "string" ||
    !/^[A-Za-z0-9][A-Za-z0-9@._+-]{0,253}$/.test(config.ownerLogin)
  )
    fail();
  if (
    !/^[a-z_][a-z0-9_-]{0,31}$/.test(config.applicationUser) ||
    config.applicationUser === "root"
  )
    fail();
  if (!/^[A-Za-z0-9][A-Za-z0-9_.-]*\.service$/.test(config.applicationService))
    fail();
  if (
    !safePath(config.database) ||
    path.dirname(config.database) === "/" ||
    !safePath(config.applicationBinary)
  )
    fail();
  if (
    !Number.isInteger(config.handoffPort) ||
    config.handoffPort < 1024 ||
    config.handoffPort > 65535 ||
    config.handoffPort === Number(origin.port || 443)
  )
    fail();
  const handoff = new URL(config.origin);
  handoff.port = String(config.handoffPort);
  return { ...config, handoffOrigin: handoff.origin };
}

export function readConfig(filename, requireRoot = false) {
  const stat = fs.lstatSync(filename);
  if (
    !stat.isFile() ||
    stat.size > 8192 ||
    (requireRoot && (stat.uid !== 0 || stat.mode & 0o077))
  )
    fail();
  return validateConfig(JSON.parse(fs.readFileSync(filename, "utf8")));
}

export function renderUnit(config, nodeBinary) {
  if (!safePath(nodeBinary)) fail();
  return `[Unit]
Description=Temporary private Silkboard owner setup handoff
After=tailscaled.service ${config.applicationService}
Requires=tailscaled.service ${config.applicationService}

[Service]
Type=simple
User=root
Group=root
UMask=0077
RuntimeDirectory=silkboard-owner-handoff
RuntimeDirectoryMode=0700
RuntimeMaxSec=20min
TimeoutStopSec=10s
KillMode=control-group
Restart=no
LimitCORE=0
MemorySwapMax=0
MemoryMax=128M
TasksMax=32
NoNewPrivileges=yes
AmbientCapabilities=CAP_SETUID
PrivateTmp=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=${path.dirname(config.database)}
RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6
UnsetEnvironment=NODE_OPTIONS NODE_PATH
ExecStart=${nodeBinary} /usr/local/libexec/silkboard-owner-handoff/main.mjs
StandardOutput=null
StandardError=journal
`;
}

export function checkPreflight(config, tailnet, serve, state) {
  const addresses = tailnet?.Self?.TailscaleIPs;
  if (
    tailnet?.BackendState !== "Running" ||
    !Array.isArray(addresses) ||
    addresses.length === 0 ||
    addresses.some((ip) => !isIP(ip)) ||
    tailnet.Self.DNSName?.replace(/\.$/, "").toLowerCase() !==
      new URL(config.origin).hostname ||
    !["waiting", "available"].includes(state)
  )
    fail();
  const configurations = [serve, ...Object.values(serve?.Foreground ?? {})];
  if (
    configurations.some(
      (c) =>
        c?.TCP?.[config.handoffPort] ||
        Object.values(c?.AllowFunnel ?? {}).some(Boolean),
    )
  )
    fail();
  const main = new URL(config.origin);
  const host = `${main.hostname}:${main.port || 443}`;
  if (
    serve?.TCP?.[main.port || 443]?.HTTPS !== true ||
    serve?.Web?.[host]?.Handlers?.["/"]?.Proxy !== config.applicationUrl
  )
    fail();
  return addresses;
}

// Installation renders only validated non-secret configuration and unit files.
if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    if (process.argv.length !== 4) fail();
    const config = readConfig(process.argv[2]);
    const { handoffOrigin: _derived, ...stored } = config;
    fs.writeFileSync(
      path.join(process.argv[3], "config.json"),
      JSON.stringify(stored, null, 2) + "\n",
      { flag: "wx", mode: 0o600 },
    );
    fs.writeFileSync(
      path.join(process.argv[3], "handoff.service"),
      renderUnit(config, process.execPath),
      { flag: "wx", mode: 0o600 },
    );
  } catch {
    process.stderr.write("Handoff configuration could not be rendered.\n");
    process.exitCode = 1;
  }
}
