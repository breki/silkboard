<script lang="ts">
  import { ApiError, type Page } from "./feed-api";
  import { agents, agentKey, type AgentActivity } from "./agents-api";
  import { eventTimestamp } from "./event-format";
  import { messagePreview } from "./markdown";
  import { routeFor } from "./plan-model";
  import { sessionEvents } from "./event-session";
  import { poll } from "./poll";
  import ListFilters from "./ListFilters.svelte";
  import type { ListFilters as Filters } from "./list-filters";
  let filters = $state<Filters>({});
  function applyFilters(value: Filters) {
    navigate(null);
    filters = value;
  }
  let { onexpire }: { onexpire: () => void } = $props();
  let page = $state<Page<AgentActivity> | null>(null);
  let before = $state<string | null>(null);
  let failed = $state(false);
  let busy = $state(false);
  let retry = $state(0);
  $effect(() => {
    const cursor = before;
    const selected = filters;
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
    page = null;
    before = cursor;
  }
</script>

<section class="agents" aria-label="Agents">
  <ListFilters agentsView onapply={applyFilters} {onexpire}>
    {#snippet heading()}
      <h2>Agents</h2>
      <p class="muted">
        Latest received activity from your main agent sessions.
      </p>
    {/snippet}

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
          {#each page.items as item, index (agentKey(item))}
            {@const latest = item.latest}
            {@const stamp = eventTimestamp(item.received_at)}
            <li>
              <article aria-labelledby={`agent-title-${index}`}>
                <header>
                  <div class="agent-project">
                    <span>{item.project}</span
                    >{#if latest.kind === "workflow"}<strong
                        class="reported"
                        data-state={latest.state}
                        >Reported {latest.state}</strong
                      >{/if}
                  </div>
                  <h3 id={`agent-title-${index}`}>
                    <a
                      href={latest.kind === "workflow"
                        ? routeFor(
                            item.project,
                            latest.task_id,
                            latest.sequence,
                          )
                        : sessionEvents(item)}
                    >
                      {item.session_name ?? `${item.tool} on ${item.hostname}`}
                    </a>
                  </h3>
                  <p class="agent-context muted">
                    {item.tool} · {item.hostname}
                  </p>
                </header>
                <div class="activity">
                  {#if latest.kind === "workflow"}
                    <p>Workflow: {latest.title}</p>
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
                <details>
                  <summary>Report &amp; identity</summary>
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
                </details>
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
        Reported workflow states come from the linked snapshot; they are not
        connection status. Subagent events remain in Events. Agents with new
        activity move toward the latest page. Return to Latest agents while
        paging to see those updates.
      </p>
    {:else if !failed}
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
  article {
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
    height: 100%;
    min-width: 0;
    padding: 1rem;
    border: 1px solid var(--line);
    border-radius: 0.75rem;
    background: var(--panel);
    overflow-wrap: anywhere;
  }
  h3,
  p,
  dl,
  dd {
    margin: 0;
  }
  h3 {
    font-size: 1rem;
  }
  h3 a {
    display: block;
    min-height: 2rem;
    padding-block: 0.35rem;
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
  .agent-project {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    font-size: 0.75rem;
    color: var(--muted);
    margin-bottom: 0.5rem;
  }
  .agent-project .reported {
    border: 0;
    padding: 0;
    margin: 0;
    color: var(--primary);
    font-size: 0.7rem;
    font-weight: 500;
  }
  .agent-project .reported[data-state="blocked"] {
    color: var(--historical-text);
  }
  details {
    font-size: 0.8rem;
  }
  summary {
    color: var(--link);
    cursor: pointer;
    min-height: 2rem;
    padding-block: 0.4rem;
  }
  details dl {
    margin-top: 0.75rem;
  }
  .result-count {
    font-size: 0.75rem;
    margin: 0;
  }
  @media (max-width: 700px), (pointer: coarse) {
    h3 a,
    summary {
      min-height: 2.75rem;
    }
  }
</style>
