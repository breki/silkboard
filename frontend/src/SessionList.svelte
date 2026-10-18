<script lang="ts">
  import { ApiError, sessions, type Page, type Session } from "./feed-api";
  import { poll } from "./poll";
  import SessionCard from "./SessionCard.svelte";

  let { project, onexpire }: { project: string; onexpire: () => void } =
    $props();
  let page = $state<Page<Session> | null>(null);
  let before = $state<string | null>(null);
  let failed = $state(false);
  let retry = $state(0);
  let tick = $state(0);
  let expanded = $state<string | null>(null);
  $effect(() => {
    const name = project,
      cursor = before;
    void retry;
    return poll(async (signal) => {
      try {
        const value = await sessions(name, cursor, signal);
        if (signal.aborted) return;
        page = value;
        failed = false;
        tick++;
      } catch (error) {
        if (signal.aborted) return;
        if (error instanceof ApiError && error.status === 401) onexpire();
        else failed = true;
      }
    });
  });
  function navigate(cursor: string | null) {
    page = null;
    expanded = null;
    before = cursor;
  }
</script>

<div class="session-heading">
  <h3>Sessions</h3>
  <p>Updates every 10 seconds while this page is visible.</p>
</div>
{#if failed}<div class="notice">
    <p role="alert">
      Unable to update sessions. Displayed messages may be out of date.
    </p>
    <button onclick={() => retry++}>Retry sessions</button>
  </div>{/if}
{#if page}
  {#if !page.items.length}<p class="empty">
      No sessions yet for {project}.
    </p>{/if}
  <div class="sessions">
    {#each page.items as session (session.id)}
      <SessionCard
        {project}
        {session}
        {tick}
        open={expanded === session.id}
        ontoggle={() =>
          (expanded = expanded === session.id ? null : session.id)}
        {onexpire}
      />
    {/each}
  </div>
  <div class="page-controls">
    {#if before !== null}<button onclick={() => navigate(null)}
        >Latest sessions</button
      >{/if}
    {#if page.next_cursor}<button onclick={() => navigate(page!.next_cursor)}
        >Earlier sessions</button
      >{/if}
  </div>
{:else if !failed}<p role="status">Loading sessions…</p>{/if}
