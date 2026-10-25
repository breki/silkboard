<script lang="ts">
  import { routeFor } from "./plan-model";
  import ListFilters from "./ListFilters.svelte";
  import type { ListFilters as Filters } from "./list-filters";
  let filters = $state<Filters>({});
  function applyFilters(value: Filters) {
    navigate(null);
    filters = value;
  }
  import { ApiError, events, type Page, type FeedEvent } from "./feed-api";
  import { poll } from "./poll";
  import { eventTimestamp } from "./event-format";
  import { messagePreview, renderMarkdown } from "./markdown";
  let { onexpire }: { onexpire: () => void } = $props();
  let page = $state<Page<FeedEvent> | null>(null);
  let before = $state<string | null>(null);
  let failed = $state(false);
  let busy = $state(false);
  let retry = $state(0);
  let expanded = $state<string | null>(null);
  const key = (event: FeedEvent) =>
    JSON.stringify([event.project, event.event_id]);
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
    expanded = null;
    before = cursor;
  }
  function toggle(event: FeedEvent) {
    expanded = expanded === key(event) ? null : key(event);
  }
  // Pointer activation anywhere on the row shares the native button action.
  // The button retains normal keyboard behavior and table row semantics.
  function clickableRow(node: HTMLTableRowElement, activate: () => void) {
    const click = (event: MouseEvent) => {
      if (event.target instanceof Element && !event.target.closest("button, a"))
        activate();
    };
    node.addEventListener("click", click);
    return {
      update(next: () => void) {
        activate = next;
      },
      destroy() {
        node.removeEventListener("click", click);
      },
    };
  }
</script>

<div class="event-heading">
  <h3>Events</h3>
  <p class="muted">
    All projects · Updates every 10 seconds while this page is visible.
  </p>
</div>
<ListFilters onapply={applyFilters} />
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
        {#each page.items as event, index (key(event))}
          {@const stamp = eventTimestamp(event.received_at)}
          {@const summary = messagePreview(event.message)}
          <tr class="event-row" use:clickableRow={() => toggle(event)}>
            <td
              ><time datetime={new Date(event.received_at).toISOString()}
                ><span>{stamp.clock}</span><span>{stamp.day}</span></time
              ></td
            >
            <td><span class="cell-label">{event.project}</span></td>
            <td
              ><span class="cell-label">{event.tool}</span><span
                class="cell-label muted">{event.hostname}</span
              ></td
            >
            <td
              ><button
                class="event-preview"
                aria-label={`Event details: ${summary}`}
                aria-expanded={expanded === key(event)}
                aria-controls={`event-details-${index}`}
                onclick={() => toggle(event)}>{summary}</button
              >{#if event.workflow_task_id}<a
                  class="workflow-link"
                  href={routeFor(event.project, event.workflow_task_id)}
                  >Open workflow</a
                >{/if}</td
            >
          </tr>
          {#if expanded === key(event)}
            <tr class="event-details"
              ><td colspan="4">
                <section
                  id={`event-details-${index}`}
                  aria-label="Event details"
                >
                  {#if event.session_name}<h4>{event.session_name}</h4>{/if}
                  <div
                    class="event-message"
                    use:renderMarkdown={event.message}
                  ></div>
                  <dl class="identity">
                    <dt>Project</dt>
                    <dd>{event.project}</dd>
                    <dt>Tool</dt>
                    <dd>{event.tool}</dd>
                    <dt>Host</dt>
                    <dd>{event.hostname}</dd>
                    <dt>Received</dt>
                    <dd>
                      <time datetime={new Date(event.received_at).toISOString()}
                        >{stamp.full}</time
                      >
                    </dd>
                    <dt>Native session</dt>
                    <dd>{event.session_id}</dd>
                    {#if event.subagent_id}<dt>Subagent</dt>
                      <dd>{event.subagent_id}</dd>{/if}
                    {#if event.parent_session_id}<dt>Parent session</dt>
                      <dd>{event.parent_session_id}</dd>{/if}
                  </dl>
                </section>
              </td></tr
            >
          {/if}
        {/each}
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
  .workflow-link {
    display: inline-block;
    padding-block: 0.5rem;
  }
</style>
