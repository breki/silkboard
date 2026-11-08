import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFileSync, spawnSync } from "node:child_process";
import { test } from "node:test";
import { createHash } from "node:crypto";
import { configure } from "../configure.mjs";
import { refresh } from "../refresh.mjs";
import { refreshGithub } from "../github-auth.mjs";
import { refreshSecrets } from "../refresh-secrets.mjs";
const root = path.resolve(import.meta.dirname, "../..");
const node = process.execPath;
const token = "a".repeat(64);
const githubToken = "github_pat_" + "c".repeat(64);
const endpoint = "https://dashboard.example.test";
function fixture(t) {
  const home = fs.mkdtempSync(path.join(os.tmpdir(), "silkboard-bombyx-"));
  t.after(() => fs.rmSync(home, { recursive: true, force: true }));
  fs.chmodSync(home, 0o700);
  const repo = path.join(home, "silkboard");
  fs.mkdirSync(repo);
  execFileSync("git", ["init", "-q", repo]);
  fs.mkdirSync(path.join(home, ".ssh"), { mode: 0o700 });
  execFileSync("ssh-keygen", [
    "-q",
    "-t",
    "ed25519",
    "-N",
    "",
    "-f",
    path.join(home, ".ssh/bombyx-deploy-key"),
  ]);
  const envFile = path.join(home, ".bombyx-env");
  const env = {
    HOME: home,
    PATH: process.env.PATH,
    BOMBYX_ENV_FILE: envFile,
    BOMBYX_PROJECT: "silkboard",
    SILKBOARD_PUBLISH_ENDPOINT: endpoint,
  };
  const run = (file, args = []) => {
    try {
      if (file === "configure.mjs") configure(home, args[0], endpoint);
      else refresh(home, envFile);
      return { status: 0, stdout: "", stderr: "" };
    } catch (_) {
      return { status: 1, stdout: "", stderr: "" };
    }
  };
  const put = (value) => fs.writeFileSync(envFile, value, { mode: 0o600 });
  const configPath = path.join(home, ".config/silkboard-publish/config.json");
  const tokenPath = path.join(
    home,
    ".config/silkboard-publish/publisher.token",
  );
  return { home, repo, envFile, env, run, put, configPath, tokenPath };
}
function hookFixture(f) {
  fs.cpSync(path.join(root, ".bombyx"), path.join(f.repo, ".bombyx"), {
    recursive: true,
  });
  const runtime = path.join(
    f.home,
    ".local/share/silkboard-tools/node-v24.14.1/bin",
  );
  fs.mkdirSync(runtime, { recursive: true });
  if (!fs.existsSync(path.join(runtime, "node")))
    fs.symlinkSync(node, path.join(runtime, "node"));
  const fakeGh = path.join(f.home, "fake-gh");
  fs.writeFileSync(
    fakeGh,
    `#!${node}
const fs = require('node:fs');
const home = process.env.HOME;
const args = process.argv.slice(2);
const input = fs.readFileSync(0, 'utf8');
fs.appendFileSync(home + '/gh-calls.jsonl', JSON.stringify({args,input,env:process.env}) + '\\n');
if (fs.existsSync(home + '/gh-stall')) setInterval(() => {}, 1000);
else if (fs.existsSync(home + '/gh-fail')) { console.error(input); console.log(input); process.exit(1); }
else if (args[1] === 'login') fs.writeFileSync(home + '/.config/gh/hosts.yml', input, {mode:0o600});
else if (args[0] === 'config') {
  if (!fs.existsSync(home + '/.config/gh/hosts.yml')) process.exit(1);
  console.log(fs.existsSync(home + '/gh-users.json') ? JSON.parse(fs.readFileSync(home + '/gh-users.json'))[0] : 'fixture-user');
}
else if (args[1] === 'logout') {
  const file = home + '/.config/gh/hosts.yml';
  if (!fs.existsSync(file)) { console.error('not logged in to any hosts'); process.exit(1); }
  const usersFile = home + '/gh-users.json';
  if (fs.existsSync(usersFile)) {
    const users = JSON.parse(fs.readFileSync(usersFile));
    if (args.at(-1) !== users.shift()) process.exit(1);
    if (users.length) fs.writeFileSync(usersFile, JSON.stringify(users));
    else { fs.unlinkSync(usersFile); fs.unlinkSync(file); }
  } else fs.unlinkSync(file);
} else process.exit(70);
`,
    { mode: 0o755 },
  );
  const auth = path.join(f.repo, ".bombyx/github-auth.mjs");
  if (fs.existsSync(auth))
    fs.writeFileSync(
      auth,
      fs
        .readFileSync(auth, "utf8")
        .replace('"/usr/bin/gh"', JSON.stringify(fakeGh)),
    );
  const run = (extraEnv = {}) =>
    spawnSync("/bin/bash", [path.join(f.repo, ".bombyx/refresh-secrets.sh")], {
      env: {
        HOME: f.home,
        PATH: "/usr/local/bin:/usr/bin:/bin",
        BOMBYX_PROJECT: "silkboard",
        BOMBYX_ENV_FILE: f.envFile,
        ...extraEnv,
      },
      encoding: "utf8",
      input: "",
      timeout: 45000,
    });
  const calls = () =>
    fs.existsSync(path.join(f.home, "gh-calls.jsonl"))
      ? fs
          .readFileSync(path.join(f.home, "gh-calls.jsonl"), "utf8")
          .trim()
          .split("\n")
          .map(JSON.parse)
      : [];
  return { run, calls, fakeGh };
}
test("refresh orchestration keeps fixed GitHub arguments, private storage and an isolated child environment", (t) => {
  const f = fixture(t);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 0);
  const hook = hookFixture(f);
  const runner = (command, args, options) => {
    assert.equal(command, "/usr/bin/gh");
    assert.ok(options.timeout > 0 && options.timeout <= 20000);
    assert.equal(options.killSignal, "SIGKILL");
    assert.deepEqual(Object.keys(options.env).sort(), [
      "GH_CONFIG_DIR",
      "GH_PROMPT_DISABLED",
      "GIT_TERMINAL_PROMPT",
      "HOME",
      "PATH",
    ]);
    return spawnSync(hook.fakeGh, args, options);
  };
  const run = () =>
    refreshSecrets(f.home, f.envFile, (home, envFile) =>
      refreshGithub(home, envFile, runner),
    );
  for (const publisher of [token, "bad", '"unterminated']) {
    f.put(`SILKBOARD_PUBLISH_TOKEN=${publisher}\nGH_TOKEN=${githubToken}\n`);
    assert.equal(run().length, publisher === token ? 0 : 1);
  }
  f.put(`SILKBOARD_PUBLISH_TOKEN=${token}\n`);
  assert.deepEqual(run(), []);
  assert.deepEqual(run(), []);
  assert.doesNotThrow(() =>
    refreshGithub(f.home, f.envFile, () => ({
      status: 1,
      stderr: "not logged in to github.com\n",
    })),
  );
  let attempts = 0;
  assert.doesNotThrow(() =>
    refreshGithub(f.home, f.envFile, () => {
      attempts++;
      if (attempts === 2) return { status: 0 };
      return { status: 1, stderr: "not logged in to any hosts" };
    }),
  );
  assert.throws(() =>
    refreshGithub(f.home, f.envFile, (_cmd, args) =>
      args[0] === "config"
        ? { status: 0, stdout: "repeated-user\n" }
        : { status: 0 },
    ),
  );
  assert.throws(() =>
    refreshGithub(f.home, f.envFile, () => ({
      status: 0,
      stdout: "--unsafe-user\n",
    })),
  );
  for (const result of [
    { error: Error("unavailable") },
    { signal: "SIGTERM" },
    { status: 1, stderr: "unexpected failure" },
    { status: 2, stderr: "not logged in to any hosts" },
  ]) {
    assert.throws(() => refreshGithub(f.home, f.envFile, () => result));
  }
  f.put(`SILKBOARD_PUBLISH_TOKEN=${token}\nGH_TOKEN=bad\n`);
  assert.equal(run().length, 1);
  f.put(`SILKBOARD_PUBLISH_TOKEN=bad\nGH_TOKEN="unterminated\n`);
  assert.equal(run().length, 2);
  f.put(`SILKBOARD_PUBLISH_TOKEN=${token}\nGH_TOKEN=${githubToken}\n`);
  const config = path.join(f.home, ".config/gh/config.yml");
  fs.writeFileSync(config, "version: 1\n", { mode: 0o600 });
  assert.deepEqual(run(), []);
  fs.chmodSync(config, 0o644);
  assert.equal(run().length, 1);
});
test("bare refresh logs GitHub in using stdin only, reruns and rotates independently", (t) => {
  const f = fixture(t);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 0);
  const hook = hookFixture(f);
  execFileSync("git", [
    "-C",
    f.repo,
    "config",
    "core.sshCommand",
    "ssh -i retained-deploy-key",
  ]);
  for (const value of [
    githubToken,
    githubToken,
    "github_pat_" + "d".repeat(64),
  ]) {
    f.put(
      `SILKBOARD_PUBLISH_TOKEN=${token}\nexport GH_TOKEN = '${value}' ignored trailing text\r\n`,
    );
    const result = hook.run({
      GH_TOKEN: "ambient-credential",
      GITHUB_TOKEN: "other-ambient-credential",
      GH_DEBUG: "api",
    });
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stdout + result.stderr, "");
    assert.equal(fs.readFileSync(f.tokenPath, "utf8"), token + "\n");
    const calls = hook.calls();
    assert.ok(calls.length > 0, "refresh must invoke GitHub CLI");
    const call = calls.at(-1);
    assert.deepEqual(call.args, [
      "auth",
      "login",
      "--hostname",
      "github.com",
      "--with-token",
      "--insecure-storage",
    ]);
    assert.equal(call.input, value + "\n");
    assert.equal(JSON.stringify(call.args).includes(value), false);
    assert.equal(JSON.stringify(call.env).includes(value), false);
    assert.equal(call.env.GH_TOKEN, undefined);
    assert.equal(call.env.GITHUB_TOKEN, undefined);
    assert.equal(call.env.GH_DEBUG, undefined);
  }
  assert.equal(
    execFileSync("git", ["-C", f.repo, "config", "core.sshCommand"], {
      encoding: "utf8",
    }).trim(),
    "ssh -i retained-deploy-key",
  );
});
test("removal clears multiple cached GitHub accounts without relying on network status", (t) => {
  const f = fixture(t);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 0);
  const hook = hookFixture(f);
  f.put(`SILKBOARD_PUBLISH_TOKEN=${token}\nGH_TOKEN=${githubToken}\n`);
  assert.equal(hook.run().status, 0);
  fs.writeFileSync(
    path.join(f.home, "gh-users.json"),
    JSON.stringify(["first-user", "second-user"]),
  );
  f.put(`SILKBOARD_PUBLISH_TOKEN=${token}\n`);
  const result = hook.run();
  assert.equal(result.status, 0, result.stderr);
  assert.equal(fs.existsSync(path.join(f.home, ".config/gh/hosts.yml")), false);
  const removals = hook.calls().filter((call) => call.args.includes("--user"));
  assert.deepEqual(
    removals.map((call) => call.args.at(-1)),
    ["first-user", "second-user"],
  );
  assert.equal(
    hook.calls().some((call) => call.args.includes("status")),
    false,
  );
});
test("removing the GitHub token logs out locally and repeated absence succeeds", (t) => {
  const f = fixture(t);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 0);
  const hook = hookFixture(f);
  f.put(`SILKBOARD_PUBLISH_TOKEN=${token}\nGH_TOKEN=${githubToken}\n`);
  assert.equal(hook.run().status, 0);
  f.put(`SILKBOARD_PUBLISH_TOKEN=${token}\n`);
  for (let i = 0; i < 2; i++) {
    const result = hook.run();
    assert.equal(result.status, 0, result.stderr);
    assert.equal(result.stdout + result.stderr, "");
    assert.equal(
      fs.existsSync(path.join(f.home, ".config/gh/hosts.yml")),
      false,
    );
    assert.equal(fs.readFileSync(f.tokenPath, "utf8"), token + "\n");
    assert.deepEqual(hook.calls().at(-1).args, [
      "auth",
      "logout",
      "--hostname",
      "github.com",
    ]);
  }
});
test("malformed selected GitHub input cannot skip or remove a valid publisher token", (t) => {
  const f = fixture(t);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 0);
  const hook = hookFixture(f);
  for (const value of [
    "classic-token",
    "github_pat_",
    "github_pat_x$(touch OWNED)",
    '"unterminated',
    "",
    "valid\nGH_TOKEN=",
  ]) {
    f.put(`SILKBOARD_PUBLISH_TOKEN=${token}\nGH_TOKEN=${value}\n`);
    const result = hook.run();
    assert.notEqual(result.status, 0);
    assert.equal(result.stdout, "");
    assert.match(result.stderr, /GitHub CLI secret refresh failed/);
    assert.doesNotMatch(result.stderr, /publisher secret refresh failed/);
    assert.equal(result.stderr.includes(value.trim() || token), false);
    assert.equal(fs.readFileSync(f.tokenPath, "utf8"), token + "\n");
  }
  assert.equal(hook.calls().length, 0);
  assert.equal(fs.existsSync(path.join(f.repo, "OWNED")), false);
});
test("GitHub failure is opaque and independent from publisher success; both failures are reported", (t) => {
  const f = fixture(t);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 0);
  const hook = hookFixture(f);
  fs.writeFileSync(path.join(f.home, "gh-fail"), "");
  for (const publisher of [token, "bad-publisher-token"]) {
    f.put(`SILKBOARD_PUBLISH_TOKEN=${publisher}\nGH_TOKEN=${githubToken}\n`);
    const result = hook.run();
    assert.notEqual(result.status, 0);
    assert.equal(result.stdout, "");
    assert.match(result.stderr, /GitHub CLI secret refresh failed/);
    assert.equal(result.stderr.includes(githubToken), false);
    assert.equal(result.stderr.includes(publisher), false);
    assert.equal(fs.existsSync(f.tokenPath), publisher === token);
    assert.equal(
      /publisher secret refresh failed/.test(result.stderr),
      publisher !== token,
    );
  }
  assert.equal(hook.calls().length, 2);
});
test("invalid publisher tokens and paths do not block valid GitHub login", (t) => {
  const f = fixture(t);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 0);
  const hook = hookFixture(f);
  for (const publisher of ["not-valid", '"unterminated']) {
    f.put(`SILKBOARD_PUBLISH_TOKEN=${publisher}\nGH_TOKEN=${githubToken}\n`);
    const result = hook.run();
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /publisher secret refresh failed/);
    assert.doesNotMatch(result.stderr, /GitHub CLI secret refresh failed/);
    assert.equal(hook.calls().at(-1).input, githubToken + "\n");
    assert.equal(fs.existsSync(f.tokenPath), false);
  }
  fs.chmodSync(path.dirname(f.configPath), 0o755);
  f.put(`SILKBOARD_PUBLISH_TOKEN=${token}\nGH_TOKEN=${githubToken}\n`);
  const result = hook.run();
  assert.notEqual(result.status, 0);
  assert.doesNotMatch(result.stderr, /GitHub CLI secret refresh failed/);
  assert.equal(hook.calls().length, 3);
});
test("GitHub storage paths are private and unsafe paths do not affect publisher refresh", (t) => {
  const f = fixture(t);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 0);
  const hook = hookFixture(f);
  f.put(`SILKBOARD_PUBLISH_TOKEN=${token}\nGH_TOKEN=${githubToken}\n`);
  assert.equal(hook.run().status, 0);
  const dir = path.join(f.home, ".config/gh");
  const hosts = path.join(dir, "hosts.yml");
  assert.equal(fs.statSync(dir).mode & 0o777, 0o700);
  assert.equal(fs.statSync(hosts).mode & 0o777, 0o600);
  fs.renameSync(hosts, hosts + ".saved");
  fs.symlinkSync(hosts + ".saved", hosts);
  const result = hook.run();
  assert.notEqual(result.status, 0);
  assert.equal(hook.calls().length, 1);
  assert.equal(fs.readFileSync(hosts + ".saved", "utf8"), githubToken + "\n");
  assert.equal(fs.readFileSync(f.tokenPath, "utf8"), token + "\n");
  fs.unlinkSync(hosts);
  fs.renameSync(hosts + ".saved", hosts);
  fs.chmodSync(dir, 0o755);
  assert.notEqual(hook.run().status, 0);
  assert.equal(hook.calls().length, 1);
});
test("a stalled GitHub command is killed well inside the sixty-second hook budget", (t) => {
  const f = fixture(t);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 0);
  const hook = hookFixture(f);
  f.put(`SILKBOARD_PUBLISH_TOKEN=${token}\nGH_TOKEN=${githubToken}\n`);
  fs.writeFileSync(path.join(f.home, "gh-stall"), "");
  const start = Date.now();
  const result = hook.run();
  assert.notEqual(result.status, 0);
  assert.equal(result.error, undefined);
  assert.ok(Date.now() - start < 35000);
  assert.match(result.stderr, /GitHub CLI secret refresh failed/);
  assert.equal(result.stderr.includes(githubToken), false);
  assert.equal(fs.readFileSync(f.tokenPath, "utf8"), token + "\n");
});
test("configure is rerunnable, signs only the clone, and refresh rotates privately", (t) => {
  const f = fixture(t);
  for (let i = 0; i < 2; i++)
    assert.equal(f.run("configure.mjs", [f.repo]).status, 0);
  const config = JSON.parse(fs.readFileSync(f.configPath));
  assert.equal(config.codex.workspace, f.repo);
  assert.equal(config.endpoint, endpoint);
  assert.equal(
    execFileSync("git", ["-C", f.repo, "config", "--local", "user.email"], {
      encoding: "utf8",
    }).trim(),
    "silkboard-vm@agents.invalid",
  );
  assert.equal(
    execFileSync("git", ["-C", f.repo, "config", "--local", "tag.gpgsign"], {
      encoding: "utf8",
    }).trim(),
    "true",
  );
  f.put(`SILKBOARD_PUBLISH_TOKEN=${token}\n`);
  for (let i = 0; i < 2; i++) {
    const out = f.run("refresh.mjs");
    assert.equal(out.status, 0);
    assert.equal(out.stdout + out.stderr, "");
    assert.equal(fs.readFileSync(f.tokenPath, "utf8"), token + "\n");
    assert.equal(fs.statSync(f.tokenPath).mode & 0o777, 0o600);
  }
  f.put(
    `OTHER=unused\r\nexport SILKBOARD_PUBLISH_TOKEN = '${"b".repeat(64)}' # rotated\r\n`,
  );
  assert.equal(f.run("refresh.mjs").status, 0);
  assert.equal(fs.readFileSync(f.tokenPath, "utf8"), "b".repeat(64) + "\n");
});
test("malformed or removed secret disables stale token without printing input", (t) => {
  const f = fixture(t);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 0);
  for (const value of [
    "",
    "OTHER=value\n",
    "SILKBOARD_PUBLISH_TOKEN=$(touch OWNED)\n",
    `SILKBOARD_PUBLISH_TOKEN=${token}extra\n`,
  ]) {
    f.put(`SILKBOARD_PUBLISH_TOKEN=${token}\n`);
    assert.equal(f.run("refresh.mjs").status, 0);
    f.put(value);
    const out = f.run("refresh.mjs");
    assert.notEqual(out.status, 0);
    assert.equal(fs.existsSync(f.tokenPath), false);
    assert.equal(out.stdout, "");
    assert.equal(out.stderr.includes(token), false);
    assert.equal(fs.existsSync(path.join(root, "OWNED")), false);
  }
});
test("refuses token symlinks and public env files", (t) => {
  const f = fixture(t);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 0);
  const outside = path.join(f.home, "untouched");
  fs.writeFileSync(outside, "keep", { mode: 0o600 });
  f.put(`SILKBOARD_PUBLISH_TOKEN=${token}\n`);
  fs.symlinkSync(outside, f.tokenPath);
  assert.notEqual(f.run("refresh.mjs").status, 0);
  assert.equal(fs.readFileSync(outside, "utf8"), "keep");
  fs.unlinkSync(f.tokenPath);
  fs.chmodSync(f.envFile, 0o644);
  assert.notEqual(f.run("refresh.mjs").status, 0);
  assert.equal(fs.existsSync(f.tokenPath), false);
});
test("configuration preserves existing choices and rejects unsafe key/config paths", (t) => {
  const f = fixture(t);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 0);
  const original = fs.readFileSync(f.configPath, "utf8");
  fs.writeFileSync(
    f.configPath,
    original.replace("dashboard.example.test", "elsewhere.invalid"),
  );
  assert.notEqual(f.run("configure.mjs", [f.repo]).status, 0);
  assert.equal(
    fs.readFileSync(f.configPath, "utf8").includes("elsewhere.invalid"),
    true,
  );
  fs.writeFileSync(f.configPath, original);
  fs.chmodSync(f.configPath, 0o644);
  assert.notEqual(f.run("configure.mjs", [f.repo]).status, 0);
});
test("refresh shell hook works twice in the exact bare environment and keeps failures opaque", (t) => {
  const f = fixture(t);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 0);
  const hook = hookFixture(f);
  f.put(`SILKBOARD_PUBLISH_TOKEN=${token}\n`);
  for (let i = 0; i < 2; i++) {
    const out = hook.run();
    assert.equal(out.status, 0);
    assert.equal(out.stdout + out.stderr, "");
  }
  f.put("SILKBOARD_PUBLISH_TOKEN=private-invalid-input\n");
  const out = hook.run();
  assert.notEqual(out.status, 0);
  assert.equal(out.stdout, "");
  assert.equal(out.stderr.includes("private-invalid-input"), false);
  assert.equal(fs.existsSync(f.tokenPath), false);
});
test("unsafe homes, missing keys and foreign checkouts cannot configure signing", (t) => {
  const f = fixture(t);
  assert.equal(f.run("configure.mjs", [f.home]).status, 1);
  const key = path.join(f.home, ".ssh/bombyx-deploy-key");
  fs.renameSync(key, key + ".saved");
  fs.symlinkSync(key + ".saved", key);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 1);
  fs.unlinkSync(key);
  fs.renameSync(key + ".saved", key);
  fs.chmodSync(f.home, 0o777);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 1);
  fs.chmodSync(f.home, 0o700);
});
test("rejects malformed quotes, nonprivate directories and oversized source without retaining a stale token", (t) => {
  const f = fixture(t);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 0);
  for (const value of [
    `SILKBOARD_PUBLISH_TOKEN="${token}`,
    `SILKBOARD_PUBLISH_TOKEN="${token}" # ok\n`,
    `SILKBOARD_PUBLISH_TOKEN=${token}\nSILKBOARD_PUBLISH_TOKEN=\n`,
    "x".repeat(65537),
  ]) {
    f.put(value);
    const out = f.run("refresh.mjs");
    assert.equal(out.status, value.endsWith("# ok\n") ? 0 : 1);
  }
  const dir = path.dirname(f.tokenPath);
  fs.chmodSync(dir, 0o755);
  assert.equal(f.run("refresh.mjs").status, 1);
  fs.chmodSync(dir, 0o700);
  const configParent = path.dirname(dir);
  fs.chmodSync(configParent, 0o777);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 1);
  fs.chmodSync(configParent, 0o700);
});
test("refuses dangling config paths and token hardlinks without modifying their other names", (t) => {
  const f = fixture(t);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 0);
  fs.unlinkSync(f.configPath);
  fs.symlinkSync(path.join(f.home, "missing"), f.configPath);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 1);
  fs.unlinkSync(f.configPath);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 0);
  f.put(`SILKBOARD_PUBLISH_TOKEN=${token}\n`);
  assert.equal(f.run("refresh.mjs").status, 0);
  const other = path.join(f.home, "hardlink");
  fs.linkSync(f.tokenPath, other);
  assert.equal(f.run("refresh.mjs").status, 1);
  assert.equal(fs.readFileSync(other, "utf8"), token + "\n");
});
test("native provisioner reruns with mocked system installers, preserving counters and clone SSH transport", (t) => {
  const f = fixture(t);
  fs.mkdirSync(path.join(f.repo, "frontend"));
  fs.cpSync(path.join(root, ".bombyx"), path.join(f.repo, ".bombyx"), {
    recursive: true,
  });
  const runtime = path.join(
    f.home,
    ".local/share/silkboard-tools/node-v24.14.1/bin",
  );
  const cargoBin = path.join(f.home, ".cargo/bin");
  const agents = path.join(f.home, ".local/share/silkboard-tools/agents/bin");
  const fake = path.join(f.home, "mock-bin");
  for (const dir of [runtime, cargoBin, agents, fake])
    fs.mkdirSync(dir, { recursive: true, mode: 0o700 });
  const script = (dir, name, body) =>
    fs.writeFileSync(
      path.join(dir, name),
      "#!/usr/bin/env bash\nset -eu\n" + body + "\n",
      { mode: 0o755 },
    );
  fs.symlinkSync(node, path.join(runtime, "node"));
  script(
    runtime,
    "npm",
    `printf '%s\\n' "$*" >> "$HOME/npm.calls"
if [[ $1 == install ]]; then
    [[ $* == "install --global --prefix $HOME/.local/share/silkboard-tools/agents @anthropic-ai/claude-code@2.1.293" ]]
    printf '#!/bin/sh\\necho "2.1.293 (Claude Code)"\\n' > "$HOME/.local/share/silkboard-tools/agents/bin/claude"
    chmod +x "$HOME/.local/share/silkboard-tools/agents/bin/claude"
fi`,
  );
  script(runtime, "npx", "exit 0");
  script(cargoBin, "rustup", "exit 0");
  script(cargoBin, "rustc", "echo rustc");
  script(cargoBin, "rustfmt", "exit 0");
  script(cargoBin, "cargo-clippy", "exit 0");
  script(
    cargoBin,
    "cargo",
    `case "$1" in
llvm-cov) echo 'cargo-llvm-cov 0.9.0';;
deny) echo 'cargo-deny 0.20.2';;
audit) echo 'cargo-audit-audit 0.22.2';;
build) mkdir -p target/release; printf '#!/bin/sh\\nexit 0\\n' > target/release/silkboard-publish;;
*) exit 70;;
esac`,
  );
  script(cargoBin, "code-dupes", "echo 'code-dupes 0.2.1'");
  for (const command of ["tailscale", "gh"]) script(fake, command, "exit 70");
  const architecture = execFileSync("uname", ["-m"], {
    encoding: "utf8",
  }).trim();
  const packageName = "kozmotic-v2.3.0-" + architecture + "-unknown-linux-gnu";
  const release = path.join(f.home, "release-fixture");
  fs.mkdirSync(path.join(release, packageName), { recursive: true });
  script(path.join(release, packageName), "kozmotic", "echo 'kozmotic 2.3.0'");
  const archive = path.join(release, "package.tar.gz");
  execFileSync("tar", ["-czf", archive, "-C", release, packageName]);
  const digest = (file) =>
    createHash("sha256").update(fs.readFileSync(file)).digest("hex");
  const archiveDigest = digest(archive);
  const binaryDigest = digest(path.join(release, packageName, "kozmotic"));
  // Replace network and published hashes with a locally generated release.
  // Actual checksum verification still runs, including corrupted-download refusal.
  script(
    fake,
    "sha256sum",
    `read -r published file
case "$file" in
*/package.tar.gz) expected=${archiveDigest};;
*/kozmotic) expected=${binaryDigest};;
*) exit 70;;
esac
printf '%s  %s\\n' "$expected" "$file" | /usr/bin/sha256sum --check --status`,
  );
  script(
    fake,
    "curl",
    `case "$*" in
*https://pkgs.tailscale.com/stable/ubuntu/noble.noarmor.gpg*|*https://cli.github.com/packages/githubcli-archive-keyring.gpg*) printf public-key-fixture > "\${!#}";;
*https://github.com/breki/kozmotic/releases/download/v2.3.0/kozmotic-v2.3.0-*)
if [[ \${CREATE_DESTINATION_DIRECTORY:-0} == 1 ]]; then mkdir "$HOME/.claude/bin/kozmotic"; fi
cp "$HOME/release-fixture/package.tar.gz" "\${!#}"; printf "downloaded\\n" >> "$HOME/kozmotic-downloads";;
*) exit 70;;
esac`,
  );
  script(fake, "sudo", `printf '%s\\n' "$*" >> "$HOME/system-installer.calls"`);
  const standalone = path.join(
    f.home,
    ".codex/packages/standalone/current/bin",
  );
  fs.mkdirSync(standalone, { recursive: true });
  fs.mkdirSync(path.join(f.home, ".local/bin"), { recursive: true });
  script(standalone, "codex", "echo 'codex-cli 9.0.0'");
  fs.symlinkSync(
    path.join(standalone, "codex"),
    path.join(f.home, ".local/bin/codex"),
  );
  script(agents, "claude", "echo '2.1.293 (Claude Code)'");
  f.put(`SILKBOARD_PUBLISH_TOKEN=${token}\n`);
  execFileSync("git", [
    "-C",
    f.repo,
    "config",
    "core.sshCommand",
    "ssh -i retained-deploy-key",
  ]);
  const env = { ...f.env, PATH: fake + ":/usr/bin:/bin" };
  hookFixture(f);
  for (let i = 0; i < 2; i++) {
    const out = spawnSync(
      "/bin/bash",
      [path.join(f.repo, ".bombyx/provision.sh")],
      { cwd: f.repo, env, encoding: "utf8", timeout: 15000 },
    );
    assert.equal(out.status, 0, out.stderr);
    assert.equal((out.stdout + out.stderr).includes(token), false);
    const counter = path.join(
      f.home,
      ".local/state/silkboard-publish/retained-counter",
    );
    if (i === 0) {
      fs.writeFileSync(counter, "retained");
      script(standalone, "codex", "echo 'codex-cli 10.0.0'");
      fs.unlinkSync(path.join(agents, "claude"));
    } else {
      assert.equal(fs.readFileSync(counter, "utf8"), "retained");
      assert.match(out.stdout, /codex-cli 10\.0\.0/);
    }
  }
  assert.doesNotMatch(
    fs.readFileSync(path.join(f.home, "npm.calls"), "utf8"),
    /@openai\/codex/,
  );
  assert.equal(
    execFileSync("git", ["-C", f.repo, "config", "core.sshCommand"], {
      encoding: "utf8",
    }).trim(),
    "ssh -i retained-deploy-key",
  );
  const calls = fs.readFileSync(
    path.join(f.home, "system-installer.calls"),
    "utf8",
  );
  assert.match(calls, /apt-get install[^\n]*\bgh\b/);
  assert.match(calls, /apt-get install[^\n]*\btailscale\b/);
  assert.match(calls, /systemctl enable --now tailscaled/);
  assert.doesNotMatch(calls, /tailscale up|gh auth/);
  assert.equal(fs.readFileSync(f.tokenPath, "utf8"), token + "\n");
  assert.equal(
    fs.existsSync(path.join(f.home, ".local/bin/silkboard-publish")),
    true,
  );
  assert.equal(
    fs.existsSync(path.join(f.home, ".claude/bin/kozmotic")),
    true,
    "a fresh VM must install the executable required by its Git-status hook",
  );
  const kozmotic = path.join(f.home, ".claude/bin/kozmotic");
  assert.equal(digest(kozmotic), binaryDigest);
  assert.equal(
    fs.readFileSync(path.join(f.home, "kozmotic-downloads"), "utf8").trim(),
    "downloaded",
  );
  const installHookTool = (extraEnv = {}) =>
    spawnSync("bash", [path.join(f.repo, ".bombyx/install-kozmotic.sh")], {
      env: { ...env, ...extraEnv },
      encoding: "utf8",
    });
  fs.writeFileSync(kozmotic, "unrelated executable");
  assert.notEqual(installHookTool().status, 0);
  assert.equal(fs.readFileSync(kozmotic, "utf8"), "unrelated executable");
  fs.unlinkSync(kozmotic);
  fs.symlinkSync(path.join(release, packageName, "kozmotic"), kozmotic);
  assert.notEqual(installHookTool().status, 0);
  assert.equal(fs.lstatSync(kozmotic).isSymbolicLink(), true);
  fs.unlinkSync(kozmotic);
  assert.notEqual(
    installHookTool({ CREATE_DESTINATION_DIRECTORY: "1" }).status,
    0,
  );
  assert.equal(fs.statSync(kozmotic).isDirectory(), true);
  assert.deepEqual(fs.readdirSync(kozmotic), []);
  fs.rmdirSync(kozmotic);
  fs.appendFileSync(archive, "corrupt download");
  assert.notEqual(installHookTool().status, 0);
  assert.equal(fs.existsSync(kozmotic), false);
  const login = spawnSync("code-dupes", ["--version"], {
    env: {
      HOME: f.home,
      PATH: path.join(f.home, ".local/bin") + ":/usr/bin:/bin",
    },
    encoding: "utf8",
  });
  assert.equal(login.status, 0);
  assert.equal(login.stdout.trim(), "code-dupes 0.2.1");
  // A conflicting local launcher must never run before link_tool rejects it.
  const localNpm = path.join(f.home, ".local/bin/npm");
  fs.unlinkSync(localNpm);
  script(
    path.dirname(localNpm),
    "npm",
    'touch "$HOME/unrelated-npm-ran"; exit 70',
  );
  fs.unlinkSync(path.join(agents, "claude"));
  const conflictingNpm = spawnSync(
    "/bin/bash",
    [path.join(f.repo, ".bombyx/provision.sh")],
    { cwd: f.repo, env, encoding: "utf8", timeout: 15000 },
  );
  assert.notEqual(conflictingNpm.status, 0);
  assert.match(conflictingNpm.stderr, /refusing to replace existing npm/);
  assert.equal(fs.existsSync(path.join(f.home, "unrelated-npm-ran")), false);
});
test("quoted values stop at the closing quote as specified by Bombyx", (t) => {
  const f = fixture(t);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 0);
  f.put(`SILKBOARD_PUBLISH_TOKEN="${token}"ignored-suffix\n`);
  assert.equal(f.run("refresh.mjs").status, 0);
  assert.equal(fs.readFileSync(f.tokenPath, "utf8"), token + "\n");
});

test("endpoint is explicitly supplied and invalid origins cannot change existing configuration", (t) => {
  const f = fixture(t);
  assert.equal(f.run("configure.mjs", [f.repo]).status, 0);
  const original = fs.readFileSync(f.configPath, "utf8");
  for (const value of [
    undefined,
    42,
    {},
    "",
    "not a URL",
    "https://:pass@dashboard.example.test",
    "http://remote.example.test",
    "https://user:pass@dashboard.example.test",
    "https://dashboard.example.test/path",
    "https://DASHBOARD.example.test",
    "https://dashboard.example.test:0",
    "https://dashboard.example.test?query",
    "https://dashboard.example.test#fragment",
  ]) {
    assert.throws(() => configure(f.home, f.repo, value));
    assert.equal(fs.readFileSync(f.configPath, "utf8"), original);
  }
  const out = spawnSync(
    node,
    [path.join(root, ".bombyx/configure-cli.mjs"), f.repo],
    { env: { HOME: f.home, PATH: process.env.PATH }, encoding: "utf8" },
  );
  assert.notEqual(out.status, 0);
  assert.equal(out.stdout, "");
});
