import fs from "node:fs";
import path from "node:path";
import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import { file } from "./private-files.mjs";

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
              method: "config/batchWrite",
              params: {
                edits: Object.entries({
                  approval_policy: "on-request",
                  approvals_reviewer: "auto_review",
                  default_permissions: ":workspace",
                }).map(([keyPath, value]) => ({
                  keyPath,
                  value,
                  mergeStrategy: "replace",
                })),
              },
            });
          } else if (message.id === 2 && message.result) {
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
