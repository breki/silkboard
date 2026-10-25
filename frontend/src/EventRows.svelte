<script lang="ts">
  import type { EventRow, EventToggle } from "./event-hierarchy";
  import { eventKey } from "./event-hierarchy";
  import { routeFor } from "./plan-model";
  import { eventTimestamp } from "./event-format";
  import { messagePreview, renderMarkdown } from "./markdown";
  let {
    items,
    expanded,
    ontoggle,
    nested = false,
  }: {
    items: EventRow[];
    expanded: string | null;
    ontoggle: EventToggle;
    nested?: boolean;
  } = $props();
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

{#each items as { event, index } (eventKey(event))}
  {@const stamp = eventTimestamp(event.received_at)}
  {@const summary = messagePreview(event.message)}
  <tr class="event-row" class:nested use:clickableRow={() => ontoggle(event)}>
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
        aria-expanded={expanded === eventKey(event)}
        aria-controls={`event-details-${index}`}
        onclick={() => ontoggle(event)}>{summary}</button
      >{#if event.workflow_task_id}<a
          class="workflow-link"
          href={routeFor(event.project, event.workflow_task_id)}
          >Open workflow</a
        >{/if}</td
    >
  </tr>
  {#if expanded === eventKey(event)}
    <tr class="event-details"
      ><td colspan="4">
        <section id={`event-details-${index}`} aria-label="Event details">
          {#if event.session_name}<h4>{event.session_name}</h4>{/if}
          <div class="event-message" use:renderMarkdown={event.message}></div>
          {#if event.description}<h4>Description</h4>
            <div
              class="event-message"
              use:renderMarkdown={event.description}
            ></div>{/if}
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

<style>
  .workflow-link {
    display: inline-block;
    padding-block: 0.5rem;
  }
  .nested td:last-child {
    border-inline-start: 2px solid var(--border);
    padding-inline-start: 0.75rem;
  }
</style>
