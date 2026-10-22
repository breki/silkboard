import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { test } from "node:test";
const root = path.resolve(import.meta.dirname, "../..");
const owner =
  "silkboard-release ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILXdIK5k8F3KBj1o8RSb4dQ1wbhQLRN/f9j9fJBuYhq3";
const vm =
  "silkboard-vm ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIG5IhlaY8z6OIaK6cg9ZHaCUBD9LRMZTMcyUCcXtBMpO";
test("bootstrap requires exactly both approved keys and rejects extra malformed or duplicate records", (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "silkboard-trust-"));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const file = path.join(dir, "signers");
  const check = (value) => {
    fs.writeFileSync(file, value);
    return spawnSync(
      "bash",
      [path.join(root, "deploy/vm/verify-release-signers.sh"), file],
      { encoding: "utf8" },
    );
  };
  for (const value of [`${owner}\n${vm}\n`, `${vm}\n${owner}\n`])
    assert.equal(check(value).status, 0);
  for (const value of [
    "",
    `${owner}\n`,
    `${vm}\n`,
    `${owner}\n${owner}\n`,
    `${owner}\n${vm}\ninvalid\n`,
    `${owner}\n${vm}\n${vm}\n`,
    `${owner}\n${vm} extra\n`,
  ]) {
    assert.notEqual(check(value).status, 0);
  }
});
