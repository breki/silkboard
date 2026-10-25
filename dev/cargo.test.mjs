import { test } from "node:test";
import assert from "node:assert/strict";
import {
  mkdtempSync,
  mkdirSync,
  chmodSync,
  writeFileSync,
  readFileSync,
  rmSync,
  symlinkSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn, spawnSync } from "node:child_process";

const wrapper = fileURLToPath(new URL("./cargo", import.meta.url));
test("routes isolated build modes and serializes complete Cargo commands", async () => {
  const dir = mkdtempSync(join(tmpdir(), "silkboard-dev-cargo-"));
  try {
    mkdirSync(join(dir, "bin"));
    writeFileSync(
      join(dir, "bin/cargo"),
      `#!/usr/bin/env node
const fs = require('node:fs');
const root = process.env.SILKBOARD_DEV_BUILD_ROOT;
if (process.argv[2] === 'metadata') {
 console.log(JSON.stringify({workspace_root:process.cwd(),workspace_members:['probe'],packages:[{id:'probe',name:'probe'}]})); process.exit(0);
}
if (process.argv[2] === 'clean' && process.env.FAIL_CLEAN) process.exit(9);
try { fs.mkdirSync(root + '/running'); } catch { process.exit(42); }
fs.appendFileSync(root + '/calls', JSON.stringify({target:process.env.CARGO_TARGET_DIR,coverage:process.env.CARGO_LLVM_COV_TARGET_DIR,args:process.argv.slice(2)}) + '\\n');
setTimeout(() => { fs.rmdirSync(root + '/running'); process.exit(process.argv.includes('fail') ? 7 : 0); }, 100);
`,
      { mode: 0o700 },
    );
    const env = {
      ...process.env,
      PATH: join(dir, "bin") + ":" + process.env.PATH,
      SILKBOARD_DEV_BUILD_ROOT: join(dir, "cache"),
    };
    const run = (args, overrides = {}) =>
      new Promise((done, reject) => {
        const p = spawn(wrapper, args, {
          env: { ...env, ...overrides },
          stdio: "pipe",
        });
        p.on("error", reject);
        p.on("exit", done);
      });
    assert.deepEqual(
      await Promise.all([
        run(["test", "some filter"]),
        run(["--arm64", "check"]),
      ]),
      [0, 0],
    );
    const calls = readFileSync(join(dir, "cache/calls"), "utf8")
      .trim()
      .split("\n")
      .map(JSON.parse);
    assert.deepEqual(
      new Set(calls.map((c) => c.target)),
      new Set([join(dir, "cache/normal"), join(dir, "cache/arm64")]),
    );
    assert.ok(calls.every((c) => c.coverage === join(dir, "cache/coverage")));
    assert.ok(calls.some((c) => c.args.join("|") === "test|some filter"));
    assert.equal(await run(["fail"]), 7);
    for (const mode of ["normal", "arm64", "coverage"])
      mkdirSync(join(dir, "cache", mode));
    writeFileSync(join(dir, "cache/workspace"), "/old-workspace\n");
    assert.equal(await run(["check"], { FAIL_CLEAN: "1" }), 9);
    assert.equal(
      readFileSync(join(dir, "cache/workspace"), "utf8"),
      "/old-workspace\n",
    );
    assert.equal(await run(["check"]), 0);
    const cleaned = readFileSync(join(dir, "cache/calls"), "utf8")
      .trim()
      .split("\n")
      .map(JSON.parse);
    assert.ok(
      cleaned.some(
        (c) => c.target === join(dir, "cache/normal") && c.args[0] === "clean",
      ),
    );
    assert.ok(
      cleaned.some(
        (c) =>
          c.target === join(dir, "cache/arm64") &&
          c.args.includes("aarch64-unknown-linux-gnu"),
      ),
    );
    assert.ok(
      cleaned.some((c) => c.args.join("|") === "llvm-cov|clean|--workspace"),
    );

    assert.equal(await run([], { SILKBOARD_DEV_BUILD_ROOT: "relative" }), 1);
    mkdirSync(join(dir, "public"), { mode: 0o755 });
    chmodSync(join(dir, "public"), 0o755);
    assert.equal(
      await run([], { SILKBOARD_DEV_BUILD_ROOT: join(dir, "public") }),
      1,
    );
    symlinkSync(join(dir, "cache"), join(dir, "alias"));
    assert.equal(
      await run([], { SILKBOARD_DEV_BUILD_ROOT: join(dir, "alias") }),
      1,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("real Cargo round trip never runs another worktree's source", async () => {
  const dir = mkdtempSync(join(tmpdir(), "silkboard-dev-freshness-"));
  try {
    for (const name of ["a", "b"]) {
      mkdirSync(join(dir, name, "src"), { recursive: true });
      writeFileSync(
        join(dir, name, "Cargo.toml"),
        '[package]\nname = "silkboard-cache-probe"\nversion = "0.1.0"\nedition = "2024"\n',
      );
      writeFileSync(
        join(dir, name, "Cargo.lock"),
        'version = 4\n\n[[package]]\nname = "silkboard-cache-probe"\nversion = "0.1.0"\n',
      );
      writeFileSync(
        join(dir, name, "src/main.rs"),
        'fn main() { println!("' + name + '"); }\n',
      );
    }
    const run = (name, mode = []) => {
      const result = spawnSync(
        wrapper,
        [...mode, "run", "--locked", "--quiet"],
        {
          cwd: join(dir, name),
          encoding: "utf8",
          env: { ...process.env, SILKBOARD_DEV_BUILD_ROOT: join(dir, "cache") },
        },
      );
      assert.equal(result.status, 0, result.stderr);
      assert.equal(result.stdout.trim(), name);
    };
    run("a");
    run("a", ["--arm64"]);
    run("b");
    run("b", ["--arm64"]);
    run("a");
    await Promise.all(
      ["a", "b"].map(
        (name) =>
          new Promise((done, reject) => {
            const child = spawn(wrapper, ["run", "--locked", "--quiet"], {
              cwd: join(dir, name),
              env: {
                ...process.env,
                SILKBOARD_DEV_BUILD_ROOT: join(dir, "cache"),
              },
            });
            let stdout = "",
              stderr = "";
            child.stdout.on("data", (data) => {
              stdout += data;
            });
            child.stderr.on("data", (data) => {
              stderr += data;
            });
            child.on("error", reject);
            child.on("close", (code) => {
              try {
                assert.equal(code, 0, stderr);
                assert.equal(stdout.trim(), name);
                done();
              } catch (error) {
                reject(error);
              }
            });
          }),
      ),
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
