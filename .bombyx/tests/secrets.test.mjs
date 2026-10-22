import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFileSync, spawnSync } from "node:child_process";
import { test } from "node:test";
import { configure } from "../configure.mjs";
import { refresh } from "../refresh.mjs";
const root = path.resolve(import.meta.dirname, "../..");
const node = process.execPath;
const token = "a".repeat(64);
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
  const runtime = path.join(
    f.home,
    ".local/share/silkboard-tools/node-v24.14.1/bin",
  );
  fs.mkdirSync(runtime, { recursive: true });
  fs.symlinkSync(node, path.join(runtime, "node"));
  f.put(`SILKBOARD_PUBLISH_TOKEN=${token}\n`);
  for (let i = 0; i < 2; i++) {
    const out = spawnSync(
      "/bin/bash",
      [path.join(root, ".bombyx/refresh-secrets.sh")],
      {
        env: { ...f.env, PATH: "/usr/local/bin:/usr/bin:/bin" },
        encoding: "utf8",
        input: "",
      },
    );
    assert.equal(out.status, 0);
    assert.equal(out.stdout + out.stderr, "");
  }
  f.put("SILKBOARD_PUBLISH_TOKEN=private-invalid-input\n");
  const out = spawnSync(node, [path.join(root, ".bombyx/refresh-cli.mjs")], {
    env: f.env,
    encoding: "utf8",
  });
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
  script(runtime, "npm", "exit 0");
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
  script(
    fake,
    "curl",
    '[[ "$*" == *https://pkgs.tailscale.com/stable/ubuntu/noble.noarmor.gpg* || "$*" == *https://cli.github.com/packages/githubcli-archive-keyring.gpg* ]] || exit 70; printf public-key-fixture > "${!#}"',
  );
  script(fake, "sudo", `printf '%s\\n' "$*" >> "$HOME/system-installer.calls"`);
  script(agents, "codex", "echo 'codex-cli 0.162.1'");
  script(agents, "claude", "echo '2.1.293 (Claude Code)'");
  f.put(`SILKBOARD_PUBLISH_TOKEN=${token}\n`);
  execFileSync("git", [
    "-C",
    f.repo,
    "config",
    "core.sshCommand",
    "ssh -i retained-deploy-key",
  ]);
  const env = { ...f.env, PATH: fake + ":" + process.env.PATH };
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
    if (i === 0) fs.writeFileSync(counter, "retained");
    else assert.equal(fs.readFileSync(counter, "utf8"), "retained");
  }
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
  const login = spawnSync("code-dupes", ["--version"], {
    env: {
      HOME: f.home,
      PATH: path.join(f.home, ".local/bin") + ":/usr/bin:/bin",
    },
    encoding: "utf8",
  });
  assert.equal(login.status, 0);
  assert.equal(login.stdout.trim(), "code-dupes 0.2.1");
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
