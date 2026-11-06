<script lang="ts">
  import { onMount, tick } from "svelte";
  import {
    locationState,
    readView,
    updateQuery,
    featureLink,
  } from "./link-state";
  import { ApiError } from "./feed-api";
  import { loadTodos } from "./todos-api";
  import { createTodoEditor, type TodoEditor } from "./todo-editor-view";
  import {
    TodoDocumentSession,
    retainedDraft,
    type TodoLeaveGuard,
    type SavePhase,
  } from "./todo-document-session";
  import { draftText, fromChecklist, type TaskRow } from "./todo-editor-model";
  import "./todo-editor.css";
  // eslint-disable-next-line no-unused-vars -- Registration callback describes the owner lifecycle boundary.
  type Register = (guard: TodoLeaveGuard | null) => void;
  let {
    onexpire,
    onregister = () => {},
  }: { onexpire: () => void; onregister?: Register } = $props();
  const address = locationState;
  const linked = $derived(readView("todos", $address.search));
  let host: HTMLDivElement;
  let help: HTMLDialogElement;
  let footerHelp: HTMLButtonElement;
  let more: HTMLDetailsElement;
  let copyField = $state<HTMLTextAreaElement>();
  let editor: TodoEditor | null = null;
  let session: TodoDocumentSession | null = null;
  let rows = $state<TaskRow[]>([]),
    phase = $state<SavePhase>("saved"),
    message = $state("");
  let loading = $state(true),
    loadError = $state(""),
    validation = $state(""),
    reading = $state(false),
    retryReady = $state(false);
  let latestText = $state(""),
    selected = $state(""),
    selectedId = $state<string | null>(null);
  let canUndo = $state(false),
    canRedo = $state(false),
    editing = $state(false),
    hint = $state(""),
    copiedDraft = $state("");
  let ready = $state(false),
    unavailable = $state(false);
  let appliedLink = "",
    alive = true;
  const controller = new AbortController();
  const count = $derived(rows.filter((row) => row.text.trim()).length);
  const completed = $derived(
    rows.filter((row) => row.text.trim() && row.completed).length,
  );
  function sync(value: TodoDocumentSession) {
    rows = value.rows;
    phase = value.phase;
    message = value.message;
    validation = value.validation;
    reading = value.reading;
    retryReady = value.retryReady;
    latestText = value.latest
      ? value.latest.items.length
        ? draftText(fromChecklist(value.latest).rows)
        : "(Empty list)"
      : "";
    selectedId = value.ids.get(selected) ?? null;
    if (editor?.view.hasFocus()) publishSelection();
  }
  function publishSelection() {
    const id = session?.ids.get(selected);
    if (!id) return;
    const value = readView("todos", window.location.search).value;
    if (value.todo === id && value.todoMode === "view") return;
    appliedLink = JSON.stringify([id, "view"]);
    updateQuery({ todo: id, mode: null }, true);
  }
  function reveal() {
    if (
      !editor?.view.hasFocus() ||
      typeof document.createRange().getClientRects !== "function"
    )
      return;
    const rect = editor.view.coordsAtPos(editor.view.state.selection.head);
    const tools = footerHelp?.closest(".todo-tools")?.getBoundingClientRect();
    if (
      window.matchMedia("(max-width: 650px)").matches &&
      tools &&
      rect.bottom > tools.top - 12
    )
      window.scrollBy(0, rect.bottom - tools.top + 12);
  }
  function mountEditor(value: TaskRow[]) {
    editor?.destroy();
    editor = createTodoEditor(host, value, {
      change: (next) => session?.update(next),
      selection: (key) => {
        selected = key;
        selectedId = session?.ids.get(key) ?? null;
        if (editor?.view.hasFocus()) publishSelection();
      },
      history: (undo, redo) => {
        canUndo = undo;
        canRedo = redo;
      },
      hint: (value) => {
        hint = value;
      },
      focus: () => {
        editing = true;
      },
      composing: (value) => session?.composition(value),
      leave: finish,
      reveal,
    });
    selected = value[0].key;
    selectedId = session?.ids.get(selected) ?? null;
    canUndo = false;
    canRedo = false;
  }
  async function load() {
    loading = true;
    loadError = "";
    try {
      const data = await loadTodos(controller.signal);
      if (!alive) return;
      session = new TodoDocumentSession(data, {
        change: sync,
        expire: () => {
          session?.keep();
          onexpire();
        },
        reset: (value) => {
          mountEditor(value);
          appliedLink = "";
        },
      });
      await tick();
      if (!alive) return;
      mountEditor(session.rows);
      sync(session);
      ready = true;
      onregister({
        confirm: () =>
          !session?.dirty ||
          window.confirm(
            "Your unsaved Todos draft will be discarded when you sign out. Cancel to keep editing or copy the draft first.",
          ),
        discard: () => session?.discard(),
        keep: () => session?.keep(),
      });
    } catch (error) {
      if (!alive) return;
      if (error instanceof ApiError && error.status === 401) onexpire();
      else
        loadError = retainedDraft()
          ? "Unable to load Todos. Your earlier draft is kept in this tab; try again to recover it."
          : "Unable to load Todos. Try again.";
    } finally {
      if (alive) loading = false;
    }
  }
  function finish() {
    editing = false;
    editor?.view.dom.blur();
    footerHelp?.focus();
    void session?.flush();
  }
  function action(value: "new" | "indent" | "outdent" | "undo" | "redo") {
    if (!editor) return;
    hint = "";
    if (value === "undo") editor.undo();
    else if (value === "redo") editor.redo();
    else editor.action(value);
    editor.focus();
    editing = true;
  }
  function toolbarKeys(event: KeyboardEvent) {
    if (
      !(event.target instanceof HTMLButtonElement) ||
      !["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)
    )
      return;
    const buttons = [
      ...(
        event.currentTarget as HTMLElement
      ).querySelectorAll<HTMLButtonElement>("button"),
    ].filter(
      (button) =>
        !button.disabled && getComputedStyle(button).display !== "none",
    );
    if (!buttons.length) return;
    const index = buttons.indexOf(event.target);
    const next =
      event.key === "Home"
        ? 0
        : event.key === "End"
          ? buttons.length - 1
          : (index + (event.key === "ArrowRight" ? 1 : -1) + buttons.length) %
            buttons.length;
    event.preventDefault();
    buttons[next].focus();
  }
  function keyboardHelp() {
    help.showModal();
  }
  async function copyDraft() {
    const text = draftText(rows);
    try {
      await navigator.clipboard.writeText(text);
      hint = "Draft copied.";
    } catch {
      copiedDraft = text;
      await tick();
      copyField?.focus();
      copyField?.select();
      hint = "Select and copy the draft below.";
    }
  }
  async function copyLink() {
    if (!selectedId) return;
    const url = new URL(
      featureLink(`/todos?todo=${selectedId}`),
      window.location.origin,
    ).href;
    try {
      await navigator.clipboard.writeText(url);
      hint = "Task link copied.";
    } catch {
      hint = "Copy the selected task's link from the address bar.";
    }
    more.open = false;
  }
  async function useSaved() {
    if (
      session?.dirty &&
      !window.confirm(
        "Replace the draft in this tab with the saved list? Copy your draft first if you want to keep it.",
      )
    )
      return;
    await session?.useSaved();
  }
  function beforeUnload(event: BeforeUnloadEvent) {
    if (session?.dirty || ["error", "conflict", "saving"].includes(phase)) {
      event.preventDefault();
      event.returnValue = "";
    }
  }
  function viewport() {
    const visual = window.visualViewport;
    const inset = visual
      ? Math.max(0, window.innerHeight - visual.height - visual.offsetTop)
      : 0;
    host
      ?.closest<HTMLElement>(".todos-editor")
      ?.style.setProperty("--keyboard-inset", `${inset}px`);
    requestAnimationFrame(() => reveal());
  }
  $effect(() => {
    const target = linked.value;
    if (!ready || !editor || !session) return;
    const key = JSON.stringify([target.todo ?? null, target.todoMode ?? null]);
    if (key === appliedLink) return;
    appliedLink = key;
    unavailable = false;
    if (!target.todo) return;
    const local = [...session.ids].find(([, id]) => id === target.todo)?.[0];
    const row = session.rows.find((row) => row.key === local);
    if (!row) {
      unavailable = true;
      return;
    }
    editor.focus(row.key);
    if (target.todoMode === "child") {
      if (row.depth >= 5 || session.rows.length >= 1000) {
        unavailable = true;
        return;
      }
      editor.action("new");
      editor.action("indent");
    }
  });
  onMount(() => {
    void load();
    window.addEventListener("beforeunload", beforeUnload);
    window.visualViewport?.addEventListener("resize", viewport);
    window.visualViewport?.addEventListener("scroll", viewport);
    return () => {
      alive = false;
      session?.keep();
      session?.dispose();
      editor?.destroy();
      controller.abort();
      onregister(null);
      window.removeEventListener("beforeunload", beforeUnload);
      window.visualViewport?.removeEventListener("resize", viewport);
      window.visualViewport?.removeEventListener("scroll", viewport);
    };
  });
</script>

<section class="todos-editor" aria-label="Todos">
  <div class="todo-heading">
    <div>
      <h2>Todos</h2>
      <p class="muted">Your private task list.</p>
    </div>
    <div class="todo-heading-actions">
      <button class="quiet todo-desktop-help" onclick={keyboardHelp}
        >Keyboard help</button
      >
      <details bind:this={more} class="todo-more">
        <summary aria-label="More todo actions">•••</summary>
        <div>
          <button
            class="quiet"
            disabled={!selectedId}
            onclick={() => void copyLink()}>Copy selected task link</button
          ><button
            class="quiet"
            disabled={!ready || reading}
            onclick={() => {
              more.open = false;
              void session?.checkSaved();
            }}>Review saved list</button
          >
        </div>
      </details>
      <button
        class="primary todo-phone-done"
        disabled={!ready}
        onclick={() => (editing ? finish() : editor?.focus())}
        >{editing ? "Done" : "Edit list"}</button
      >
    </div>
  </div>
  <p class="todo-phone-instruction muted">
    Tap task text to write. Check a box to complete it.
  </p>
  <span id="todo-editor-instructions" class="todo-sr-only"
    >Edit task text directly. Enter splits a task. Tab indents and Shift+Tab
    outdents while editing text. Escape leaves editing; then Tab reaches other
    controls. Arrow keys follow text and wrapped lines. Checkboxes use ordinary
    Tab and Space.</span
  >
  {#if loading}<p role="status">Loading todos…</p>{/if}
  {#if loadError}<p role="alert">
      {loadError}
      <button class="quiet" onclick={() => void load()}>Try again</button>
    </p>{/if}
  {#if unavailable}<p role="alert">
      The linked todo is unavailable for this view. <button
        class="quiet"
        onclick={() => updateQuery({ todo: null, mode: null })}
        >Clear todo selection</button
      >
    </p>{/if}
  <div class="todo-document" class:todo-loading={!ready}>
    <div class="todo-document-head">
      <span
        >{count}
        {count === 1 ? "task" : "tasks"} <span aria-hidden="true">/</span>
        {completed} completed</span
      ><span class="todo-save-state" role="status"
        >{phase === "saving"
          ? "Saving…"
          : phase === "saved"
            ? "All changes saved"
            : phase === "error" || phase === "conflict"
              ? "Changes unconfirmed"
              : "Unsaved changes"}</span
      >
    </div>
    {#if message}<div class="todo-save-error" role="alert">
        <strong>{message}</strong>
        <p>
          Your edits stay in this tab. Check the saved version before continuing
          to save.
        </p>
        <div class="todo-recovery-actions">
          <button disabled={reading} onclick={() => void session?.checkSaved()}
            >{reading ? "Checking…" : "Review saved list"}</button
          ><button class="quiet" onclick={() => void copyDraft()}
            >Copy my draft</button
          >{#if retryReady}<button
              class="primary"
              onclick={() => void session?.retry()}>Retry save</button
            >{/if}
        </div>
        {#if latestText}<div class="todo-version-comparison">
            <label
              >Your draft<textarea
                readonly
                aria-label="Your draft"
                value={draftText(rows)}></textarea></label
            ><label
              >Latest saved list<textarea
                readonly
                aria-label="Latest saved list"
                value={latestText}></textarea></label
            >
          </div>
          <button disabled={reading} onclick={() => void useSaved()}
            >Use saved list</button
          >{/if}
      </div>{/if}
    <div class="todo-editor-host" bind:this={host}></div>
    <div
      class="todo-tools"
      role="toolbar"
      tabindex="-1"
      onkeydown={toolbarKeys}
      aria-label="Task editing tools"
      onpointerdown={(event) => {
        if (event.target instanceof Element && event.target.closest("button"))
          event.preventDefault();
      }}
    >
      <button
        class="quiet todo-new-tool"
        disabled={!ready}
        onclick={() => action("new")}
        ><span aria-hidden="true">＋</span><span>New task</span></button
      >
      <button class="quiet" disabled={!ready} onclick={() => action("outdent")}
        ><span aria-hidden="true">⇤</span><span>Outdent</span></button
      >
      <button class="quiet" disabled={!ready} onclick={() => action("indent")}
        ><span aria-hidden="true">⇥</span><span>Indent</span></button
      >
      <span class="todo-tool-divider" aria-hidden="true"></span>
      <button class="quiet" disabled={!canUndo} onclick={() => action("undo")}
        ><span aria-hidden="true">↶</span><span>Undo</span></button
      >
      <button
        class="quiet todo-redo-tool"
        disabled={!canRedo}
        onclick={() => action("redo")}
        ><span aria-hidden="true">↷</span><span>Redo</span></button
      >
      <button
        class="quiet todo-footer-help"
        bind:this={footerHelp}
        onclick={keyboardHelp}
        ><span aria-hidden="true">?</span><span>Help</span></button
      >
      <span class="todo-shortcuts"
        ><kbd>Enter</kbd> new task · <kbd>Tab</kbd> indent · <kbd>Esc</kbd> leave
        editor</span
      >
    </div>
  </div>
  {#if validation}<p class="todo-validation" role="status">{validation}</p>{/if}
  {#if hint}<p class="todo-hint" role="status">{hint}</p>{/if}
  {#if copiedDraft}<label class="todo-copy-draft"
      >Copy your draft<textarea
        bind:this={copyField}
        readonly
        aria-label="Copy your draft"
        value={copiedDraft}></textarea></label
    >{/if}
  <p class="muted todo-below">
    Checkboxes stay independent. Up to five levels and 1,000 tasks.
  </p>
</section>
<dialog class="todo-help" bind:this={help} aria-labelledby="todo-help-title">
  <div class="todo-help-heading">
    <h3 id="todo-help-title">Writing your list</h3>
    <button
      class="quiet"
      aria-label="Close keyboard help"
      onclick={() => help.close()}>×</button
    >
  </div>
  <p>One document. Type where you are.</p>
  <dl>
    <div>
      <dt>Split into a new task</dt>
      <dd><kbd>Enter</kbd></dd>
    </div>
    <div>
      <dt>Join adjacent tasks at the edge</dt>
      <dd><kbd>Backspace</kbd> / <kbd>Delete</kbd></dd>
    </div>
    <div>
      <dt>Indent task and its children</dt>
      <dd><kbd>Tab</kbd></dd>
    </div>
    <div>
      <dt>Outdent task and its children</dt>
      <dd><kbd>Shift+Tab</kbd></dd>
    </div>
    <div>
      <dt>Undo / redo</dt>
      <dd><kbd>Ctrl/⌘ Z</kbd> / <kbd>Ctrl/⌘ Shift Z</kbd></dd>
    </div>
    <div>
      <dt>Leave editing, then reach controls</dt>
      <dd><kbd>Esc</kbd>, then <kbd>Tab</kbd></dd>
    </div>
  </dl>
  <p>
    Arrow keys follow text, including wrapped lines. On checkboxes and controls,
    Tab moves focus normally. New tasks start unchecked. Deleting a parent
    promotes its children. Undo restores text and structure while this editor
    stays open.
  </p>
  <p>
    Changes save automatically after typing pauses. Empty or invalid tasks stay
    as drafts until finished or removed. Save failures keep the draft in this
    tab; leaving or reloading this page, or explicitly discarding the draft, can
    lose unsaved text.
  </p>
</dialog>
