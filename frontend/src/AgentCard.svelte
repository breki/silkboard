<script lang="ts">
  import type { AgentActivity } from "./agents-api";
  import { eventTimestamp } from "./event-format";
  import { messagePreview } from "./markdown";
  import { routeFor } from "./plan-model";
  import { sessionEvents } from "./event-session";
  let {
    item,
    expanded = false,
    onexpand,
  }: {
    item: AgentActivity;
    expanded?: boolean;
    // eslint-disable-next-line no-unused-vars -- Callback describes the disclosure state.
    onexpand: (open: boolean) => void;
  } = $props();
  const id = $props.id();
  const latest = $derived(item.latest),
    stamp = $derived(eventTimestamp(item.received_at));
</script>

<article aria-labelledby={id}>
  <header>
    <div class="agent-project">
      <span>{item.project}</span>{#if latest.kind === "workflow"}<strong
          class="reported"
          data-state={latest.state}>{`Reported ${latest.state}`}</strong
        >{/if}
    </div>
    <h3 {id}>
      <a
        href={latest.kind === "workflow"
          ? routeFor(item.project, latest.task_id, latest.sequence)
          : sessionEvents(item)}
      >
        {item.session_name ?? `${item.tool} on ${item.hostname}`}
      </a>
    </h3>
    <p class="agent-context muted">
      {`${item.tool} · ${item.hostname}`}
    </p>
  </header>
  <div class="activity">
    {#if latest.kind === "workflow"}
      <p>{`Workflow: ${latest.title}`}</p>
    {:else}
      <p>{`Event: ${messagePreview(latest.message)}`}</p>
    {/if}
  </div>
  <footer>
    <span class="muted">Received · Local time</span>
    <time datetime={new Date(item.received_at).toISOString()}>{stamp.full}</time
    >
  </footer>
  <details
    open={expanded}
    ontoggle={(event) => {
      if (event.currentTarget.open !== expanded)
        onexpand(event.currentTarget.open);
    }}
  >
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

<style>
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
  .agent-project {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    font-size: 0.75rem;
    color: var(--muted);
    margin-bottom: 0.5rem;
  }
  .reported {
    font-size: 0.7rem;
    font-weight: 500;
    color: var(--primary);
  }
  .reported[data-state="blocked"] {
    color: var(--historical-text);
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
  @media (max-width: 700px), (pointer: coarse) {
    h3 a,
    summary {
      min-height: 2.75rem;
    }
  }
</style>
