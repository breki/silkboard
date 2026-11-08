import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { test } from "node:test";
import { configureCodex } from "../codex-config.mjs";

function fixture(t, response = '{"id":2,"result":{}}') {
  const home = fs.mkdtempSync(path.join(os.tmpdir(), "silkboard-approvals-"));
  t.after(() => fs.rmSync(home, { recursive: true, force: true }));
  fs.mkdirSync(home + "/.local/bin", { recursive: true });
  fs.writeFileSync(
    home + "/.local/bin/codex",
    `#!${process.execPath}
const fs = require('node:fs');
const lines = require('node:readline').createInterface({input: process.stdin});
lines.on('line', line => {
  const request = JSON.parse(line);
  if (request.id === 1) console.log('{"id":1,"result":{}}');
  if (request.id === 2) {
    fs.writeFileSync(process.env.HOME + '/request.json', line);
    console.log(${JSON.stringify(response)});
  }
});
`,
    { mode: 0o700 },
  );
  return home;
}

test("provisioning requests only the three persistent approval defaults on every run", async (t) => {
  const home = fixture(t);
  for (let i = 0; i < 2; i++) {
    await configureCodex(home);
    assert.deepEqual(JSON.parse(fs.readFileSync(home + "/request.json")), {
      id: 2,
      method: "config/batchWrite",
      params: {
        edits: [
          {
            keyPath: "approval_policy",
            value: "on-request",
            mergeStrategy: "replace",
          },
          {
            keyPath: "approvals_reviewer",
            value: "auto_review",
            mergeStrategy: "replace",
          },
          {
            keyPath: "default_permissions",
            value: ":workspace",
            mergeStrategy: "replace",
          },
        ],
      },
    });
  }
});

test("configuration errors, invalid responses, early exit and timeouts fail provisioning", async (t) => {
  for (const response of [
    '{"id":2,"error":{"message":"private"}}',
    "invalid",
    "{}",
  ]) {
    const home = fixture(t, response);
    await assert.rejects(configureCodex(home, 200), /configuration failed/);
  }
  const home = fixture(t);
  fs.writeFileSync(home + "/.local/bin/codex", "#!/bin/sh\nexit 1\n");
  await assert.rejects(configureCodex(home), /configuration failed/);
  fs.unlinkSync(home + "/.local/bin/codex");
  await assert.rejects(configureCodex(home), /configuration failed/);
});

test("unsafe Codex paths are rejected before invoking the writer", async (t) => {
  const home = fixture(t);
  fs.mkdirSync(home + "/other");
  fs.symlinkSync(home + "/other", home + "/.codex");
  await assert.rejects(configureCodex(home), /unsafe Codex directory/);
  fs.unlinkSync(home + "/.codex");
  fs.mkdirSync(home + "/.codex", { mode: 0o700 });
  fs.writeFileSync(home + "/other/config.toml", "retained");
  fs.symlinkSync(home + "/other/config.toml", home + "/.codex/config.toml");
  await assert.rejects(configureCodex(home));
  assert.equal(
    fs.readFileSync(home + "/other/config.toml", "utf8"),
    "retained",
  );
});
