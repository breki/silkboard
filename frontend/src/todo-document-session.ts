import { ApiError } from "./feed-api";
import {
  loadTodos,
  saveTodoDocument,
  type Checklist,
  type DocumentEntry,
} from "./todos-api";
import { fromChecklist, serialize, type TaskRow } from "./todo-editor-model";
export type SavePhase = "saved" | "pending" | "saving" | "error" | "conflict";
interface Submission {
  revision: string;
  entries: DocumentEntry[];
  rows: TaskRow[];
}
interface Retained {
  base: Checklist;
  rows: TaskRow[];
  savedRows: TaskRow[];
  ids: [string, string][];
  submission: Submission | null;
  phase: SavePhase;
}
let retained: Retained | null = null;
export const retainedDraft = () => retained;
export const clearRetainedDraft = () => {
  retained = null;
};
export const hasRetainedDraft = () => retained !== null;
const copyRows = (rows: TaskRow[]) => rows.map((row) => ({ ...row }));
const equalRows = (a: TaskRow[], b: TaskRow[]) =>
  a.length === b.length &&
  a.every(
    (row, n) =>
      row.key === b[n].key &&
      row.text === b[n].text &&
      row.depth === b[n].depth &&
      row.completed === b[n].completed,
  );
const equalSaved = (a: Checklist, b: Checklist) =>
  a.revision === b.revision &&
  a.items.length === b.items.length &&
  a.items.every(
    (row, n) =>
      row.id === b.items[n].id &&
      row.parent_id === b.items[n].parent_id &&
      row.text === b.items[n].text &&
      row.completed === b.items[n].completed,
  );
interface Callbacks {
  // eslint-disable-next-line no-unused-vars -- Optional document validation boundary.
  validate?: (rows: TaskRow[]) => string;
  // eslint-disable-next-line no-unused-vars -- Callback argument documents the session boundary.
  change: (session: TodoDocumentSession) => void;
  expire: () => void;
  // eslint-disable-next-line no-unused-vars -- Callback argument documents the reset boundary.
  reset: (rows: TaskRow[]) => void;
}
interface API {
  save: typeof saveTodoDocument;
  load: typeof loadTodos;
}
export interface TodoLeaveGuard {
  confirm: () => boolean;
  discard: () => void;
  keep: () => void;
}
export class TodoDocumentSession {
  rows: TaskRow[];
  ids: Map<string, string>;
  phase: SavePhase = "saved";
  message = "";
  latest: Checklist | null = null;
  retryReady = false;
  reading = false;
  private base: Checklist;
  private savedRows: TaskRow[];
  private submission: Submission | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private inFlight = false;
  private composing = false;
  private disposed = false;
  private discarded = false;
  private version = 0;
  private controller = new AbortController();
  private callbacks: Callbacks;
  private api: API;
  constructor(
    data: Checklist,
    callbacks: Callbacks,
    api: API = { save: saveTodoDocument, load: loadTodos },
    memory: Retained | null = retainedDraft(),
  ) {
    this.callbacks = callbacks;
    this.api = api;
    const initial = fromChecklist(data);
    this.base = data;
    this.rows = initial.rows;
    this.savedRows = copyRows(this.rows);
    this.ids = initial.ids;
    if (memory) {
      this.base = memory.base;
      this.rows = copyRows(memory.rows);
      this.savedRows = copyRows(memory.savedRows);
      this.ids = new Map(memory.ids);
      this.submission = memory.submission;
      this.phase = memory.phase;
      this.reconcile(data);
    }
    this.emit();
    if (this.phase === "pending") this.schedule();
  }
  get dirty() {
    return !equalRows(this.rows, this.savedRows);
  }
  get validation() {
    return (
      serialize(this.rows, this.ids).error ??
      this.callbacks.validate?.(this.rows) ??
      ""
    );
  }
  revalidate() {
    this.emit();
    this.schedule();
  }
  update(rows: TaskRow[]) {
    this.rows = rows;
    this.version++;
    if (!["error", "conflict"].includes(this.phase) && !this.inFlight)
      this.phase = this.dirty ? "pending" : "saved";
    this.emit();
    this.schedule();
  }
  composition(value: boolean) {
    this.composing = value;
    this.cancelTimer();
    if (!value) this.schedule();
  }
  private emit() {
    if (!this.disposed) {
      if (this.phase === "saved" && !this.dirty) clearRetainedDraft();
      this.callbacks.change(this);
    }
  }
  private cancelTimer() {
    if (this.timer !== null) {
      clearTimeout(this.timer);
      this.timer = null;
    }
  }
  private schedule() {
    this.cancelTimer();
    if (
      this.disposed ||
      this.inFlight ||
      this.reading ||
      this.composing ||
      !this.dirty ||
      ["error", "conflict"].includes(this.phase) ||
      this.validation
    )
      return;
    this.timer = setTimeout(() => {
      this.timer = null;
      void this.submit();
    }, 600);
  }
  async flush() {
    this.cancelTimer();
    await this.submit();
  }
  private async submit() {
    if (
      this.disposed ||
      this.inFlight ||
      this.reading ||
      this.composing ||
      !this.dirty ||
      ["error", "conflict"].includes(this.phase)
    )
      return;
    const wire = serialize(this.rows, this.ids);
    if (wire.error || this.validation) {
      this.emit();
      return;
    }
    const submitted = {
      revision: this.base.revision,
      entries: wire.items,
      rows: copyRows(this.rows),
    };
    this.submission = submitted;
    this.inFlight = true;
    this.phase = "saving";
    this.message = "";
    this.emit();
    try {
      const saved = await this.api.save(
        submitted.revision,
        submitted.entries,
        this.controller.signal,
      );
      if (this.disposed) return;
      this.base = { revision: saved.revision, items: saved.items };
      this.ids = new Map(Object.entries(saved.keys));
      this.savedRows = submitted.rows;
      this.submission = null;
      this.phase = this.dirty ? "pending" : "saved";
    } catch (error) {
      if (!this.disposed) this.failed(error);
    } finally {
      this.inFlight = false;
      if (!this.disposed) {
        this.emit();
        this.schedule();
      }
    }
  }
  private failed(error: unknown) {
    this.retryReady = false;
    this.phase =
      error instanceof ApiError && error.status === 409 ? "conflict" : "error";
    this.message =
      this.phase === "conflict"
        ? "This list changed in another tab. Your edits are kept here."
        : "Couldn't confirm that these changes were saved. Your draft is kept in this tab.";
    if (error instanceof ApiError && error.status === 401) {
      this.keep();
      this.callbacks.expire();
    }
  }
  private matchesSubmission(data: Checklist) {
    const submitted = this.submission;
    if (
      !submitted ||
      BigInt(data.revision) !== BigInt(submitted.revision) + BigInt(1) ||
      data.items.length !== submitted.entries.length
    )
      return null;
    const ids = new Map(
      submitted.entries.map((entry, n) => [entry.key, data.items[n].id]),
    );
    const matches = submitted.entries.every((entry, n) => {
      const row = data.items[n];
      return (
        (entry.id === null || entry.id === row.id) &&
        entry.text === row.text &&
        entry.completed === row.completed &&
        row.parent_id ===
          (entry.parent_key === null ? null : ids.get(entry.parent_key))
      );
    });
    return matches ? ids : null;
  }
  private reconcile(data: Checklist) {
    const ids = this.matchesSubmission(data);
    if (ids) {
      this.base = data;
      this.ids = ids;
      this.savedRows = this.submission!.rows;
      this.submission = null;
      this.latest = null;
      this.retryReady = false;
      this.message = "";
      this.phase = this.dirty ? "pending" : "saved";
    } else if (equalSaved(data, this.base)) {
      this.latest = null;
      this.retryReady = this.dirty;
      this.phase = this.dirty ? "error" : "saved";
      this.message = this.dirty
        ? "The saved list is unchanged. Your draft is ready to retry."
        : "";
    } else {
      this.latest = data;
      this.retryReady = false;
      this.phase = "conflict";
      this.message =
        "Review your draft and the latest saved list. Your draft has not been replaced.";
    }
  }
  async checkSaved() {
    if (this.reading || this.inFlight || this.disposed) return;
    this.cancelTimer();
    this.reading = true;
    this.emit();
    try {
      const data = await this.api.load(this.controller.signal);
      if (this.disposed) return;
      this.reconcile(data);
    } catch (error) {
      if (!this.disposed) {
        this.failed(error);
        this.message =
          "Couldn't load the saved list. Your draft is still here.";
      }
    } finally {
      this.reading = false;
      this.emit();
      this.schedule();
    }
  }
  async retry() {
    if (!this.retryReady) return;
    this.retryReady = false;
    this.latest = null;
    this.message = "";
    this.phase = "pending";
    await this.submit();
  }
  async useSaved() {
    if (!this.latest || this.reading || this.disposed) return;
    const version = this.version;
    this.reading = true;
    this.emit();
    try {
      const data = await this.api.load(this.controller.signal);
      if (this.disposed) return;
      if (this.version !== version) {
        this.message =
          "Your draft changed while the saved list was loading. Review it again before replacing the draft.";
        return;
      }
      const loaded = fromChecklist(data);
      this.base = data;
      this.rows = loaded.rows;
      this.ids = loaded.ids;
      this.savedRows = copyRows(this.rows);
      this.submission = null;
      this.latest = null;
      this.retryReady = false;
      this.phase = "saved";
      this.message = "";
      this.version++;
      clearRetainedDraft();
      this.callbacks.reset(this.rows);
    } catch (error) {
      if (!this.disposed) this.failed(error);
    } finally {
      this.reading = false;
      this.emit();
    }
  }
  keep() {
    if (
      !this.discarded &&
      (this.dirty ||
        this.inFlight ||
        ["error", "conflict"].includes(this.phase))
    ) {
      retained = {
        base: this.base,
        rows: copyRows(this.rows),
        savedRows: copyRows(this.savedRows),
        ids: [...this.ids],
        submission: this.submission,
        phase: this.phase,
      };
    }
  }
  discard() {
    this.discarded = true;
    clearRetainedDraft();
  }
  dispose() {
    this.cancelTimer();
    this.disposed = true;
    this.controller.abort();
  }
}
