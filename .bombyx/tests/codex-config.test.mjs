import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { test } from "node:test";
import { configureCodex, withProgressAuthorization } from "../codex-config.mjs";

function fixture(t, response = '{"id":3,"result":{}}') {
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
    if (request.method !== 'config/read') process.exit(70);
    const override = process.env.HOME + '/read-response.json';
    if (fs.existsSync(override)) { console.log(fs.readFileSync(override, 'utf8')); return; }
    const policyFile = process.env.HOME + '/policy.json';
    const policy = fs.existsSync(policyFile) ? JSON.parse(fs.readFileSync(policyFile)) : '';
    console.log(JSON.stringify({id:2,result:{layers:[{name:{type:'user',file:process.env.CODEX_HOME+'/config.toml'},version:'fixture-version',config:{auto_review:{extra_policy:policy}}}]}}));
  }
  if (request.id === 3) {
    fs.writeFileSync(process.env.HOME + '/request.json', line);
    fs.writeFileSync(process.env.HOME + '/policy.json', JSON.stringify(request.params.edits.at(-1).value));
    console.log(${JSON.stringify(response)});
  }
});
`,
    { mode: 0o700 },
  );
  return home;
}

test("provisioning preserves existing rules and installs bounded publishing authorization once across reruns", async (t) => {
  const home = fixture(t);
  fs.writeFileSync(
    home + "/policy.json",
    JSON.stringify("Existing owner policy."),
  );
  let previousPolicy;
  for (let i = 0; i < 2; i++) {
    await configureCodex(home);
    const request = JSON.parse(fs.readFileSync(home + "/request.json"));
    assert.deepEqual(
      {
        ...request,
        params: { ...request.params, edits: request.params.edits.slice(0, 3) },
      },
      {
        id: 3,
        method: "config/batchWrite",
        params: {
          filePath: home + "/.codex/config.toml",
          expectedVersion: "fixture-version",
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
      },
    );
    const policy = request.params.edits[3];
    assert.equal(policy.keyPath, "auto_review.extra_policy");
    assert.equal(policy.mergeStrategy, "replace");
    assert.match(policy.value, /^Existing owner policy\./);
    assert.ok(policy.value.includes(home + "/.local/bin/silkboard-publish"));
    assert.ok(
      policy.value.includes(home + "/.config/silkboard-publish/config.json"),
    );
    assert.match(policy.value, /does not authorize dashboard reads/i);
    assert.equal(
      policy.value.split("silkboard-publish-authorization:start").length,
      2,
    );
    if (previousPolicy) assert.equal(policy.value, previousPolicy);
    previousPolicy = policy.value;
  }
});

test("managed publishing policy is additive, updateable and refuses broken markers", () => {
  const home = "/home/agent";
  const fresh = withProgressAuthorization("", home);
  assert.equal(withProgressAuthorization(fresh, home), fresh);
  const start = "<!-- silkboard-publish-authorization:start -->";
  const end = "<!-- silkboard-publish-authorization:end -->";
  const updated = withProgressAuthorization(
    `Before\n${start}\nOld rule\n${end}\nAfter`,
    home,
  );
  assert.ok(updated.startsWith("Before\n"));
  assert.ok(updated.endsWith("\nAfter"));
  assert.ok(!updated.includes("Old rule"));
  for (const broken of [start, end, end + start, start + start + end])
    assert.throws(() => withProgressAuthorization(broken, home));
});
test("missing, disabled or malformed user layers fail without replacing the policy", async (t) => {
  for (const kind of ["missing", "disabled", "version", "policy", "empty"]) {
    const home = fixture(t);
    const layer = {
      name: { type: "user", file: home + "/.codex/config.toml" },
      version: "fixture-version",
      config: {},
    };
    if (kind === "disabled") layer.disabledReason = "Managed restriction";
    if (kind === "version") delete layer.version;
    if (kind === "policy")
      layer.config = { auto_review: { extra_policy: true } };
    const result =
      kind === "empty" ? {} : { layers: kind === "missing" ? [] : [layer] };
    fs.writeFileSync(
      home + "/read-response.json",
      JSON.stringify({ id: 2, result }),
    );
    await assert.rejects(configureCodex(home), /configuration failed/);
    assert.equal(fs.existsSync(home + "/request.json"), false);
  }
});

test("configuration errors, invalid responses, early exit and timeouts fail provisioning", async (t) => {
  for (const response of [
    '{"id":3,"error":{"message":"private"}}',
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
