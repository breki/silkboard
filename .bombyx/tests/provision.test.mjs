import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir, arch } from 'node:os';
import { join, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

const root = resolve(import.meta.dirname, '../..');
function fixture(t) {
  const dir = mkdtempSync(join(tmpdir(), 'silkboard-provision-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const bin = join(dir, 'bin');
  const home = join(dir, 'home');
  const log = join(dir, 'commands.jsonl');
  mkdirSync(bin);
  mkdirSync(join(home, '.config/silkboard'), { recursive: true });
  writeFileSync(join(home, '.config/silkboard/dev-env.sh'), ':\n');
  for (const command of ['apt-get', 'bash', 'npm', 'cargo', 'codex', 'claude']) {
    writeFileSync(join(bin, command), `#!/usr/bin/env node
const fs = require('node:fs');
const name = require('node:path').basename(process.argv[1]);
fs.appendFileSync(process.env.FIXTURE_LOG, JSON.stringify([name, ...process.argv.slice(2)]) + '\\n');
if (process.env.FIXTURE_FAIL === name) process.exit(7);
`, { mode: 0o755 });
  }
  writeFileSync(join(bin, 'sudo'), '#!/bin/sh\n[ "$1" = -n ] || exit 2\nshift\ncase "$1" in apt-get) exec "$@" ;; *) exit 97 ;; esac\n', { mode: 0o755 });
  writeFileSync(join(bin, 'curl'), `#!/usr/bin/env node
const fs = require('node:fs');
const args = process.argv.slice(2);
fs.appendFileSync(process.env.FIXTURE_LOG, JSON.stringify(['curl', ...args]) + '\\n');
fs.writeFileSync(args[args.indexOf('--output') + 1], 'wrong download bytes');
`, { mode: 0o755 });
  const env = { ...process.env, HOME: home, PATH: `${bin}:${process.env.PATH}`, FIXTURE_LOG: log };
  return {
    home,
    prepareTools() {
      const nodeArch = arch() === 'arm64' ? 'arm64' : 'x64';
      const nodeBin = join(home, '.local/share/silkboard-dev', `node-v24.14.1-linux-${nodeArch}`, 'bin');
      mkdirSync(nodeBin, { recursive: true });
      writeFileSync(join(nodeBin, 'node'), `#!/bin/sh
if [ "\$1" = --version ]; then echo v24.14.1; else exec '${process.execPath}' "\$@"; fi
`, { mode: 0o755 });
      mkdirSync(join(home, '.cargo/bin'), { recursive: true });
      writeFileSync(join(home, '.cargo/bin/rustup'), '#!/bin/sh\nexit 0\n', { mode: 0o755 });
    },
    run(script, extra = {}) {
      return spawnSync('/bin/bash', [join(root, '.bombyx', script)], {
        cwd: tmpdir(), encoding: 'utf8', timeout: 10000, env: { ...env, BOMBYX_VM_HOST: 'fixture', ...extra },
      });
    },
    calls() { try { return readFileSync(log, 'utf8').trim().split('\n').map(JSON.parse); } catch { return []; } },
  };
}

test('provisioning refuses non-Bombyx use before installing anything', (t) => {
  const f = fixture(t);
  assert.notEqual(f.run('provision.sh', { BOMBYX_VM_HOST: '' }).status, 0);
  assert.deepEqual(f.calls(), []);
});

test('provisioning installs native prerequisites and invokes tools then project setup', (t) => {
  const f = fixture(t);
  const result = f.run('provision.sh');
  assert.equal(result.status, 0, result.stderr);
  const calls = f.calls();
  assert.deepEqual(calls[0], ['apt-get', 'update']);
  assert.equal(calls[1][0], 'apt-get');
  assert.ok(calls[1].includes('bubblewrap'));
  assert.ok(!calls[1].includes('docker.io'));
  assert.deepEqual(calls.slice(2), [
    ['bash', join(root, '.bombyx/install-tools.sh')],
    ['bash', join(root, '.bombyx/setup.sh')],
  ]);
});

test('package failure stops before installing user tools', (t) => {
  const f = fixture(t);
  assert.equal(f.run('provision.sh', { FIXTURE_FAIL: 'apt-get' }).status, 7);
  assert.deepEqual(f.calls(), [['apt-get', 'update']]);
});

test('tool installation failure stops before project setup', (t) => {
  const f = fixture(t);
  assert.equal(f.run('provision.sh', { FIXTURE_FAIL: 'bash' }).status, 7);
  assert.equal(f.calls().filter((a) => a[0] === 'bash').length, 1);
});

test('project setup installs locked dependencies and starts both agent CLIs', (t) => {
  const f = fixture(t);
  const result = f.run('setup.sh');
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(f.calls(), [
    ['npm', 'ci', '--prefix', 'frontend'],
    ['npm', 'exec', '--', 'playwright', 'install', '--with-deps', 'chromium'],
    ['cargo', 'fetch', '--locked'],
    ['codex', '--version'],
    ['claude', '--version'],
  ]);
});

test('a corrupted bootstrap download fails before running installers', (t) => {
  const f = fixture(t);
  const result = f.run('install-tools.sh');
  assert.notEqual(result.status, 0);
  assert.match(result.stderr + result.stdout, /checksum|FAILED/i);
  assert.ok(f.calls().every((a) => a[0] === 'curl'));
});


test('repeat tool setup preserves agent settings and adds shell initialization once', (t) => {
  const f = fixture(t);
  f.prepareTools();
  mkdirSync(join(f.home, '.codex'));
  const config = join(f.home, '.codex/config.toml');
  writeFileSync(config, 'approval_policy = "on-request"\n');
  for (let attempt = 0; attempt < 2; attempt++) {
    const result = f.run('install-tools.sh');
    assert.equal(result.status, 0, result.stderr);
  }
  assert.equal(readFileSync(config, 'utf8'), 'approval_policy = "on-request"\n');
  for (const profile of ['.profile', '.bashrc']) {
    assert.equal(readFileSync(join(f.home, profile), 'utf8').trim().split('\n').length, 1);
  }
  const envFile = readFileSync(join(f.home, '.config/silkboard/dev-env.sh'), 'utf8');
  assert.match(envFile, /RUSTUP_TOOLCHAIN=1.93.1/);
  assert.match(envFile, /agents\/node_modules\/.bin/);
  assert.ok(!/sandbox_mode|approval_policy/.test(envFile));
});
