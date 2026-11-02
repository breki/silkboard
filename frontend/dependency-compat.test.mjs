import assert from "node:assert/strict";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import path from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { ESLint } from "eslint";
import tsParser from "@typescript-eslint/parser";

const frontend = fileURLToPath(new URL(".", import.meta.url));
const require = createRequire(import.meta.url);
const eslintRequire = createRequire(require.resolve("eslint"));
const { Minimatch, minimatch } = eslintRequire("minimatch");

test("ESLint discovers intended files and prunes ignored directories", async (t) => {
  const root = await mkdtemp(path.join(tmpdir(), "silkboard-lint-compat-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const selected = [
    "src/nested/active.js",
    "src/nested/active.ts",
    "src/[draft]/has space.ts",
    "src/.hidden/active.js",
  ];
  for (const file of [
    ...selected,
    "src/nested/generated/ignored.js",
    "src/wrong.txt",
    "outside/ignored.js",
  ]) {
    const target = path.join(root, file);
    await mkdir(path.dirname(target), { recursive: true });
    await writeFile(
      target,
      file.endsWith(".ts")
        ? "export const marker: number = missingValue;\n"
        : "missingValue();\n",
    );
  }
  const eslint = new ESLint({
    cwd: root,
    overrideConfigFile: true,
    overrideConfig: [
      {
        files: ["src/**/*.{js,ts}"],
        languageOptions: { parser: tsParser },
        rules: { "no-undef": "error" },
      },
      { ignores: ["src/**/generated/**"] },
    ],
  });
  const results = await eslint.lintFiles(["src/**/*.{js,ts}"]);
  assert.deepEqual(
    results.map((result) => path.relative(root, result.filePath)).sort(),
    selected.map((file) => path.normalize(file)).sort(),
  );
  for (const result of results) {
    assert.equal(result.fatalErrorCount, 0);
    assert.deepEqual(
      result.messages.map((message) => message.ruleId),
      ["no-undef"],
    );
  }
});

test("real project config retains TypeScript, Svelte and rune-module parsing", async () => {
  const eslint = new ESLint({ cwd: frontend });
  for (const [filePath, source] of [
    ["src/compat/plain.ts", "export const answer: number = 1;"],
    [
      "src/compat/Component.svelte",
      '<script lang="ts">const answer: number = 1;</script><p>{answer}</p>',
    ],
    [
      "src/compat/counter.svelte.ts",
      "export const counter = $state({ value: 1 });",
    ],
  ]) {
    const [result] = await eslint.lintText(source, { filePath });
    assert.equal(result.errorCount, 0, JSON.stringify(result.messages));
    assert.equal(result.warningCount, 0, JSON.stringify(result.messages));
  }
  for (const ignored of [
    "dist/Component.svelte",
    "coverage/example.ts",
    "test-results/example.js",
  ]) {
    assert.equal(await eslint.isPathIgnored(ignored), true, ignored);
  }
});

test("consumer matcher retains partial traversal, extglobs and escaped literals", () => {
  assert.equal(minimatch(".project.ts", "*.ts", { dot: true }), true);
  assert.equal(minimatch("project.js", "*.ts", { dot: true }), false);
  const matcher = new Minimatch("src/**/+(Pages|Components)/*.{svelte,ts}", {
    dot: true,
  });
  assert.equal(matcher.match("src/nested", true), true);
  assert.equal(matcher.match("outside", true), false);
  for (const [file, expected] of [
    ["src/nested/Pages/Home.svelte", true],
    ["src/.private/Components/State.ts", true],
    ["src/nested/Pages/Home.js", false],
    ["src/nested/Other/Home.svelte", false],
  ]) {
    assert.equal(matcher.match(file), expected, file);
  }
  const literal = new Minimatch("src/\\[draft\\]/file\\?.ts");
  assert.equal(literal.match("src/[draft]/file?.ts"), true);
  assert.equal(literal.match("src/d/file1.ts"), false);
});
