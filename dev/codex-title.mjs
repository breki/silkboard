// Local metadata-only client; never starts a server or reads thread history.
import { createRequire } from "node:module";
import { realpathSync, statSync } from "node:fs";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(
  new URL("../frontend/package.json", import.meta.url),
);
const { Client, WebSocket } = require("undici");

export function nameForTask(taskTitle) {
  if (
    typeof taskTitle !== "string" ||
    !taskTitle.trim() ||
    /[\p{Cc}\p{Cs}]/u.test(taskTitle)
  ) {
    throw new Error("Task title must be nonempty single-line text");
  }
  return Array.from(`Silkboard · ${taskTitle.trim()}`).slice(0, 120).join("");
}

function connect(env) {
  const home = env.CODEX_HOME || (env.HOME && join(env.HOME, ".codex"));
  if (!home || !isAbsolute(home) || typeof process.getuid !== "function")
    throw new Error("Private Codex control socket unavailable");
  let socketPath;
  try {
    socketPath = realpathSync(
      join(home, "app-server-control/app-server-control.sock"),
    );
    const socket = statSync(socketPath),
      parent = statSync(dirname(socketPath));
    if (
      !socket.isSocket() ||
      socket.uid !== process.getuid() ||
      parent.uid !== process.getuid() ||
      (parent.mode & 0o077) !== 0
    )
      throw new Error();
  } catch {
    throw new Error("Private Codex control socket unavailable");
  }
  // The origin is fixed; socketPath makes the transport Unix-only, without a
  // TCP fallback or proxy-environment handling.
  const client = new Client("http://localhost", { socketPath });
  const socket = new WebSocket("ws://localhost", { dispatcher: client });
  return {
    socket,
    dispose: async () => {
      socket.close();
      await client.destroy();
    },
  };
}

export async function renameTask(
  taskTitle,
  { env = process.env, transport = connect, timeoutMs = 2000 } = {},
) {
  const title = nameForTask(taskTitle),
    threadId = env.CODEX_THREAD_ID;
  if (typeof threadId !== "string" || !/^[A-Za-z0-9_-]{1,200}$/.test(threadId))
    throw new Error("Native Codex thread ID unavailable");
  const { socket, dispose } = transport(env);
  try {
    await new Promise((done, reject) => {
      let finished = false,
        initialized = false;
      const finish = (message) => {
        if (finished) return;
        finished = true;
        clearTimeout(timer);
        if (message) reject(new Error(message));
        else done();
      };
      const fail = () => finish("Codex title update unavailable");
      const send = (message) => {
        if (finished) return;
        try {
          socket.send(JSON.stringify(message));
        } catch {
          fail();
        }
      };
      const timer = setTimeout(
        () => finish("Codex title update timed out"),
        timeoutMs,
      );
      socket.addEventListener("error", fail);
      socket.addEventListener("close", fail);
      socket.addEventListener("open", () =>
        send({
          id: 1,
          method: "initialize",
          params: {
            clientInfo: { name: "silkboard-task-title", version: "1" },
          },
        }),
      );
      socket.addEventListener("message", ({ data }) => {
        if (finished) return;
        let response;
        try {
          if (typeof data !== "string" || data.length > 65536)
            throw new Error();
          response = JSON.parse(data);
          if (
            !response ||
            typeof response !== "object" ||
            Array.isArray(response)
          )
            throw new Error();
        } catch {
          fail();
          return;
        }
        if (response.id === 1 && !initialized) {
          if (response.error || !("result" in response)) {
            fail();
            return;
          }
          initialized = true;
          send({ method: "initialized" });
          send({
            id: 2,
            method: "thread/name/set",
            params: { threadId, name: title },
          });
        } else if (response.id === 2 && initialized) {
          if (response.error || !("result" in response)) fail();
          else finish();
        }
      });
    });
    return title;
  } finally {
    await dispose();
  }
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  try {
    if (process.argv.length !== 4 || process.argv[2] !== "--task-title")
      throw new Error("Usage: node dev/codex-title.mjs --task-title TEXT");
    const title = await renameTask(process.argv[3]);
    console.log(JSON.stringify({ renamed: true, title }));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
