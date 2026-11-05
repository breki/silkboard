<script lang="ts">
  import { untrack } from "svelte";
  import { readSession } from "./event-session";
  import {
    locationState,
    readView,
    updateQuery,
    filterFields,
  } from "./link-state";
  import LinkError from "./LinkError.svelte";
  const address = locationState;
  const linked = $derived(readView("events", $address.search));
  const session = $derived(readSession($address.search));
  function clearSession(replace = false) {
    updateQuery(
      {
        project: null,
        hostname: null,
        tool: null,
        session_id: null,
        subagent_id: null,
        before: null,
        event: null,
        collapsed: null,
      },
      replace,
    );
  }
  import type { SessionGroup } from "./event-hierarchy";
  import { SvelteSet } from "svelte/reactivity";
  import EventRows from "./EventRows.svelte";
  import SessionRows from "./SessionRows.svelte";
  import { eventKey as key, groupEvents, sessionKey } from "./event-hierarchy";
  const hierarchical = $derived(linked.value.hierarchical);
  const collapsed = new SvelteSet<string>();
  let groupLimit = $state(false);
  function toggleGroup(id: string) {
    const visible = new Set(page?.items.map(sessionKey) ?? []);
    const next = new SvelteSet(
      linked.value.collapsed.filter((key) => visible.has(key)),
    );
    if (next.has(id)) next.delete(id);
    else next.add(id);
    try {
      updateQuery({ collapsed: next.size ? JSON.stringify([...next]) : null });
      groupLimit = false;
    } catch {
      groupLimit = true;
    }
  }
  import ListFilters from "./ListFilters.svelte";
  import type { ListFilters as Filters } from "./list-filters";
  let filters = $state<Filters>({});
  function applyFilters(value: Filters) {
    const next =
      session.kind === "session"
        ? {
            from: value.from,
            to: value.to,
            ...session.value,
            subagent_id: session.value.subagent_id ?? "",
          }
        : value;
    updateQuery(filterFields(next));
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
  const expanded = $derived(linked.value.event);
  let signature = "";
  $effect(() => {
    const state = linked.value;
    const key = JSON.stringify([linked.valid, state.filters, state.before]);
    if (key !== signature) {
      signature = key;
      untrack(() => {
        filters = state.filters;
        before = state.before;
        page = null;
      });
    }
  });
  $effect(() => {
    const groups = linked.value.collapsed;
    untrack(() => {
      collapsed.clear();
      for (const key of groups) collapsed.add(key);
    });
  });
  let target = $state<FeedEvent | null>(null),
    targetUnavailable = $state(false);
  const onPage = $derived(page?.items.find((event) => key(event) === expanded));

  $effect(() => {
    const selected = expanded,
      present = onPage,
      loaded = page;
    void retry;
    target = null;
    targetUnavailable = false;
    if (!linked.valid || !selected || !loaded || present) return;
    const [project, event_id] = JSON.parse(selected);
    const controller = new AbortController();
    void events(null, controller.signal, { project, event_id })
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
  const request = $derived(
    JSON.stringify([linked.valid, linked.value.before, linked.value.filters]),
  );
  $effect(() => {
    const [valid, cursor, selected] = JSON.parse(request);
    if (!valid) return;
    void retry;
    return poll(async (signal) => {
      busy = true;
      try {
        const value = await events(cursor, signal, selected);
        if (signal.aborted) return;
        page = value;
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
    updateQuery({ before: cursor, collapsed: null, event: null });
  }
  function toggle(event: FeedEvent) {
    updateQuery({ event: expanded === key(event) ? null : key(event) });
  }
</script>

{#if session.kind === "invalid"}
  <p role="alert">
    This agent session link is invalid. Clear it to choose another filter.
  </p>
  <button class="quiet" onclick={() => clearSession()}
    >Clear session filter</button
  >
{:else}
  {#if session.kind === "session"}<section
      class="session-filter"
      aria-label="Selected agent session"
    >
      <h4>Exact agent session</h4>
      <dl class="identity">
        <dt>Project</dt>
        <dd>{session.value.project}</dd>
        <dt>Agent</dt>
        <dd>{session.value.tool} · {session.value.hostname}</dd>
        <dt>Native session</dt>
        <dd>{session.value.session_id}</dd>
        <dt>Subagent</dt>
        <dd>{session.value.subagent_id ?? "Primary session"}</dd>
      </dl>
      <button class="quiet" onclick={() => clearSession()}
        >Clear session filter</button
      >
    </section>{/if}
{/if}
<ListFilters
  value={filters}
  onapply={applyFilters}
  {onexpire}
  sessionScoped={session.kind === "session"}
  disabled={!linked.valid || session.kind === "invalid"}
  onclear={() => clearSession(true)}
>
  {#snippet heading()}
    <div class="event-heading">
      <h3>Events</h3>
      <p class="muted">
        All projects · Updates every 10 seconds while this page is visible.
      </p>
    </div>
    <div class="event-view-switch" role="group" aria-label="Events view">
      <button
        aria-pressed={!hierarchical}
        onclick={() => updateQuery({ view: null })}>Flat list</button
      >
      <button
        aria-pressed={hierarchical}
        onclick={() => updateQuery({ view: "hierarchy" })}
        >Session hierarchy</button
      >
    </div>
  {/snippet}
  {#if !linked.valid && session.kind !== "invalid"}<LinkError />{/if}
  {#if groupLimit}<p role="alert">
      This group change exceeds the bookmark size limit. Expand another group or
      clear collapsed groups. <button
        onclick={() => {
          updateQuery({ collapsed: null });
          groupLimit = false;
        }}>Clear collapsed groups</button
      >
    </p>{/if}
  {#if targetUnavailable}<p role="alert">
      The linked event is unavailable. <button
        onclick={() => updateQuery({ event: null })}>Clear linked event</button
      ><button onclick={() => retry++}>Retry linked event</button>
    </p>{/if}
  {#if target}<section aria-label="Linked event">
      <p class="muted">
        Linked event outside the current page or collapsed groups.
      </p>
      <table class="events-table">
        <tbody
          ><EventRows
            items={[{ event: target, index: -1 }]}
            {expanded}
            ontoggle={toggle}
          /></tbody
        >
      </table>
    </section>{/if}
  {#if hierarchical}<p class="muted">
      Relationships use this page only. Parent-session groups come first,
      ordered by their newest visible family event; unmatched subagents follow.
      Events within each session retain receipt order. Missing parents are not
      loaded.
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
            ><th scope="col">Time</th><th scope="col">Project</th><th
              scope="col">Agent</th
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
      {#if before !== null}<button
          disabled={busy}
          onclick={() => navigate(null)}>Latest events</button
        >{/if}
      {#if page.next_cursor}<button
          disabled={busy}
          onclick={() => navigate(page!.next_cursor)}>Earlier events</button
        >{/if}
    </div>
  {:else if !failed && linked.valid}<p role="status">Loading events…</p>{/if}
</ListFilters>

<style>
  .event-view-switch {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    margin-block: 0.75rem;
  }
</style>
