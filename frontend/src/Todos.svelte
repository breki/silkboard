<script lang="ts">
  import { locationState, readView, updateQuery } from "./link-state";
  const address = locationState;
  const linked = $derived(readView("todos", $address.search));
  import { onMount, tick, untrack } from "svelte";
  import { ApiError } from "./feed-api";
  import {
    loadTodos,
    changeTodo,
    taskText,
    type Checklist,
    type Todo,
    type TodoChange,
  } from "./todos-api";
  import TodoList from "./TodoList.svelte";
  let { onexpire }: { onexpire: () => void } = $props();
  let data = $state<Checklist | null>(null);
  let busy = $state(false),
    uncertain = $state(false),
    notice = $state("");
  let draft = $state(""),
    parent = $state<Todo | null>(null),
    editing = $state<Todo | null>(null),
    removing = $state<Todo | null>(null);
  let input = $state<HTMLInputElement>(),
    editInput = $state<HTMLInputElement>(),
    confirmButton = $state<HTMLButtonElement>();
  let editText = $state("");
  const controller = new AbortController();
  async function reload() {
    busy = true;
    notice = "";
    try {
      const value = await loadTodos(controller.signal);
      if (controller.signal.aborted) return;
      data = value;
      uncertain = false;
      if (editing && !value.items.some((item) => item.id === editing!.id)) {
        notice =
          "The edited todo was deleted elsewhere. Your edit text is kept; copy it before cancelling.";
      }
      if (parent && !value.items.some((item) => item.id === parent!.id)) {
        parent = null;
        notice =
          "The selected parent was deleted. Your draft is now a top-level todo.";
      }
      removing = null;
    } catch (error) {
      if (!controller.signal.aborted) failed(error, false);
    } finally {
      if (!controller.signal.aborted) busy = false;
    }
  }
  function failed(error: unknown, mutation: boolean) {
    if (error instanceof ApiError && error.status === 401) {
      onexpire();
      return;
    }
    if (mutation) uncertain = true;
    notice = mutation
      ? error instanceof ApiError && error.status === 409
        ? "The list changed in another tab. Your changes were not saved. Reload todos before trying again."
        : "Unable to confirm this change was saved. Your draft and last loaded todos are kept. Reload todos and check the result before trying again."
      : "Unable to load todos. Existing rows and drafts are kept. Try Reload todos.";
  }
  function selectionKey() {
    const state = readView("todos", window.location.search).value;
    return JSON.stringify([state.todo, state.todoMode]);
  }
  async function save(
    change: TodoChange,
    finishEdit = false,
    returnFocus?: HTMLElement,
  ) {
    if (!data || busy || uncertain) return;
    const submittedSelection = selectionKey();
    busy = true;
    notice = "";
    try {
      const value = await changeTodo(data.revision, change, controller.signal);
      if (controller.signal.aborted) return;
      data = value;
      const sameSelection = selectionKey() === submittedSelection;
      if (sameSelection && change.kind === "add") {
        draft = "";
        parent = null;
      } else if (
        sameSelection &&
        finishEdit &&
        change.kind === "edit" &&
        editing?.id === change.id
      )
        editing = null;
      if (sameSelection) removing = null;
      if (
        sameSelection &&
        (change.kind === "add" ||
          finishEdit ||
          (change.kind === "delete" &&
            readView("todos", window.location.search).value.todo === change.id))
      )
        updateQuery({ todo: null, mode: null }, true);
      busy = false;
      await tick();
      if (sameSelection)
        (returnFocus?.isConnected ? returnFocus : input)?.focus();
    } catch (error) {
      if (!controller.signal.aborted) failed(error, true);
    } finally {
      if (!controller.signal.aborted) busy = false;
    }
  }
  async function edit(item: Todo) {
    updateQuery({ todo: item.id, mode: "edit" });
    editing = item;
    editText = item.text;
    removing = null;
    await tick();
    editInput?.focus();
  }
  async function child(item: Todo) {
    updateQuery({ todo: item.id, mode: "child" });
    parent = item;
    removing = null;
    await tick();
    input?.focus();
  }
  async function remove(item: Todo) {
    removing = item;
    await tick();
    confirmButton?.focus();
  }
  let selection = "",
    unavailable = $state(false);
  $effect(() => {
    const state = linked.value,
      loaded = data;
    if (!loaded) return;
    const key = JSON.stringify([state.todo, state.todoMode]);
    if (key === selection) return;
    selection = key;
    untrack(() => {
      editing = null;
      parent = null;
      removing = null;
      unavailable = false;
      if (!state.todo) return;
      const item = loaded.items.find((todo) => todo.id === state.todo);
      if (!item) {
        unavailable = true;
        return;
      }
      if (state.todoMode === "edit") void edit(item);
      else if (state.todoMode === "child") {
        let depth = 1,
          ancestor = item.parent_id;
        while (ancestor) {
          depth++;
          ancestor =
            loaded.items.find((todo) => todo.id === ancestor)?.parent_id ??
            null;
        }
        if (depth >= 5) {
          unavailable = true;
          return;
        }
        void child(item);
      } else
        void tick().then(() =>
          document
            .getElementById("todo-" + item.id)
            ?.scrollIntoView({ block: "nearest" }),
        );
    });
  });
  onMount(() => {
    void reload();
    return () => controller.abort();
  });
</script>

<section aria-label="Todos">
  <h2>Todos</h2>
  <p class="muted">
    Your private checklist. Checkboxes are independent. Children are indented;
    each level keeps creation order. Up to five levels and 1,000 todos.
  </p>
  <button class="quiet" disabled={busy} onclick={() => void reload()}
    >Reload todos</button
  >
  {#if unavailable || (!notice && linked.value.todo && data && !data.items.some((item) => item.id === linked.value.todo))}<p
      role="alert"
    >
      The linked todo is unavailable for this view. <button
        class="quiet"
        onclick={() => updateQuery({ todo: null, mode: null })}
        >Clear todo selection</button
      >
    </p>{/if}
  {#if notice}<p role="alert">{notice}</p>{/if}
  {#if busy}<p role="status">
      {data ? "Saving or loading todos…" : "Loading todos…"}
    </p>{/if}
  {#if data}
    <form
      onsubmit={(event) => {
        event.preventDefault();
        void save({
          kind: "add",
          parent_id: parent?.id ?? null,
          text: draft.trim(),
        });
      }}
    >
      <label for="todo-new"
        >{parent ? `New child of ${parent.text}` : "New todo"}</label
      >
      <input
        id="todo-new"
        bind:this={input}
        bind:value={draft}
        disabled={busy}
        maxlength="1000"
        required
      />
      {#if draft && !taskText(draft.trim())}<p>
          Use a non-empty single line of at most 1,000 UTF-8 bytes.
        </p>{/if}
      <button
        class="primary"
        disabled={busy || uncertain || !taskText(draft.trim())}>Add todo</button
      >
      {#if parent}<button
          type="button"
          class="quiet"
          disabled={busy}
          onclick={() => {
            parent = null;
            updateQuery({ todo: null, mode: null });
          }}>Use top level</button
        >{/if}
    </form>
    {#if editing}
      <form
        aria-label={`Edit ${editing.text}`}
        onsubmit={(event) => {
          event.preventDefault();
          const target = data!.items.find((item) => item.id === editing!.id);
          if (!target) return;
          void save(
            {
              kind: "edit",
              id: editing!.id,
              text: editText.trim(),
              completed: target.completed,
            },
            true,
          );
        }}
      >
        <label for="todo-edit">Edit todo text</label><input
          id="todo-edit"
          bind:this={editInput}
          bind:value={editText}
          disabled={busy}
          maxlength="1000"
          required
        />
        {#if editText && !taskText(editText.trim())}<p>
            Use a non-empty single line of at most 1,000 UTF-8 bytes.
          </p>{/if}
        <button
          class="primary"
          disabled={busy ||
            uncertain ||
            !taskText(editText.trim()) ||
            !data.items.some((item) => item.id === editing!.id)}
          >Save text</button
        >
        <button
          type="button"
          disabled={busy}
          onclick={() => {
            editing = null;
            updateQuery({ todo: null, mode: null });
          }}>Cancel edit</button
        >
      </form>
    {/if}
    {#if removing}
      <section class="panel" aria-label="Confirm todo deletion">
        <h3>Delete todo?</h3>
        <p>{removing.text}</p>
        <p>
          {data.items.some((item) => item.parent_id === removing!.id)
            ? "Its children will be kept and moved up one level, keeping their own children and checkboxes. This deletion cannot be undone."
            : "This deletion cannot be undone."}
        </p>
        <button
          bind:this={confirmButton}
          class="danger"
          disabled={busy || uncertain}
          onclick={() => void save({ kind: "delete", id: removing!.id })}
          >Confirm delete</button
        >
        <button disabled={busy} onclick={() => (removing = null)}
          >Cancel delete</button
        >
      </section>
    {/if}
    {#if !data.items.length}<p>No todos yet. Add your first todo above.</p>{/if}
    <TodoList
      items={data.items}
      selected={linked.value.todo}
      disabled={busy || uncertain}
      onedit={(item) => void edit(item)}
      onchild={(item) => void child(item)}
      ondelete={(item) => void remove(item)}
      oncheck={(item, checkbox) =>
        void save(
          {
            kind: "edit",
            id: item.id,
            text: item.text,
            completed: !item.completed,
          },
          false,
          checkbox,
        )}
    />
  {/if}
</section>

<style>
  form {
    margin-block: 1rem;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
  }
  label {
    width: 100%;
    overflow-wrap: anywhere;
  }
  input {
    min-width: 0;
    flex: 1 1 12rem;
  }
  p {
    overflow-wrap: anywhere;
  }
</style>
