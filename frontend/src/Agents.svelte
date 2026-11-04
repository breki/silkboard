<script lang="ts">
  import { untrack } from "svelte";
  import AgentCard from "./AgentCard.svelte";
  import LinkError from "./LinkError.svelte";
  import {
    locationState,
    readView,
    updateQuery,
    filterFields,
  } from "./link-state";
  const address = locationState;
  const linked = $derived(readView("agents", $address.search));
  const expanded = $derived(linked.value.agentDetails);
  import { ApiError, type Page } from "./feed-api";
  import { agents, agentKey, type AgentActivity } from "./agents-api";
  import { poll } from "./poll";
  import ListFilters from "./ListFilters.svelte";
  import type { ListFilters as Filters } from "./list-filters";
  let filters = $state<Filters>({});
  function applyFilters(value: Filters) {
    updateQuery(filterFields(value));
  }
  let { onexpire }: { onexpire: () => void } = $props();
  let page = $state<Page<AgentActivity> | null>(null);
  let before = $state<string | null>(null);
  let failed = $state(false);
  let busy = $state(false);
  let retry = $state(0);
  let signature = "";
  $effect(() => {
    const request = linked;
    const key = JSON.stringify([
      request.valid,
      request.value.filters,
      request.value.before,
    ]);
    if (key === signature) return;
    signature = key;
    untrack(() => {
      filters = request.value.filters;
      before = request.value.before;
      page = null;
    });
  });
  let target = $state<AgentActivity | null>(null),
    targetUnavailable = $state(false);
  $effect(() => {
    const key = expanded;
    const present = page?.items.some((item) => agentKey(item) === key);
    void retry;
    target = null;
    targetUnavailable = false;
    if (!linked.valid || !key || !page || present) return;
    const [project, hostname, tool, session_id] = JSON.parse(key);
    const controller = new AbortController();
    void agents(null, controller.signal, {
      project,
      hostname,
      tool,
      session_id,
    })
      .then((result) => {
        if (!controller.signal.aborted) {
          target = result.items[0] ?? null;
          targetUnavailable = !target;
        }
      })
      .catch((error) => {
        if (controller.signal.aborted) return;
        if (error instanceof ApiError && error.status === 401) onexpire();
        else targetUnavailable = true;
      });
    return () => controller.abort();
  });
  const requestKey = $derived(
    JSON.stringify([linked.valid, linked.value.before, linked.value.filters]),
  );
  $effect(() => {
    const [valid, cursor, selected] = JSON.parse(requestKey);
    if (!valid) return;
    void retry;
    return poll(async (signal) => {
      busy = true;
      try {
        const result = await agents(cursor, signal, selected);
        if (signal.aborted) return;
        page = result;
        failed = false;
      } catch (error) {
        if (signal.aborted) return;
        if (error instanceof ApiError && error.status === 401) onexpire();
        else failed = true;
      } finally {
        if (!signal.aborted) busy = false;
      }
    });
  });
  function navigate(cursor: string | null) {
    updateQuery({ before: cursor });
  }
</script>

<section class="agents" aria-label="Agents">
  <ListFilters
    agentsView
    value={filters}
    disabled={!linked.valid}
    onapply={applyFilters}
    {onexpire}
  >
    {#snippet heading()}
      <h2>Agents</h2>
      <p class="muted">
        Latest received activity from your main agent sessions.
      </p>
    {/snippet}

    {#if !linked.valid}<LinkError />{/if}
    {#if target}<section aria-label="Linked main agent">
        <AgentCard
          item={target}
          expanded={true}
          onexpand={() => updateQuery({ agent_details: null })}
        />
      </section>{/if}
    {#if targetUnavailable}<p role="alert">
        The linked agent is unavailable. <button
          onclick={() => updateQuery({ agent_details: null })}
          >Clear linked agent</button
        ><button onclick={() => retry++}>Retry linked agent</button>
      </p>{/if}
    {#if failed}
      <div class="notice">
        <p role="alert">
          Unable to update agents. Displayed activity may be out of date.
        </p>
        <button onclick={() => retry++}>Retry agents</button>
      </div>
    {/if}
    {#if page}
      {#if !page.items.length}
        <p>
          {Object.values(filters).some(
            (value) => value !== "" && value !== undefined,
          )
            ? "No main agents match these filters."
            : before === null
              ? "No main agent reports have arrived yet."
              : "No main agents on this page. Return to the latest activity."}
        </p>
      {:else}
        <p class="result-count muted">
          {page.items.length} agents on this page · Latest received first
        </p>
        <ul class="agent-cards" aria-label="Main agents">
          {#each page.items as item (agentKey(item))}
            <li>
              <AgentCard
                {item}
                expanded={expanded === agentKey(item)}
                onexpand={(open) =>
                  updateQuery({ agent_details: open ? agentKey(item) : null })}
              />
            </li>
          {/each}
        </ul>
      {/if}
      <nav aria-label="Agent pages">
        {#if before !== null}<button
            disabled={busy}
            onclick={() => navigate(null)}>Latest agents</button
          >{/if}
        {#if page.next_cursor}<button
            disabled={busy}
            onclick={() => navigate(page!.next_cursor)}>Older agents</button
          >{/if}
      </nav>
      <p class="muted">
        Reported workflow states come from the linked snapshot; they are not
        connection status. Subagent events remain in Events. Agents with new
        activity move toward the latest page. Return to Latest agents while
        paging to see those updates.
      </p>
    {:else if !failed && linked.valid}
      <p role="status">Loading agents…</p>
    {/if}
  </ListFilters>
</section>

<style>
  .agent-cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 19rem), 1fr));
    gap: 0.75rem;
    list-style: none;
    padding: 0;
    margin-block: 1rem;
  }
  li {
    min-width: 0;
  }
  nav {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-block: 1rem;
  }
  .result-count {
    font-size: 0.75rem;
    margin: 0;
  }
</style>
