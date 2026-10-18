<script lang="ts">
  import {
    ApiError,
    history,
    type Page,
    type Session,
    type Message,
  } from "./feed-api";

  let {
    project,
    session,
    tick,
    open,
    ontoggle,
    onexpire,
  }: {
    project: string;
    session: Session;
    tick: number;
    open: boolean;
    ontoggle: () => void;
    onexpire: () => void;
  } = $props();
  let page = $state<Page<Message> | null>(null);
  let before = $state<string | null>(null);
  let failed = $state(false);
  let busy = $state(false);
  let retry = $state(0);
  const name = $derived(session.session_name ?? `${session.tool} session`);
  $effect(() => {
    const cursor = before,
      id = session.id,
      name = project;
    void tick;
    void retry;
    if (!open) return;
    const controller = new AbortController();
    busy = true;
    failed = false;
    void (async () => {
      try {
        const value = await history(name, id, cursor, controller.signal);
        if (!controller.signal.aborted) page = value;
      } catch (error) {
        if (controller.signal.aborted) return;
        if (error instanceof ApiError && error.status === 401) onexpire();
        else failed = true;
      } finally {
        if (!controller.signal.aborted) busy = false;
      }
    })();
    return () => controller.abort();
  });
  function navigate(cursor: string | null) {
    page = null;
    before = cursor;
  }
  const date = (value: number) => new Date(value).toLocaleString();
</script>

<article class="session-card">
  <div class="card-heading">
    <h4>{name}</h4>
    <span class="tool">{session.tool}</span>
  </div>
  <dl class="identity">
    <dt>Host</dt>
    <dd>{session.hostname}</dd>
    <dt>Native session</dt>
    <dd>{session.session_id}</dd>
    {#if session.subagent_id}<dt>Subagent</dt>
      <dd>{session.subagent_id}</dd>{/if}
    {#if session.parent_session_id}<dt>Parent session</dt>
      <dd>{session.parent_session_id}</dd>{/if}
  </dl>
  <p class="latest-message">{session.latest.message}</p>
  <p class="timestamp">
    Received <time datetime={new Date(session.latest.received_at).toISOString()}
      >{date(session.latest.received_at)}</time
    >
  </p>
  <button
    class="history-toggle"
    aria-expanded={open}
    aria-controls={`history-${session.id}`}
    onclick={ontoggle}>{open ? "Hide history" : "View history"}</button
  >
  {#if open}
    <section
      id={`history-${session.id}`}
      class="history"
      aria-label={`Message history for ${name}`}
    >
      <h5>Message history</h5>
      <p class="muted">Newest messages first</p>
      {#if failed}<p role="alert">
          Unable to update message history. Try again.
        </p>
        <button onclick={() => retry++}>Retry history</button>{/if}
      {#if page}
        {#if !page.items.length}<p>No messages on this page.</p>{/if}
        <ol>
          {#each page.items as message (message.event_id)}<li>
              <time datetime={new Date(message.received_at).toISOString()}
                >{date(message.received_at)}</time
              >
              <p>{message.message}</p>
            </li>{/each}
        </ol>
        <div class="page-controls">
          {#if before !== null}<button
              disabled={busy}
              onclick={() => navigate(null)}>Latest messages</button
            >{/if}
          {#if page.next_cursor}<button
              disabled={busy}
              onclick={() => navigate(page!.next_cursor)}
              >Earlier messages</button
            >{/if}
        </div>
      {:else if !failed}<p role="status">Loading messages…</p>{/if}
    </section>
  {/if}
</article>
