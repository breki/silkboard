<script lang="ts">
  import { agents, agentKey, type AgentActivity } from "./agents-api";
  import { ApiError } from "./feed-api";
  import Modal from "./Modal.svelte";
  let {
    project,
    value,
    onselect,
    onexpire,
  }: {
    project: string;
    value: AgentActivity | null;
    // eslint-disable-next-line no-unused-vars -- Callback parameter describes the contract.
    onselect: (value: AgentActivity | null) => void;
    onexpire: () => void;
  } = $props();
  let open = $state(false),
    search = $state(""),
    busy = $state(false),
    failed = $state(false),
    retry = $state(0);
  let before = $state<string | null>(null),
    next = $state<string | null>(null),
    items = $state<AgentActivity[]>([]);
  let trigger = $state<HTMLButtonElement>();
  const title = (item: AgentActivity) =>
    item.session_name ??
    (item.latest.kind === "workflow"
      ? item.latest.title
      : `${item.tool} on ${item.hostname}`);
  $effect(() => {
    if (!open) return;
    const scope = project,
      query = search.trim(),
      cursor = before;
    void retry;
    const controller = new AbortController();
    if (cursor === null) items = [];
    busy = true;
    failed = false;
    next = null;
    const timer = setTimeout(async () => {
      try {
        const page = await agents(cursor, controller.signal, {
          project: scope,
          q: query,
        });
        if (controller.signal.aborted) return;
        items =
          cursor === null
            ? page.items
            : [
                ...new Map(
                  [...items, ...page.items].map((item) => [
                    agentKey(item),
                    item,
                  ]),
                ).values(),
              ];
        next = page.next_cursor;
      } catch (error) {
        if (controller.signal.aborted) return;
        if (error instanceof ApiError && error.status === 401) onexpire();
        else failed = true;
      } finally {
        if (!controller.signal.aborted) busy = false;
      }
    }, 200);
    return () => {
      clearTimeout(timer);
      controller.abort();
    };
  });
  function close() {
    open = false;
    trigger?.focus();
  }
  function choose(item: AgentActivity | null) {
    onselect(item);
    close();
  }
</script>

<button
  class="agent-trigger"
  type="button"
  bind:this={trigger}
  aria-label={`Agent: ${value ? title(value) : "All agents"}`}
  aria-haspopup="dialog"
  onclick={() => {
    search = "";
    before = null;
    open = true;
  }}
  ><span class="muted">Agent</span>
  {value ? title(value) : "All agents"}<span aria-hidden="true">⌄</span></button
>
<Modal {open} title="Choose agent" onclose={close}>
  <label class="picker-search"
    >Search agents<input
      data-initial-focus
      value={search}
      oninput={(event) => {
        search = event.currentTarget.value;
        before = null;
      }}
      maxlength="513"
      placeholder="Project, task, tool or host"
    /></label
  >
  <button
    class="quiet agent-option"
    type="button"
    aria-pressed={value === null}
    onclick={() => choose(null)}>All agents</button
  >
  <ul class="agent-options" aria-label="Available agents">
    {#each items as item (agentKey(item))}<li>
        <button
          class="quiet agent-option"
          type="button"
          aria-pressed={value !== null && agentKey(value) === agentKey(item)}
          onclick={() => choose(item)}
          ><span>{title(item)}</span><small
            >{item.project} · {item.tool} / {item.hostname}</small
          ><small class="native-session">{item.session_id}</small></button
        >
      </li>{/each}
  </ul>
  {#if busy}<p role="status">Loading agents…</p>{:else if failed}<p
      role="alert"
    >
      Unable to load agents.
    </p>
    <button type="button" onclick={() => retry++}>Retry agent choices</button
    >{:else if !items.length}<p class="muted">
      No matching main agents. Try a project, task, tool or host.
    </p>{/if}
  {#if next}<button
      type="button"
      disabled={busy}
      onclick={() => (before = next)}>More agents</button
    >{/if}
</Modal>

<style>
  .agent-trigger {
    gap: 0.6rem;
    justify-content: space-between;
    text-align: left;
  }
  .picker-search {
    font-size: 0.8rem;
    color: var(--muted);
    margin: 1.25rem 0 0.75rem;
  }
  input {
    margin-top: 0.5rem;
  }
  .agent-options {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  .agent-option {
    display: flex;
    align-items: start;
    flex-direction: column;
    width: 100%;
    text-align: left;
    justify-content: start;
    padding: 0.75rem;
  }
  small {
    color: var(--muted);
    font-size: 0.75rem;
  }
  .native-session {
    overflow-wrap: anywhere;
    font-family: ui-monospace, monospace;
  }
</style>
