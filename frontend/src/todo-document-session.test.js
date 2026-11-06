import { afterEach, expect, test, vi } from "vitest";
import {
  TodoDocumentSession,
  clearRetainedDraft,
  retainedDraft,
} from "./todo-document-session";
import { ApiError } from "./feed-api";
const base = {
  revision: "0",
  items: [{ id: "1", parent_id: null, text: "Task", completed: false }],
};
const deferred = () => {
  let resolve, reject;
  const promise = new Promise((a, b) => {
    resolve = a;
    reject = b;
  });
  return { promise, resolve, reject };
};
const reply = (revision, items) => ({
  revision,
  items: items.map((row, n) => ({
    id: row.id ?? String(n + 2),
    parent_id: row.parent_key
      ? (items.find((r) => r.key === row.parent_key)?.id ?? "2")
      : null,
    text: row.text,
    completed: row.completed,
  })),
  keys: Object.fromEntries(
    items.map((row, n) => [row.key, row.id ?? String(n + 2)]),
  ),
});
const sessions = [];
function start(api) {
  vi.useFakeTimers();
  const callbacks = { change: vi.fn(), expire: vi.fn(), reset: vi.fn() };
  const session = new TodoDocumentSession(base, callbacks, api);
  sessions.push(session);
  return { session, callbacks };
}
afterEach(() => {
  sessions.forEach((s) => s.dispose());
  sessions.length = 0;
  clearRetainedDraft();
  vi.useRealTimers();
});
test("coalesces typing during an in-flight save and never resets the editor on acknowledgements", async () => {
  const first = deferred();
  const api = {
    save: vi
      .fn()
      .mockReturnValueOnce(first.promise)
      .mockImplementation(async (revision, items) =>
        reply(String(Number(revision) + 1), items),
      ),
    load: vi.fn(),
  };
  const { session, callbacks } = start(api);
  const key = session.rows[0].key;
  session.update([{ ...session.rows[0], text: "First edit" }]);
  await vi.advanceTimersByTimeAsync(700);
  expect(api.save).toHaveBeenCalledTimes(1);
  session.update([{ ...session.rows[0], text: "Latest edit" }]);
  await vi.advanceTimersByTimeAsync(2000);
  expect(session.rows[0].text).toBe("Latest edit");
  expect(api.save).toHaveBeenCalledTimes(1);
  first.resolve(reply("1", api.save.mock.calls[0][1]));
  await vi.advanceTimersByTimeAsync(700);
  expect(api.save).toHaveBeenCalledTimes(2);
  expect(api.save.mock.calls[1][0]).toBe("1");
  expect(api.save.mock.calls[1][1][0].text).toBe("Latest edit");
  expect(session.phase).toBe("saved");
  expect(session.rows[0].key).toBe(key);
  expect(callbacks.reset).not.toHaveBeenCalled();
});
test("conflicts retain an editable draft and require explicit recovery before another write", async () => {
  const remote = {
    revision: "1",
    items: [{ ...base.items[0], text: "Other tab" }],
  };
  const api = {
    save: vi.fn().mockRejectedValue(new ApiError(409)),
    load: vi.fn().mockResolvedValue(remote),
  };
  const { session } = start(api);
  session.update([{ ...session.rows[0], text: "My edit" }]);
  await vi.advanceTimersByTimeAsync(700);
  session.update([{ ...session.rows[0], text: "Still editing" }]);
  await vi.advanceTimersByTimeAsync(5000);
  expect(session.phase).toBe("conflict");
  expect(api.save).toHaveBeenCalledTimes(1);
  await session.checkSaved();
  expect(session.rows[0].text).toBe("Still editing");
  expect(session.latest.items[0].text).toBe("Other tab");
});
test("an unconfirmed save is recognized by an exact subsequent read without creating new IDs again", async () => {
  let submitted;
  const api = {
    save: vi.fn(async (revision, items) => {
      submitted = reply("1", items);
      throw new ApiError(0);
    }),
    load: vi.fn(async () => submitted),
  };
  const { session, callbacks } = start(api);
  session.update([
    ...session.rows,
    { key: "new-task", depth: 1, text: "New", completed: true },
  ]);
  await vi.advanceTimersByTimeAsync(700);
  expect(session.phase).toBe("error");
  await session.checkSaved();
  expect(session.phase).toBe("saved");
  expect(session.ids.get("new-task")).toBe("3");
  expect(api.save).toHaveBeenCalledTimes(1);
  expect(callbacks.reset).not.toHaveBeenCalled();
});
test("composition and invalid empty drafts do not write or discard the text", async () => {
  const api = { save: vi.fn(), load: vi.fn() };
  const { session } = start(api);
  session.composition(true);
  session.update([{ ...session.rows[0], text: "日本語" }]);
  await vi.advanceTimersByTimeAsync(2000);
  expect(api.save).not.toHaveBeenCalled();
  session.update([{ ...session.rows[0], text: "" }]);
  session.composition(false);
  await vi.advanceTimersByTimeAsync(1000);
  expect(api.save).not.toHaveBeenCalled();
  expect(session.validation).toBeTruthy();
  expect(session.rows[0].text).toBe("");
});
test("session expiry keeps a private in-memory draft for sign-in recovery", async () => {
  const api = {
    save: vi.fn().mockRejectedValue(new ApiError(401)),
    load: vi.fn(),
  };
  const { session, callbacks } = start(api);
  session.update([{ ...session.rows[0], text: "Keep me" }]);
  await vi.advanceTimersByTimeAsync(700);
  expect(callbacks.expire).toHaveBeenCalledTimes(1);
  expect(retainedDraft().rows[0].text).toBe("Keep me");
});
test("reading the unchanged saved version enables one explicit retry", async () => {
  const api = {
    save: vi
      .fn()
      .mockRejectedValueOnce(new ApiError(0))
      .mockImplementation(async (revision, items) => reply("1", items)),
    load: vi.fn().mockResolvedValue(base),
  };
  const { session } = start(api);
  session.update([{ ...session.rows[0], text: "Draft" }]);
  await vi.advanceTimersByTimeAsync(700);
  await session.checkSaved();
  expect(session.retryReady).toBe(true);
  expect(api.save).toHaveBeenCalledTimes(1);
  await session.retry();
  expect(api.save).toHaveBeenCalledTimes(2);
  expect(session.phase).toBe("saved");
});
test("loading a saved replacement never discards edits made while that read was pending", async () => {
  const read = deferred();
  const api = {
    save: vi.fn().mockRejectedValue(new ApiError(409)),
    load: vi.fn().mockReturnValue(read.promise),
  };
  const { session, callbacks } = start(api);
  session.update([{ ...session.rows[0], text: "Draft" }]);
  await vi.advanceTimersByTimeAsync(700);
  session.latest = { revision: "1", items: base.items };
  const pending = session.useSaved();
  session.update([{ ...session.rows[0], text: "Newer local edit" }]);
  read.resolve({ revision: "1", items: base.items });
  await pending;
  expect(session.rows[0].text).toBe("Newer local edit");
  expect(callbacks.reset).not.toHaveBeenCalled();
});
test("explicit replacement resets history only after a fresh saved read", async () => {
  const remote = {
    revision: "2",
    items: [{ ...base.items[0], text: "Remote" }],
  };
  const api = {
    save: vi.fn().mockRejectedValue(new ApiError(409)),
    load: vi.fn().mockResolvedValue(remote),
  };
  const { session, callbacks } = start(api);
  session.update([{ ...session.rows[0], text: "Draft" }]);
  await session.flush();
  await session.retry();
  expect(api.save).toHaveBeenCalledTimes(1);
  await session.useSaved();
  expect(callbacks.reset).not.toHaveBeenCalled();
  await session.checkSaved();
  await session.useSaved();
  expect(session.rows[0].text).toBe("Remote");
  expect(session.dirty).toBe(false);
  expect(callbacks.reset).toHaveBeenCalledOnce();
  expect(session.phase).toBe("saved");
});
test("failed recovery reads retain the draft and permit another explicit review", async () => {
  const api = {
    save: vi.fn().mockRejectedValue(new Error("offline")),
    load: vi.fn().mockRejectedValue(new Error("offline")),
  };
  const { session } = start(api);
  session.update([{ ...session.rows[0], text: "Draft" }]);
  await session.flush();
  await session.checkSaved();
  expect(session.message).toContain("Couldn't load");
  session.latest = { revision: "1", items: base.items };
  await session.useSaved();
  expect(session.rows[0].text).toBe("Draft");
  expect(session.phase).toBe("error");
  expect(session.reading).toBe(false);
});
test("restoring a retained draft compares the fresh server snapshot and explicit discard clears memory", async () => {
  const api = {
    save: vi.fn().mockRejectedValue(new ApiError(0)),
    load: vi.fn(),
  };
  const { session } = start(api);
  session.update([{ ...session.rows[0], text: "Draft" }]);
  await session.flush();
  session.keep();
  const { session: restored } = start(api);
  expect(restored.rows[0].text).toBe("Draft");
  expect(restored.retryReady).toBe(true);
  restored.discard();
  restored.keep();
  expect(retainedDraft()).toBeNull();
});
test("recovery recognizes all saved fields and refuses mismatched remote content", async () => {
  for (const mutate of [
    (d) => {
      d.items[0].completed = true;
    },
    (d) => {
      d.items[0].id = "9";
    },
    (d) => {
      d.items[0].text = "Remote";
    },
    (d) => {
      d.items[0].parent_id = "9";
    },
    (d) => {
      d.items = [];
    },
  ]) {
    clearRetainedDraft();
    const remote = structuredClone({ ...base, revision: "1" });
    mutate(remote);
    const api = {
      save: vi.fn().mockRejectedValue(new ApiError(0)),
      load: vi.fn().mockResolvedValue(remote),
    };
    const { session } = start(api);
    session.update([{ ...session.rows[0], text: "Draft" }]);
    await session.flush();
    await session.checkSaved();
    expect(session.phase).toBe("conflict");
    expect(session.rows[0].text).toBe("Draft");
    session.dispose();
  }
});
test("disposed reads and writes never reset the editor or expire a new session", async () => {
  const read = deferred(),
    save = deferred();
  const api = {
    save: vi.fn().mockReturnValue(save.promise),
    load: vi.fn().mockReturnValue(read.promise),
  };
  const { session, callbacks } = start(api);
  session.update([{ ...session.rows[0], text: "Draft" }]);
  const pending = session.flush();
  await session.checkSaved();
  expect(api.load).not.toHaveBeenCalled();
  session.dispose();
  save.reject(new ApiError(401));
  await pending;
  expect(callbacks.expire).not.toHaveBeenCalled();
  const { session: second, callbacks: cb } = start(api);
  const reading = second.checkSaved();
  await second.checkSaved();
  expect(api.load).toHaveBeenCalledOnce();
  second.dispose();
  read.resolve(base);
  await reading;
  expect(cb.reset).not.toHaveBeenCalled();
});
test("clean undo cancels a scheduled save and invalid drafts never flush", async () => {
  const api = { save: vi.fn(), load: vi.fn().mockResolvedValue(base) };
  const { session } = start(api);
  const original = session.rows;
  session.update([{ ...original[0], text: "Changed" }]);
  session.update(original);
  await session.flush();
  await vi.advanceTimersByTimeAsync(1000);
  expect(api.save).not.toHaveBeenCalled();
  await session.checkSaved();
  expect(session.phase).toBe("saved");
  session.keep();
  expect(retainedDraft()).toBeNull();
  session.update([{ ...original[0], text: "" }]);
  await session.flush();
  expect(api.save).not.toHaveBeenCalled();
  session.composition(true);
  await session.flush();
});
test("reviewing a saved list pauses writes until the read is reconciled", async () => {
  const read = deferred();
  const api = {
    save: vi.fn(async (revision, items) => reply("1", items)),
    load: vi.fn().mockReturnValue(read.promise),
  };
  const { session } = start(api);
  session.update([{ ...session.rows[0], text: "First" }]);
  const reviewing = session.checkSaved();
  session.update([{ ...session.rows[0], text: "Second" }]);
  await vi.advanceTimersByTimeAsync(1500);
  await session.flush();
  expect(api.save).not.toHaveBeenCalled();
  read.resolve(base);
  await reviewing;
  expect(session.retryReady).toBe(true);
  await session.retry();
  expect(api.save).toHaveBeenCalledOnce();
  expect(api.save.mock.calls[0][1][0].text).toBe("Second");
});
