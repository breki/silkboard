import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { test } from "node:test";

const helper = path.resolve(import.meta.dirname, "../install-codex.sh");
function fixture(t) {
  const home = fs.mkdtempSync(path.join(os.tmpdir(), "silkboard-codex-"));
  t.after(() => fs.rmSync(home, { recursive: true, force: true }));
  const bin = path.join(home, ".local/bin");
  const mock = path.join(home, "mock-bin");
  const agents = path.join(home, ".local/share/silkboard-tools/agents");
  const runtime = path.join(home, ".local/share/silkboard-tools/node-v24.14.1");
  const current = path.join(home, ".codex/packages/standalone/current");
  for (const dir of [bin, mock, agents + "/bin", runtime + "/bin"])
    fs.mkdirSync(dir, { recursive: true });
  const script = (file, body) =>
    fs.writeFileSync(file, "#!/bin/sh\nset -eu\n" + body + "\n", {
      mode: 0o755,
    });
  script(
    mock + "/curl",
    `for arg do dest="$arg"; done
printf 'download\\n' >> "$HOME/downloads"
cp "$HOME/installer" "$dest"`,
  );
  script(
    home + "/installer",
    `test "$CODEX_NON_INTERACTIVE" = 1
test "$CODEX_INSTALL_DIR" = "$HOME/.local/bin"
test "$CODEX_HOME" = "$HOME/.codex"
test "$CODEX_RELEASE" = latest
test "\${CODEX_INSTALL_DAEMON_ONLY:-0}" = 0
test ! -e "$HOME/install-fail"
mkdir -p "$CODEX_HOME/packages/standalone/current/bin"
printf '#!/bin/sh\\necho "codex-cli 9.0.0"\\n' > "$CODEX_HOME/packages/standalone/current/bin/codex"
chmod +x "$CODEX_HOME/packages/standalone/current/bin/codex"
ln -sf "$CODEX_HOME/packages/standalone/current/bin/codex" "$CODEX_INSTALL_DIR/codex"`,
  );
  // Any attempt to use npm, including during migration, fails this fixture.
  script(mock + "/npm", 'touch "$HOME/npm-called"; exit 70');
  const env = {
    HOME: home,
    PATH: runtime + "/bin:" + bin + ":" + mock + ":/usr/bin:/bin",
    BOMBYX_PROJECT: "silkboard",
  };
  const run = (extra = {}) =>
    spawnSync("/bin/bash", [helper], {
      env: { ...env, ...extra },
      encoding: "utf8",
      timeout: 10000,
    });
  const legacy = (prefix) => {
    const entry = prefix + "/lib/node_modules/@openai/codex/bin/codex.js";
    fs.mkdirSync(path.dirname(entry), { recursive: true });
    script(entry, 'echo "codex-cli 0.162.1"');
    fs.symlinkSync(
      "../lib/node_modules/@openai/codex/bin/codex.js",
      prefix + "/bin/codex",
    );
  };
  return {
    home,
    bin,
    mock,
    agents,
    runtime,
    current,
    script,
    env,
    run,
    legacy,
  };
}

test("standalone install needs no npm and reprovisioning preserves an upgraded version", (t) => {
  const f = fixture(t);
  let out = f.run({ CODEX_INSTALL_DAEMON_ONLY: "1", CODEX_RELEASE: "0.162.1" });
  assert.equal(out.status, 0, out.stderr);
  assert.equal(fs.existsSync(f.home + "/npm-called"), false);
  assert.equal(fs.readlinkSync(f.bin + "/codex"), f.current + "/bin/codex");
  f.script(f.current + "/bin/codex", 'echo "codex-cli 10.0.0"');
  out = f.run();
  assert.equal(out.status, 0, out.stderr);
  assert.equal(fs.readFileSync(f.home + "/downloads", "utf8"), "download\n");
  assert.match(out.stdout, /10\.0\.0/);
});

test("migration retires both known npm launchers, keeps private state and agrees with PATH", (t) => {
  const f = fixture(t);
  f.legacy(f.agents);
  f.legacy(f.runtime);
  fs.symlinkSync(f.agents + "/bin/codex", f.bin + "/codex");
  fs.mkdirSync(f.home + "/.codex/sessions", { recursive: true });
  for (const file of ["auth.json", "config.toml", "sessions/retained"])
    fs.writeFileSync(f.home + "/.codex/" + file, "private-retained");
  const out = f.run();
  assert.equal(out.status, 0, out.stderr);
  for (const prefix of [f.agents, f.runtime]) {
    assert.equal(fs.existsSync(prefix + "/bin/codex"), false);
    assert.equal(
      fs.existsSync(prefix + "/lib/node_modules/@openai/codex/bin/codex.js"),
      true,
    );
  }
  for (const file of ["auth.json", "config.toml", "sessions/retained"])
    assert.equal(
      fs.readFileSync(f.home + "/.codex/" + file, "utf8"),
      "private-retained",
    );
  const resolved = spawnSync(
    "/bin/sh",
    ["-c", "command -v codex; codex --version"],
    { env: f.env, encoding: "utf8" },
  );
  assert.equal(resolved.status, 0);
  assert.equal(resolved.stdout, f.bin + "/codex\ncodex-cli 9.0.0\n");
  assert.equal(fs.existsSync(f.home + "/npm-called"), false);
});

test("failed installer leaves the old launcher usable for retry", (t) => {
  const f = fixture(t);
  f.legacy(f.agents);
  fs.symlinkSync(f.agents + "/bin/codex", f.bin + "/codex");
  fs.writeFileSync(f.home + "/install-fail", "");
  assert.notEqual(f.run().status, 0);
  assert.equal(fs.readlinkSync(f.bin + "/codex"), f.agents + "/bin/codex");
  assert.equal(fs.existsSync(f.agents + "/bin/codex"), true);
  fs.unlinkSync(f.home + "/install-fail");
  const out = f.run();
  assert.equal(out.status, 0, out.stderr);
});

test("unrelated command files, directories and symlinks are refused before downloading", (t) => {
  for (const kind of [
    "file",
    "directory",
    "symlink",
    "runtime-file",
    "path-file",
  ]) {
    const f = fixture(t);
    const dest =
      kind === "runtime-file"
        ? f.runtime + "/bin/codex"
        : kind === "path-file"
          ? f.mock + "/codex"
          : f.bin + "/codex";
    if (kind === "directory") fs.mkdirSync(dest);
    else if (kind === "symlink") fs.symlinkSync(f.home + "/missing", dest);
    else f.script(dest, "echo unrelated");
    const before = fs.lstatSync(dest);
    assert.notEqual(f.run().status, 0, kind);
    assert.equal(fs.lstatSync(dest).ino, before.ino);
    assert.equal(fs.existsSync(f.home + "/downloads"), false);
  }
});

test("broken standalone selections and custom homes require inspection", (t) => {
  const f = fixture(t);
  fs.symlinkSync(f.current + "/bin/codex", f.bin + "/codex");
  const out = f.run();
  assert.notEqual(out.status, 0);
  assert.match(out.stderr, /standalone launcher is broken/);
  assert.equal(fs.existsSync(f.home + "/downloads"), false);
  fs.unlinkSync(f.bin + "/codex");
  assert.notEqual(f.run({ CODEX_HOME: f.home + "/custom-codex" }).status, 0);
  assert.equal(fs.existsSync(f.home + "/downloads"), false);
});

test("successful installer exit alone cannot retire npm launchers", (t) => {
  const f = fixture(t);
  f.legacy(f.agents);
  fs.symlinkSync(f.agents + "/bin/codex", f.bin + "/codex");
  f.script(f.home + "/installer", "exit 0");
  assert.notEqual(f.run().status, 0);
  assert.equal(fs.existsSync(f.agents + "/bin/codex"), true);
  fs.unlinkSync(f.bin + "/codex");
  fs.mkdirSync(f.current + "/bin", { recursive: true });
  f.script(f.current + "/bin/codex", "echo unrelated");
  fs.symlinkSync(f.current + "/bin/codex", f.bin + "/codex");
  assert.notEqual(f.run().status, 0);
  assert.equal(fs.existsSync(f.agents + "/bin/codex"), true);
});
