import { test } from "node:test";
import assert from "node:assert/strict";
import { nameForTask, renameTask } from "./codex-title.mjs";

const env = { CODEX_THREAD_ID: "native-session-id" };
function control(reply) {
  const calls = [];
  const socket = new EventTarget();
  const state = { disposed: false, calls };
  socket.send = (value) => {
    const request = JSON.parse(value);
    calls.push(request);
    reply(
      request,
      (data) =>
        socket.dispatchEvent(
          new MessageEvent("message", {
            data: typeof data === "string" ? data : JSON.stringify(data),
          }),
        ),
      socket,
    );
  };
  state.transport = () => {
    queueMicrotask(() => socket.dispatchEvent(new Event("open")));
    return {
      socket,
      dispose: async () => {
        state.disposed = true;
      },
    };
  };
  return state;
}

test("task labels are readable, bounded and reject empty or control input", () => {
  assert.equal(
    nameForTask("  Reuse local builds  "),
    "Silkboard · Reuse local builds",
  );
  assert.equal(Array.from(nameForTask("é".repeat(200))).length, 120);
  for (const value of ["", "   ", "line\nbreak", "nul\0", "\ud800", null])
    assert.throws(() => nameForTask(value));
});

test("only initialization and native self-targeted rename are sent", async () => {
  const c = control((request, reply) => {
    if (request.id === 1) reply({ id: 1, result: { serverInfo: "ignored" } });
    if (request.id === 2) reply({ id: 2, result: {} });
  });
  const title = await renameTask("Reuse local builds", {
    env,
    transport: c.transport,
  });
  assert.equal(title, "Silkboard · Reuse local builds");
  assert.deepEqual(
    c.calls.map((c) => c.method),
    ["initialize", "initialized", "thread/name/set"],
  );
  assert.deepEqual(c.calls[2].params, {
    threadId: env.CODEX_THREAD_ID,
    name: title,
  });
  assert.equal(c.disposed, true);
});

test("missing or invalid native identity fails before connecting", async () => {
  for (const id of [undefined, "", "other\nthread", "x".repeat(201)]) {
    await assert.rejects(
      renameTask("Task", {
        env: { CODEX_THREAD_ID: id },
        transport: () => {
          assert.fail("must not connect");
        },
      }),
    );
  }
});

test("RPC rejection, malformed responses and dropped connections fail clearly", async () => {
  const replies = [
    (_, reply) => reply({ id: 1, error: { message: "PRIVATE" } }),
    (_, reply) => reply("not JSON"),
    (_, reply) => reply("x".repeat(65537)),
    (_, reply, socket) => socket.dispatchEvent(new Event("error")),
    (_, reply, socket) => socket.dispatchEvent(new Event("close")),
    (request, reply) =>
      reply(
        request.id === 1
          ? { id: 1, result: {} }
          : request.id === 2
            ? { id: 2, error: { message: "PRIVATE" } }
            : { method: "ignored" },
      ),
    (_, reply) => reply({ id: 1 }),
    (request, reply) => {
      if (request.id === 1) reply({ id: 1, result: {} });
      if (request.id === 2) reply({ id: 2 });
    },
  ];
  for (const reply of replies) {
    const c = control(reply);
    await assert.rejects(
      renameTask("Task", { env, transport: c.transport }),
      (error) => !error.message.includes("PRIVATE"),
    );
    assert.equal(c.disposed, true);
  }
});

test("unrelated notifications do not complete a rename; timeout is bounded", async () => {
  const c = control((_, reply) =>
    reply({ method: "thread/status/changed", params: {} }),
  );
  await assert.rejects(
    renameTask("Task", { env, transport: c.transport, timeoutMs: 10 }),
    /timed out/,
  );
  assert.equal(c.disposed, true);
});

// Exercise the real Unix/WebSocket handshake against an owned metadata fixture.
import { createServer } from "node:http";
import { createHash } from "node:crypto";
import {
  mkdtempSync,
  mkdirSync,
  chmodSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

async function fixture(run, rejectRename = false) {
  const home = mkdtempSync(join(tmpdir(), "silkboard-codex-title-"));
  const control = join(home, "app-server-control");
  mkdirSync(control, { mode: 0o700 });
  const calls = [],
    sockets = new Set();
  const server = createServer();
  server.on("upgrade", (request, socket) => {
    sockets.add(socket);
    socket.on("close", () => sockets.delete(socket));
    const accept = createHash("sha1")
      .update(
        request.headers["sec-websocket-key"] +
          "258EAFA5-E914-47DA-95CA-C5AB0DC85B11",
      )
      .digest("base64");
    socket.write(
      "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: " +
        accept +
        "\r\n\r\n",
    );
    let buffer = Buffer.alloc(0);
    socket.on("data", (data) => {
      buffer = Buffer.concat([buffer, data]);
      while (buffer.length >= 6) {
        const opcode = buffer[0] & 15;
        let length = buffer[1] & 127,
          offset = 2;
        if (length === 126) {
          if (buffer.length < 8) return;
          length = buffer.readUInt16BE(2);
          offset = 4;
        }
        if (buffer.length < offset + 4 + length) return;
        const mask = buffer.subarray(offset, offset + 4),
          payload = Buffer.from(
            buffer.subarray(offset + 4, offset + 4 + length),
          );
        buffer = buffer.subarray(offset + 4 + length);
        if (opcode === 8) {
          socket.end();
          return;
        }
        for (let i = 0; i < payload.length; i++) payload[i] ^= mask[i % 4];
        const request = JSON.parse(payload.toString());
        calls.push(request);
        if (request.id) {
          const response =
            request.id === 2 && rejectRename
              ? { id: 2, error: { message: "PRIVATE-RPC-DETAIL" } }
              : { id: request.id, result: {} };
          const bytes = Buffer.from(JSON.stringify(response));
          socket.write(
            Buffer.concat([Buffer.from([0x81, bytes.length]), bytes]),
          );
        }
      }
    });
  });
  await new Promise((done) =>
    server.listen(join(control, "app-server-control.sock"), done),
  );
  try {
    await run(
      { CODEX_HOME: home, CODEX_THREAD_ID: "fixture-current" },
      calls,
      control,
    );
  } finally {
    for (const socket of sockets) socket.destroy();
    await new Promise((done) => server.close(done));
    rmSync(home, { recursive: true, force: true });
  }
}

async function cli(args, env) {
  return new Promise((done, reject) => {
    const child = spawn(
      process.execPath,
      [fileURLToPath(new URL("./codex-title.mjs", import.meta.url)), ...args],
      { env: { ...process.env, ...env } },
    );
    let stdout = "",
      stderr = "";
    child.stdout.on("data", (data) => {
      stdout += data;
    });
    child.stderr.on("data", (data) => {
      stderr += data;
    });
    child.on("error", reject);
    child.on("close", (code) => done({ code, stdout, stderr }));
  });
}

test("native CLI uses only the owned Unix socket and current thread", async () => {
  await fixture(async (env, calls) => {
    const result = await cli(["--task-title", "Current task"], env);
    assert.equal(result.code, 0, result.stderr);
    assert.equal(JSON.parse(result.stdout).title, "Silkboard · Current task");
    assert.deepEqual(
      calls.map((c) => c.method),
      ["initialize", "initialized", "thread/name/set"],
    );
    assert.equal(calls[2].params.threadId, env.CODEX_THREAD_ID);
    assert.equal((await cli(["--thread-id", "somebody-else"], env)).code, 1);
  });
});

test("native failures and unsafe socket storage do not expose private details", async () => {
  await fixture(async (env) => {
    const result = await cli(["--task-title", "Task"], env);
    assert.equal(result.code, 1);
    assert.ok(!result.stderr.includes("PRIVATE-RPC-DETAIL"));
  }, true);
  await fixture(async (env, _, control) => {
    chmodSync(control, 0o755);
    await assert.rejects(renameTask("Task", { env }), /Private Codex/);
  });
  await assert.rejects(
    renameTask("Task", { env: { ...env, CODEX_HOME: "relative" } }),
    /Private Codex/,
  );
  await assert.rejects(
    renameTask("Task", {
      env: { ...env, CODEX_HOME: "/must-not-exist-for-silkboard" },
    }),
    /Private Codex/,
  );
  const home = mkdtempSync(join(tmpdir(), "silkboard-codex-title-file-"));
  try {
    mkdirSync(join(home, "app-server-control"));
    writeFileSync(
      join(home, "app-server-control/app-server-control.sock"),
      "not a socket",
    );
    await assert.rejects(
      renameTask("Task", { env: { ...env, CODEX_HOME: home } }),
      /Private Codex/,
    );
  } finally {
    rmSync(home, { recursive: true, force: true });
  }
});

test("a send failure stops later protocol actions", async () => {
  const c = control((request, reply) => {
    if (request.id === 1) reply({ id: 1, result: {} });
    if (request.method === "initialized")
      throw new Error("PRIVATE-SEND-DETAIL");
  });
  await assert.rejects(
    renameTask("Task", { env, transport: c.transport }),
    /unavailable/,
  );
  assert.deepEqual(
    c.calls.map((c) => c.method),
    ["initialize", "initialized"],
  );
  assert.equal(c.disposed, true);
});
