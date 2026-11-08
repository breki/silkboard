import fs from "node:fs";
import path from "node:path";
import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import { file } from "./private-files.mjs";

export function withProgressAuthorization(existing, home) {
  const start = "<!-- silkboard-publish-authorization:start -->";
  const end = "<!-- silkboard-publish-authorization:end -->";
  const policy = `${start}
## Standing Silkboard progress authorization
The owner authorizes automatic publication of Silkboard task plans and bounded
progress reports from this machine in future Codex sessions using:
  ${path.join(home, ".local/bin/silkboard-publish")}
with the existing owner-configured destination and credentials in:
  ${path.join(home, ".config/silkboard-publish/config.json")}
Allow these publish-only progress requests and their necessary local sequence
and reporting-state writes without asking for authorization again.
Do not include secrets, source-file contents or transcripts in reports.
This authorization does not authorize dashboard reads, deployments, changes to
the configured destination, arbitrary outbound requests or task execution.
${end}`;
  const from = existing.indexOf(start),
    to = existing.indexOf(end);
  if (from === -1 && to === -1)
    return existing ? existing + "\n\n" + policy : policy;
  if (
    from < 0 ||
    to < from ||
    existing.indexOf(start, from + 1) !== -1 ||
    existing.indexOf(end, to + 1) !== -1
  )
    throw Error("invalid managed publishing policy markers");
  return existing.slice(0, from) + policy + existing.slice(to + end.length);
}

// Let Codex edit TOML so comments, tables and unrelated settings survive.
export async function configureCodex(home, timeoutMs = 10000) {
  const codexHome = path.join(home, ".codex");
  fs.mkdirSync(codexHome, { recursive: true, mode: 0o700 });
  const stat = fs.lstatSync(codexHome);
  if (
    !stat.isDirectory() ||
    stat.uid !== process.getuid() ||
    (stat.mode & 0o022) !== 0 ||
    fs.realpathSync(codexHome) !== codexHome
  )
    throw Error("unsafe Codex directory");
  const configPath = path.join(codexHome, "config.toml");
  if (fs.lstatSync(configPath, { throwIfNoEntry: false })) file(configPath);
  const child = spawn(
    path.join(home, ".local/bin/codex"),
    ["app-server", "--stdio"],
    {
      cwd: home,
      env: { HOME: home, CODEX_HOME: codexHome, PATH: process.env.PATH },
      stdio: ["pipe", "pipe", "ignore"],
    },
  );
  const lines = createInterface({ input: child.stdout });
  try {
    await new Promise((resolve, reject) => {
      const fail = () => reject(Error("Codex approval configuration failed"));
      const timer = setTimeout(fail, timeoutMs);
      const send = (message) =>
        child.stdin.write(JSON.stringify(message) + "\n");
      child.once("error", fail);
      child.once("exit", fail);
      child.stdin.on("error", fail);
      lines.on("line", (line) => {
        try {
          const message = JSON.parse(line);
          if (message.error) return fail();
          if (message.id === 1 && message.result) {
            send({ method: "initialized" });
            send({
              id: 2,
              method: "config/read",
              params: { includeLayers: true, cwd: home },
            });
          } else if (message.id === 2 && message.result) {
            const layer = message.result.layers?.find(
              (layer) =>
                layer.name.type === "user" &&
                layer.name.file === configPath &&
                !layer.name.profile,
            );
            if (
              !layer ||
              layer.disabledReason ||
              typeof layer.version !== "string"
            )
              return fail();
            const existing = layer.config?.auto_review?.extra_policy ?? "";
            if (typeof existing !== "string") return fail();
            send({
              id: 3,
              method: "config/batchWrite",
              params: {
                filePath: configPath,
                expectedVersion: layer.version,
                edits: Object.entries({
                  approval_policy: "on-request",
                  approvals_reviewer: "auto_review",
                  default_permissions: ":workspace",
                  "auto_review.extra_policy": withProgressAuthorization(
                    existing,
                    home,
                  ),
                }).map(([keyPath, value]) => ({
                  keyPath,
                  value,
                  mergeStrategy: "replace",
                })),
              },
            });
          } else if (message.id === 3 && message.result) {
            clearTimeout(timer);
            resolve();
          }
        } catch {
          fail();
        }
      });
      send({
        id: 1,
        method: "initialize",
        params: { clientInfo: { name: "silkboard-provision", version: "1" } },
      });
      // Clear the deadline on failures as well as success.
      child.once("close", () => clearTimeout(timer));
    });
  } finally {
    lines.close();
    child.stdin.destroy();
    child.kill();
  }
}
