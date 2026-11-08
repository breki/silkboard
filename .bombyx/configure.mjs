import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { execFileSync } from "node:child_process";
import { isDeepStrictEqual } from "node:util";
import { directory, file, replace, paths } from "./private-files.mjs";
export function configure(home, checkout, endpoint) {
  if (typeof endpoint !== "string" || !endpoint)
    throw Error("endpoint required");
  const origin = new URL(endpoint);
  if (
    origin.protocol !== "https:" ||
    origin.username ||
    origin.password ||
    origin.port === "0" ||
    origin.origin !== endpoint
  )
    throw Error("HTTPS origin required");
  const root = fs.realpathSync(checkout);
  if (root !== path.resolve(home, "silkboard")) throw Error("checkout");
  const settings = JSON.parse(
    fs.readFileSync(new URL("./settings.json", import.meta.url), "utf8"),
  );
  const key = path.join(home, ".ssh/bombyx-deploy-key");
  file(key);
  const p = paths(home);
  const local = path.join(home, ".local");
  if (!fs.existsSync(local)) fs.mkdirSync(local, { mode: 0o700 });
  const localStat = fs.lstatSync(local);
  if (
    !localStat.isDirectory() ||
    localStat.uid !== process.getuid() ||
    (localStat.mode & 0o022) !== 0 ||
    fs.realpathSync(local) !== local
  )
    throw Error("local");
  const stateParent = path.join(local, "state");
  if (!fs.existsSync(stateParent)) fs.mkdirSync(stateParent, { mode: 0o700 });
  const stateStat = fs.lstatSync(stateParent);
  if (
    !stateStat.isDirectory() ||
    stateStat.uid !== process.getuid() ||
    (stateStat.mode & 0o022) !== 0 ||
    fs.realpathSync(stateParent) !== stateParent
  )
    throw Error("state");
  const state = path.join(stateParent, "silkboard-publish");
  directory(state);
  const config = {
    endpoint,
    project: settings.project,
    token_file: p.token,
    state_dir: state,
    codex: {
      workspace: root,
      hostname: os.hostname(),
      session_name: "Silkboard development",
    },
  };
  if (fs.lstatSync(p.config, { throwIfNoEntry: false })) {
    const existing = JSON.parse(file(p.config));
    if (existing.codex && Object.hasOwn(existing.codex, "plan_reporting")) {
      if (typeof existing.codex.plan_reporting !== "boolean")
        throw Error("invalid existing plan-reporting opt-in");
      config.codex.plan_reporting = existing.codex.plan_reporting;
    }
    if (!isDeepStrictEqual(existing, config))
      throw Error("existing config differs");
  }
  for (const [name, value] of [
    ["user.name", settings.author_name],
    ["user.email", settings.author_email],
    ["gpg.format", "ssh"],
    ["user.signingkey", key],
    ["commit.gpgsign", "true"],
    ["tag.gpgsign", "true"],
  ]) {
    execFileSync("git", ["-C", root, "config", "--local", name, value], {
      stdio: ["ignore", "ignore", "ignore"],
    });
  }
  replace(p.config, JSON.stringify(config, null, 2) + "\n");
}
