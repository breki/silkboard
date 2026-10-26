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
    Main agent sessions only · Own latest received activity · Updates while
    visible. Subagent events remain in Events; workflow reports remain in
    Workflows.
  </p>
  <p class="muted">
    Workflow status is from the linked received snapshot. Late arrivals can be
    the latest observation.
  </p>
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
        {before === null
          ? "No main agent reports have arrived yet."
          : "No main agents on this page. Return to the latest activity."}
      </p>
    {:else}
      <ul class="agent-cards" aria-label="Main agents">
        {#each page.items as item, index (agentKey(item))}
          {@const latest = item.latest}
          {@const stamp = eventTimestamp(item.received_at)}
          <li>
            <article aria-labelledby={`agent-title-${index}`}>
              <header>
                <h3 id={`agent-title-${index}`}>
                  <a
                    href={latest.kind === "workflow"
                      ? routeFor(item.project, latest.task_id, latest.sequence)
                      : sessionEvents(item)}
                  >
                    {item.session_name ?? `${item.tool} on ${item.hostname}`}
                  </a>
                </h3>
                <p class="agent-context muted">{item.tool} · {item.hostname}</p>
              </header>
              <dl class="agent-identity">
                <div>
                  <dt>Project</dt>
                  <dd>{item.project}</dd>
                </div>
                <div>
                  <dt>Native session</dt>
                  <dd><code>{item.session_id}</code></dd>
                </div>
              </dl>
              <div class="activity">
                <h4>Latest activity</h4>
                {#if latest.kind === "workflow"}
                  <p>Workflow: {latest.title}</p>
                  <strong class="reported">Reported {latest.state}</strong>
                {:else}
                  <p>Event: {messagePreview(latest.message)}</p>
                {/if}
              </div>
              <footer>
                <span class="muted">Received · Local time</span>
                <time datetime={new Date(item.received_at).toISOString()}
                  >{stamp.full}</time
                >
              </footer>
            </article>
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
      Agents with new activity move toward the latest page. Return to Latest
      agents while paging to see those updates.
    </p>
  {:else if !failed}
    <p role="status">Loading agents…</p>
  {/if}
</section>

<style>
  .agent-cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 19rem), 1fr));
    gap: 1rem;
    list-style: none;
    padding: 0;
    margin-block: 1rem;
  }
  li {
    min-width: 0;
  }
  article {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    height: 100%;
    min-width: 0;
    padding: 1rem;
    border: 1px solid var(--line);
    border-radius: 0.75rem;
    background: var(--panel);
    overflow-wrap: anywhere;
  }
  h3,
  h4,
  p,
  dl,
  dd {
    margin: 0;
  }
  h3 {
    font-size: 1.15rem;
  }
  h3 a {
    display: block;
    min-height: 2.75rem;
    padding-block: 0.35rem;
  }
  h4 {
    font-size: 0.9rem;
    margin-bottom: 0.4rem;
  }
  .agent-identity {
    display: grid;
    gap: 0.6rem;
  }
  dt,
  footer span {
    font-size: 0.85rem;
    color: var(--muted);
  }
  dd {
    min-width: 0;
  }
  code {
    font-size: 0.85rem;
  }
  .reported {
    display: inline-block;
    margin-top: 0.6rem;
    padding: 0.2rem 0.6rem;
    border: 1px solid var(--line);
    border-radius: 1rem;
    font-size: 0.85rem;
  }
  footer {
    margin-top: auto;
    padding-top: 0.5rem;
    border-top: 1px solid var(--line);
  }
  footer span,
  time {
    display: block;
  }
  time {
    font-size: 0.9rem;
  }
  nav {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-block: 1rem;
  }
</style>
