<script lang="ts">
  import type { SessionGroup } from "./event-hierarchy";
  import { SvelteSet } from "svelte/reactivity";
  import EventRows from "./EventRows.svelte";
  import SessionRows from "./SessionRows.svelte";
  import { eventKey as key, groupEvents, sessionKey } from "./event-hierarchy";
  let hierarchical = $state(false);
  const collapsed = new SvelteSet<string>();
  function toggleGroup(id: string) {
    if (collapsed.has(id)) collapsed.delete(id);
    else collapsed.add(id);
  }
  import ListFilters from "./ListFilters.svelte";
  import type { ListFilters as Filters } from "./list-filters";
  let filters = $state<Filters>({});
  function applyFilters(value: Filters) {
    navigate(null);
    filters = value;
  }
  import { ApiError, events, type Page, type FeedEvent } from "./feed-api";
  import { poll } from "./poll";
  let { onexpire }: { onexpire: () => void } = $props();
  let page = $state<Page<FeedEvent> | null>(null);
  const hierarchy = $derived(groupEvents(page?.items ?? []));
  // One keyed sequence retains child controls when a parent enters this page.
  const sessionGroups = $derived.by(() => {
    const entries: { group: SessionGroup; parent: SessionGroup | null }[] = [];
    for (const group of hierarchy.roots) {
      entries.push({ group, parent: null });
      for (const child of group.children)
        entries.push({ group: child, parent: group });
    }
    for (const group of hierarchy.unmatched)
      entries.push({ group, parent: null });
    return entries;
  });
  let before = $state<string | null>(null);
  let failed = $state(false);
  let busy = $state(false);
  let retry = $state(0);
  let expanded = $state<string | null>(null);
  $effect(() => {
    const cursor = before;
    const selected = filters;
    void retry;
    return poll(async (signal) => {
      busy = true;
      try {
        const value = await events(cursor, signal, selected);
        if (signal.aborted) return;
        page = value;
        const visible = new Set(value.items.map(sessionKey));
        for (const id of collapsed) {
          if (!visible.has(id)) collapsed.delete(id);
        }
        failed = false;
        if (
          expanded !== null &&
          !value.items.some((event) => key(event) === expanded)
        )
          expanded = null;
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
    page = null;
    collapsed.clear();
    expanded = null;
    before = cursor;
  }
  function toggle(event: FeedEvent) {
    expanded = expanded === key(event) ? null : key(event);
  }
</script>

<div class="event-heading">
  <h3>Events</h3>
  <p class="muted">
    All projects · Updates every 10 seconds while this page is visible.
  </p>
</div>
<div class="event-view-switch" role="group" aria-label="Events view">
  <button aria-pressed={!hierarchical} onclick={() => (hierarchical = false)}
    >Flat list</button
  >
  <button aria-pressed={hierarchical} onclick={() => (hierarchical = true)}
    >Session hierarchy</button
  >
</div>
<ListFilters onapply={applyFilters} />
{#if hierarchical}<p class="muted">
    Relationships use this page only. Parent-session groups come first, ordered
    by their newest visible family event; unmatched subagents follow. Events
    within each session retain receipt order. Missing parents are not loaded.
  </p>{/if}
{#if failed}
  <div class="notice">
    <p role="alert">
      Unable to update events. Displayed events may be out of date.
    </p>
    <button onclick={() => retry++}>Retry events</button>
  </div>
{/if}
{#if page}
  {#if !page.items.length}
    <p class="empty">
      {Object.values(filters).some(
        (value) => value !== undefined && value !== "",
      )
        ? "No events match these filters on this page."
        : "No events on this page. New reports appear on the latest page."}
    </p>
  {:else}
    <table class="events-table" aria-label="Events from all projects">
      <colgroup
        ><col class="event-time" /><col class="event-project" /><col
          class="event-agent"
        /><col class="event-summary" /></colgroup
      >
      <thead
        ><tr
          ><th scope="col">Time</th><th scope="col">Project</th><th scope="col"
            >Agent</th
          ><th scope="col">Message</th></tr
        ></thead
      >
      <tbody>
        {#if hierarchical}
          {#each sessionGroups as { group, parent } (group.key)}
            {#if parent === null || !collapsed.has(parent.key)}
              <SessionRows
                {group}
                {collapsed}
                {expanded}
                nested={parent !== null}
                parentName={parent?.name ?? ""}
                unmatched={parent === null &&
                  group.identity.subagent_id !== null}
                ontogglegroup={toggleGroup}
                ontoggleevent={toggle}
              />
            {/if}
          {/each}
        {:else}
          <EventRows
            items={page.items.map((event, index) => ({ event, index }))}
            {expanded}
            ontoggle={toggle}
          />
        {/if}
      </tbody>
    </table>
  {/if}
  <div class="page-controls">
    {#if before !== null}<button disabled={busy} onclick={() => navigate(null)}
        >Latest events</button
      >{/if}
    {#if page.next_cursor}<button
        disabled={busy}
        onclick={() => navigate(page!.next_cursor)}>Earlier events</button
      >{/if}
  </div>
{:else if !failed}<p role="status">Loading events…</p>{/if}

<style>
  .event-view-switch {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-block: 0.75rem;
  }
  .event-view-switch button[aria-pressed="true"] {
    background: var(--primary);
    color: var(--primary-text);
  }
</style>
