import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { configParent, directory, file } from "./private-files.mjs";
import { secretValue } from "./secret-value.mjs";

export function refreshGithub(home, envFile, runGh = spawnSync) {
  const value = secretValue(file(envFile), "GH_TOKEN");
  if (value !== undefined && !/^github_pat_[A-Za-z0-9_]{1,244}$/.test(value))
    throw Error("token");
  const configDir = path.join(configParent(home), "gh");
  directory(configDir);
  const checkFiles = () => {
    for (const name of ["config.yml", "hosts.yml"]) {
      const target = path.join(configDir, name);
      if (fs.lstatSync(target, { throwIfNoEntry: false })) file(target);
    }
  };
  checkFiles();
  const deadline = Date.now() + 20_000;
  const run = (args, input = "") => {
    const timeout = deadline - Date.now();
    if (timeout <= 0) throw Error("deadline");
    const result = runGh("/usr/bin/gh", args, {
      input,
      encoding: "utf8",
      timeout,
      killSignal: "SIGKILL",
      maxBuffer: 16_384,
      env: {
        HOME: home,
        PATH: "/usr/bin:/bin",
        GH_CONFIG_DIR: configDir,
        GH_PROMPT_DISABLED: "1",
        GIT_TERMINAL_PROMPT: "0",
      },
    });
    if (result.error || result.signal) throw Error("gh");
    return result;
  };
  if (value !== undefined) {
    const result = run(
      [
        "auth",
        "login",
        "--hostname",
        "github.com",
        "--with-token",
        "--insecure-storage",
      ],
      value + "\n",
    );
    if (result.status !== 0) throw Error("gh");
  } else {
    // Offline lookup avoids network-dependent auth status when removing access.
    const seen = new Set();
    while (true) {
      const account = run(["config", "get", "user", "--host", "github.com"]);
      if (account.status !== 0 || !account.stdout.trim()) {
        const empty = run(["auth", "logout", "--hostname", "github.com"]);
        if (empty.status === 0) continue;
        if (
          empty.status !== 1 ||
          ![
            "not logged in to any hosts",
            "not logged in to github.com",
          ].includes(empty.stderr.trim())
        )
          throw Error("gh");
        break;
      }
      const user = account.stdout.trim();
      if (
        !/^[A-Za-z0-9][A-Za-z0-9-]{0,38}(?:\[bot\])?$/.test(user) ||
        seen.has(user) ||
        seen.size >= 16
      )
        throw Error("account");
      seen.add(user);
      if (
        run(["auth", "logout", "--hostname", "github.com", "--user", user])
          .status !== 0
      )
        throw Error("gh");
    }
  }
  checkFiles();
}
