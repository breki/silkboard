import fs from "node:fs";
import path from "node:path";
import { execFile, spawn } from "node:child_process";
import { once } from "node:events";
import { checkPreflight } from "./config.mjs";
import { createHandoff, prepareLink } from "./server.mjs";

const environment = { PATH: "/usr/sbin:/usr/bin:/sbin:/bin", LANG: "C.UTF-8" };

export function capture(command, args) {
  return new Promise((resolve, reject) => {
    execFile(
      command,
      args,
      {
        encoding: "utf8",
        timeout: 10_000,
        maxBuffer: 524288,
        env: environment,
      },
      (error, stdout) => {
        if (error) reject(new Error("Handoff preflight failed."));
        else resolve(stdout);
      },
    );
  });
}

export function preparationArgs(config) {
  return [
    "-u",
    config.applicationUser,
    "--",
    "/usr/bin/env",
    `SILKBOARD_DATABASE=${config.database}`,
    `SILKBOARD_OWNER_ORIGIN=${config.origin}`,
    config.applicationBinary,
    "owner",
    "prepare-setup",
  ];
}

export async function readSetupState(config) {
  const response = await fetch(`${config.applicationUrl}/api/auth/setup`, {
    redirect: "error",
    signal: AbortSignal.timeout(5000),
  });
  if (!response.ok) throw new Error("Owner setup is unavailable.");
  let body = "";
  for await (const chunk of response.body) {
    body += Buffer.from(chunk).toString("utf8");
    if (Buffer.byteLength(body) > 1024)
      throw new Error("Owner setup is unavailable.");
  }
  return JSON.parse(body).state;
}

export function checkDatabase(config, applicationUid) {
  const file = fs.lstatSync(config.database);
  const directory = fs.lstatSync(path.dirname(config.database));
  if (
    !file.isFile() ||
    !directory.isDirectory() ||
    file.uid !== applicationUid ||
    directory.uid !== applicationUid ||
    file.mode & 0o077 ||
    directory.mode & 0o077 ||
    fs.realpathSync(config.database) !== config.database
  )
    throw new Error("Private application state required.");
}

// A foreground Serve process owns only its ephemeral listener. Closing either
// side closes the other; there is no global reset or port-off cleanup command.
export function attachProxy(
  server,
  proxy,
  signalTarget = process,
  onFailure = () => {},
) {
  let stopping = false;
  const stop = () => {
    if (stopping) return;
    stopping = true;
    server.close();
    server.closeAllConnections();
    proxy.kill("SIGTERM");
  };
  const onExit = () => {
    if (!stopping) onFailure();
    stop();
  };
  signalTarget.once("SIGTERM", stop);
  signalTarget.once("SIGINT", stop);
  proxy.once("exit", onExit);
  proxy.once("error", onExit);
  server.once("close", () => {
    stop();
    signalTarget.removeListener("SIGTERM", stop);
    signalTarget.removeListener("SIGINT", stop);
  });
  return stop;
}

export async function startHandoff(config, io = {}) {
  const run = io.capture ?? capture;
  const state = io.readState ?? readSetupState;
  const uidText = (
    await run("/usr/bin/id", ["-u", "--", config.applicationUser])
  ).trim();
  if (
    !/^[0-9]+$/.test(uidText) ||
    !Number.isSafeInteger(Number(uidText)) ||
    Number(uidText) === 0
  )
    throw new Error("An existing non-root application account is required.");
  (io.checkDatabase ?? checkDatabase)(config, Number(uidText));
  const tailnet = JSON.parse(
    await run("/usr/bin/tailscale", ["status", "--json"]),
  );
  const serve = JSON.parse(
    await run("/usr/bin/tailscale", ["serve", "status", "--json"]),
  );
  const selfAddresses = checkPreflight(
    config,
    tailnet,
    serve,
    await state(config),
  );
  const prepare = async (target, signal) => {
    // Refuse address/DNS drift before preparing a secret, even if the
    // guest was re-enrolled during this twenty-minute window.
    const current = JSON.parse(
      await run("/usr/bin/tailscale", ["status", "--json"]),
    );
    checkPreflight(config, current, serve, "available");
    if (
      JSON.stringify(current.Self.TailscaleIPs) !==
      JSON.stringify(selfAddresses)
    )
      throw new Error("Guest identity changed.");
    return prepareLink(
      target,
      signal,
      "/usr/sbin/runuser",
      preparationArgs(config),
    );
  };
  const server = (io.createServer ?? createHandoff)({
    origin: config.handoffOrigin,
    target: config.origin,
    login: config.ownerLogin,
    selfAddresses,
    prepare,
  });
  await once(server, "listening");
  let proxy;
  try {
    proxy = (io.spawnProxy ?? spawn)(
      "/usr/bin/tailscale",
      [
        "serve",
        "--yes",
        `--https=${config.handoffPort}`,
        "unix:/run/silkboard-owner-handoff/http.sock",
      ],
      { stdio: "ignore", env: environment },
    );
  } catch {
    server.close();
    server.closeAllConnections();
    throw new Error("Private handoff could not start.");
  }
  attachProxy(
    server,
    proxy,
    io.signalTarget,
    io.onProxyFailure ??
      (() => {
        process.exitCode = 1;
      }),
  );
  return { server, proxy };
}
