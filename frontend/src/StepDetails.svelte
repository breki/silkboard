<script lang="ts">
  import type { PlanLinkHandler } from "./plan-model";
  import type { PlanDetail } from "./plan-api";
  import type { PlanStep } from "./plan-model";
  import { routeFor, stateLabel, stateSymbol } from "./plan-model";
  import { eventTimestamp } from "./event-format";
  import { renderMarkdown } from "./markdown";
  let {
    detail,
    step,
    pinned,
    onopen,
  }: {
    detail: PlanDetail;
    step: PlanStep;
    pinned: boolean;
    onopen: PlanLinkHandler;
  } = $props();
  const old = $derived(
    detail.prior_completions.find((c) => c.step_id === step.id),
  );
  const reporter = $derived(detail.snapshot.reporter);
</script>

<h3>{step.title}</h3>
<p class="step-state" data-state={step.state}>
  <span aria-hidden="true">{stateSymbol[step.state]}</span>
  {stateLabel[step.state]}
</p>
{#if pinned}<p class="historical-label">Earlier snapshot · Pinned</p>{/if}
{#if step.description}<div
    class="event-message"
    use:renderMarkdown={step.description}
  ></div>{/if}
<h4>Prerequisites</h4>
{#if step.depends_on.length}
  <ul>
    {#each step.depends_on as id (id)}
      {@const prerequisite = detail.snapshot.steps.find((s) => s.id === id)!}
      <li>{prerequisite.title} · {stateLabel[prerequisite.state]}</li>
    {/each}
  </ul>
{:else}<p>No prerequisites.</p>{/if}
<h4>Agent report</h4>
{#if step.report}<div
    class="event-message"
    use:renderMarkdown={step.report}
  ></div>{:else}<p>No report text in this snapshot.</p>{/if}
{#if step.blocked_reason}<h4>Blocking reason</h4>
  <div
    class="event-message"
    use:renderMarkdown={step.blocked_reason}
  ></div>{/if}
{#if old}
  <p class="notice">
    Previously marked done in received history.
    <a
      href={routeFor(
        detail.summary.project,
        detail.summary.task_id,
        old.sequence,
      )}
      onclick={(event) =>
        onopen(
          event,
          routeFor(
            detail.summary.project,
            detail.summary.task_id,
            old.sequence,
          ),
        )}>View that snapshot</a
    >
    · Received {eventTimestamp(old.received_at).full}
  </p>
{/if}
<dl class="identity">
  <dt>Snapshot received</dt>
  <dd>
    <time datetime={new Date(detail.summary.received_at).toISOString()}
      >{eventTimestamp(detail.summary.received_at).full}</time
    >
  </dd>
  <dt>Reported tool</dt>
  <dd>{reporter.tool}</dd>
  <dt>Reported host</dt>
  <dd>{reporter.hostname}</dd>
  {#if reporter.session_name}<dt>Session name</dt>
    <dd>{reporter.session_name}</dd>{/if}
  <dt>Native session</dt>
  <dd>{reporter.session_id}</dd>
  {#if reporter.subagent_id}<dt>Subagent</dt>
    <dd>{reporter.subagent_id}</dd>{/if}
  {#if reporter.parent_session_id}<dt>Parent session</dt>
    <dd>{reporter.parent_session_id}</dd>{/if}
</dl>
