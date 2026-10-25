<script lang="ts">
  import { ApiError, type Page } from "./feed-api";
  import { agents, agentKey, type AgentActivity } from "./agents-api";
  import { eventTimestamp } from "./event-format";
  import { messagePreview } from "./markdown";
  import { routeFor } from "./plan-model";
  import { sessionEvents } from "./event-session";
  import { poll } from "./poll";
  let { onexpire }: { onexpire: () => void } = $props();
  let page = $state<Page<AgentActivity> | null>(null);
  let before = $state<string | null>(null);
  let failed = $state(false);
  let busy = $state(false);
  let retry = $state(0);
  $effect(() => {
    const cursor = before;
    void retry;
    return poll(async (signal) => {
      busy = true;
      try {
        const result = await agents(cursor, signal);
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
    page = null;
    before = cursor;
  }
</script>

<section class="agents" aria-label="Agents">
  <h2>Agents</h2>
  <p class="muted">
    One native session or subagent per row · Latest received activity across
    events and workflow history · Updates while visible.
  </p>
  <p class="muted">
    Workflow status is from the linked received snapshot. Late arrivals can be
    the latest observation.
  </p>
  {#if failed}<div class="notice">
      <p role="alert">
        Unable to update agents. Displayed activity may be out of date.
      </p>
      <button onclick={() => retry++}>Retry agents</button>
    </div>{/if}
  {#if page}
    {#if !page.items.length}<p>
        {before === null
          ? "No agent reports have arrived yet."
          : "No agents on this page. Return to the latest activity."}
      </p>
    {:else}
      <table aria-label="Agents and latest activity">
        <thead
          ><tr
            ><th scope="col">Agent</th><th scope="col">Project</th><th
              scope="col">Latest activity</th
            ><th scope="col">Received</th></tr
          ></thead
        ><tbody>
          {#each page.items as item (agentKey(item))}
            {@const latest = item.latest}
            {@const stamp = eventTimestamp(item.received_at)}
            <tr>
              <td
                ><a
                  class="agent-label"
                  href={latest.kind === "workflow"
                    ? routeFor(item.project, latest.task_id, latest.sequence)
                    : sessionEvents(item)}
                  >{item.session_name ?? `${item.tool} on ${item.hostname}`}</a
                ><span class="muted agent-context"
                  >{item.tool} · {item.hostname} · {item.subagent_id === null
                    ? "Primary session"
                    : "Subagent"}</span
                >
                <details>
                  <summary>Session identity</summary>
                  <dl class="identity">
                    <dt>Session name</dt>
                    <dd>{item.session_name ?? "Not reported"}</dd>
                    <dt>Host</dt>
                    <dd>{item.hostname}</dd>
                    <dt>Tool</dt>
                    <dd>{item.tool}</dd>
                    <dt>Native session</dt>
                    <dd>{item.session_id}</dd>
                    {#if item.subagent_id}<dt>Subagent</dt>
                      <dd>
                        {item.subagent_id}
                      </dd>{/if}{#if item.parent_session_id}<dt>
                        Parent session
                      </dt>
                      <dd>{item.parent_session_id}</dd>{/if}
                  </dl>
                </details>
              </td><td>{item.project}</td>
              <td
                >{#if latest.kind === "workflow"}<span class="activity-summary"
                    >Workflow: {latest.title}</span
                  ><strong class="agent-status">Reported {latest.state}</strong
                  >{:else}<span class="activity-summary"
                    >Event: {messagePreview(latest.message)}</span
                  >{/if}</td
              >
              <td
                ><time
                  datetime={new Date(item.received_at).toISOString()}
                  title={stamp.full}
                  ><span>{stamp.clock}</span><span
                    >{stamp.full.slice(0, 10)}</span
                  ></time
                ></td
              >
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
    <nav aria-label="Agent pages">
      {#if before !== null}<button
          disabled={busy}
          onclick={() => navigate(null)}>Latest agents</button
        >{/if}{#if page.next_cursor}<button
          disabled={busy}
          onclick={() => navigate(page!.next_cursor)}>Older agents</button
        >{/if}
    </nav>
    <p class="muted">
      Agents with new activity move toward the latest page. Return to Latest
      agents while paging to see those updates.
    </p>
  {:else if !failed}<p role="status">Loading agents…</p>{/if}
</section>

<style>
  table {
    width: 100%;
    table-layout: fixed;
    border-collapse: collapse;
  }
  th,
  td {
    text-align: start;
    vertical-align: top;
    padding: 0.65rem 0.4rem;
    border-bottom: 1px solid var(--line);
    overflow-wrap: anywhere;
  }
  td span,
  time span,
  .agent-status {
    display: block;
  }
  nav {
    display: flex;
    gap: 0.5rem;
    margin-block: 1rem;
  }
  summary {
    cursor: pointer;
    margin-top: 0.3rem;
  }
  @media (max-width: 600px) {
    th,
    td {
      padding: 0.5rem 0.2rem;
      font-size: 0.88rem;
    }
    th,
    time {
      font-size: 0.75rem;
    }
    .agent-label,
    .agent-context,
    .activity-summary {
      display: -webkit-box;
      -webkit-box-orient: vertical;
      -webkit-line-clamp: 3;
      overflow: hidden;
    }
  }
</style>
