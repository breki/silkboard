<script lang="ts">
  import type { PlanLinkHandler, PlanSelector } from "./plan-model";
  import { onMount } from "svelte";
  import { ApiError } from "./feed-api";
  import {
    readPlan,
    listPlans,
    planHistory,
    type PlanDetail,
  } from "./plan-api";
  import {
    routeFor,
    receivedChanges,
    reportAge,
    stateLabel,
  } from "./plan-model";
  import { eventTimestamp } from "./event-format";
  import { renderMarkdown } from "./markdown";
  import { poll } from "./poll";
  import PlanGraph from "./PlanGraph.svelte";
  import PlanHistory from "./PlanHistory.svelte";
  let {
    project,
    task,
    sequence,
    onexpire,
    onopen,
    onsequence,
  }: {
    project: string;
    task: string;
    sequence: string | null;
    onexpire: () => void;
    onopen: PlanLinkHandler;
    onsequence: PlanSelector;
  } = $props();
  let shown = $state<PlanDetail | null>(null);
  let pinned = $state(false);
  let latestSequence = $state<string | null>(null);
  let failed = $state("");
  let loading = $state(false);
  let retry = $state(0);
  let now = $state(Date.now());
  let auxiliary: AbortController | null = null;
  let extraBusy = $state(false);
  let extraFailure = $state("");
  let comparison = $state<{
    sequence: string;
    previous: string;
    changes: ReturnType<typeof receivedChanges>;
  } | null>(null);
  let comparisonEmpty = $state(false);
  let alive = true;
  function apply(value: PlanDetail, historical: boolean) {
    if (shown?.summary.sequence !== value.summary.sequence) {
      comparison = null;
      comparisonEmpty = false;
      extraFailure = "";
    }
    shown = value;
    pinned = historical;
  }
  async function failure(
    error: unknown,
    snapshot: boolean,
    signal: AbortSignal,
  ) {
    if (error instanceof ApiError && error.status === 401) {
      onexpire();
      return "";
    }
    if (error instanceof ApiError && error.status === 404) {
      if (snapshot) return "That received snapshot was not found.";
      try {
        await listPlans(null, signal);
        return "That task plan was not found.";
      } catch (probe) {
        if (probe instanceof ApiError && probe.status === 401) {
          onexpire();
          return "";
        }
        if (probe instanceof ApiError && probe.status === 404)
          return "Task plans are not available on this server.";
      }
    }
    return "Unable to update this plan. Displayed reports may be out of date.";
  }
  $effect(() => {
    const selected = sequence;
    void retry;
    auxiliary?.abort();
    return poll(async (signal) => {
      loading = true;
      now = Date.now();
      let historical = false;
      try {
        const latest = await readPlan(project, task, {}, signal);
        if (signal.aborted || !alive) return;
        if (
          latestSequence !== null &&
          BigInt(latest.summary.sequence) < BigInt(latestSequence)
        )
          throw new ApiError(0);
        latestSequence = latest.summary.sequence;
        if (selected === null) apply(latest, false);
        else if (!pinned || shown?.summary.sequence !== selected) {
          historical = true;
          const older = await readPlan(
            project,
            task,
            { sequence: selected },
            signal,
          );
          if (signal.aborted || !alive) return;
          apply(older, true);
        }
        failed = "";
      } catch (error) {
        if (signal.aborted || !alive) return;
        const message = await failure(error, historical, signal);
        if (!signal.aborted && alive) failed = message;
      } finally {
        if (!signal.aborted && alive) {
          loading = false;
          now = Date.now();
        }
      }
    });
  });
  function beginExtra() {
    auxiliary?.abort();
    auxiliary = new AbortController();
    extraBusy = true;
    extraFailure = "";
    return auxiliary;
  }
  async function chooseRevision(revision: string) {
    const controller = beginExtra();
    try {
      const value = await readPlan(
        project,
        task,
        { revision },
        controller.signal,
      );
      if (controller.signal.aborted || !alive) return;
      apply(value, true);
      onsequence(routeFor(project, task, value.summary.sequence));
    } catch (error) {
      if (controller.signal.aborted || !alive) return;
      const message = await failure(error, true, controller.signal);
      if (!controller.signal.aborted && alive) extraFailure = message;
    } finally {
      if (auxiliary === controller && alive) {
        auxiliary = null;
        extraBusy = false;
      }
    }
  }
  async function comparePrevious() {
    if (!shown) return;
    const target = shown;
    const controller = beginExtra();
    comparison = null;
    comparisonEmpty = false;
    try {
      const previous = await planHistory(
        project,
        task,
        "history",
        target.summary.sequence,
        controller.signal,
      );
      if (controller.signal.aborted || !alive) return;
      if (shown?.summary.sequence !== target.summary.sequence) return;
      if (!previous.items.length) {
        comparisonEmpty = true;
        return;
      }
      const older = await readPlan(
        project,
        task,
        { sequence: previous.items[0].sequence },
        controller.signal,
      );
      if (
        controller.signal.aborted ||
        !alive ||
        shown?.summary.sequence !== target.summary.sequence
      )
        return;
      comparison = {
        sequence: target.summary.sequence,
        previous: older.summary.sequence,
        changes: receivedChanges(older.snapshot, target.snapshot),
      };
    } catch (error) {
      if (controller.signal.aborted || !alive) return;
      const message = await failure(error, true, controller.signal);
      if (!controller.signal.aborted && alive) extraFailure = message;
    } finally {
      if (auxiliary === controller && alive) {
        auxiliary = null;
        extraBusy = false;
      }
    }
  }
  onMount(() => {
    const visibility = () => {
      if (document.hidden) auxiliary?.abort();
    };
    document.addEventListener("visibilitychange", visibility);
    return () => {
      alive = false;
      auxiliary?.abort();
      document.removeEventListener("visibilitychange", visibility);
    };
  });
</script>

<section class="task-plan" aria-label="Task plan">
  <nav aria-label="Task breadcrumbs">
    <a href="/plans" onclick={(event) => onopen(event, "/plans")}>Task plans</a>
    / {project}
  </nav>
  {#if failed}<div class="notice">
      <p role="alert">{failed}</p>
      <button disabled={loading} onclick={() => retry++}>Retry plan</button>
    </div>{/if}
  {#if shown}
    <h2>{shown.snapshot.title}</h2>
    {#if pinned}<p class="historical-label">Earlier snapshot · Pinned</p>{/if}
    {#if sequence !== null && (!pinned || shown.summary.sequence !== sequence)}<p
        role="status"
      >
        Loading selected snapshot; the previous report is still displayed.
      </p>{/if}
    {#if pinned && latestSequence !== null && BigInt(latestSequence) > BigInt(shown.summary.sequence)}<p
        class="notice"
      >
        Newer reports are available. <a
          href={routeFor(project, task)}
          onclick={(event) => onopen(event, routeFor(project, task))}
          >Return to Current plan</a
        >
      </p>{/if}
    {#if shown.snapshot.description}<div
        class="event-message plan-description"
        use:renderMarkdown={shown.snapshot.description}
      ></div>{/if}
    {#if shown.snapshot.github_issue}<p>
        <a
          href={`https://github.com/${shown.snapshot.github_issue.repository}/issues/${shown.snapshot.github_issue.number}`}
          >GitHub issue #{shown.snapshot.github_issue.number}</a
        >
      </p>{/if}
    <div class="plan-summary panel">
      <p aria-live="polite" aria-atomic="true">
        <strong
          >Last reported: <span
            class="reported-state"
            data-state={shown.summary.state}>{shown.summary.state}</span
          ></strong
        >
        · {shown.summary.counts.active} active · {shown.summary.counts.blocked} blocked
      </p>
      <p>
        {shown.summary.counts.done} of {shown.summary.counts.total} steps marked done
      </p>
      <div class="count-track" aria-hidden="true">
        <div
          style:width={`${(shown.summary.counts.done / shown.summary.counts.total) * 100}%`}
        ></div>
      </div>
      <p class="muted">States are agent reports, not verification results.</p>
      <p>
        Reported by {shown.snapshot.reporter.tool} on {shown.snapshot.reporter
          .hostname}{#if shown.snapshot.reporter.session_name}
          · {shown.snapshot.reporter.session_name}{/if}
      </p>
      <p>
        Snapshot received <time
          datetime={new Date(shown.summary.received_at).toISOString()}
          >{eventTimestamp(shown.summary.received_at).full}</time
        >
        · {reportAge(shown.summary.received_at, now)}
      </p>
      <p>
        Definition revision {shown.summary.plan_revision}{#if pinned}
          · <a
            href={routeFor(project, task)}
            onclick={(event) => onopen(event, routeFor(project, task))}
            >Current plan</a
          >{/if}
      </p>
      {#if shown.summary.history_incomplete}<p class="notice">
          Some reports were not received. Received history is incomplete; this
          snapshot does not reconstruct missing updates.
        </p>{/if}
    </div>
    <PlanGraph detail={shown} {pinned} {onopen} />
    <PlanHistory
      {project}
      {task}
      {onexpire}
      {onopen}
      onrevision={(revision) => void chooseRevision(revision)}
    />
    <section class="received-comparison">
      <button disabled={extraBusy} onclick={() => void comparePrevious()}
        >Compare with previous received report</button
      >
      {#if extraBusy}<p role="status">Loading received snapshot…</p>{/if}
      {#if extraFailure}<p role="alert">{extraFailure}</p>{/if}
      {#if comparisonEmpty}<p>
          No earlier report was received for comparison.
        </p>{/if}
      {#if comparison && comparison.sequence === shown.summary.sequence}
        {@const changes = comparison.changes}
        <h3>Changes between received snapshots</h3>
        {#if changes.gap}<p class="notice">
            This comparison spans missing reports; unreceived intermediate work
            is unknown.
          </p>{/if}
        {#if changes.continuity}<p>
            Reported session continuity changed. Only received identities are
            shown.
          </p>{/if}
        <ul>
          {#each changes.added as step (step.id)}<li>
              Added: {step.title}
            </li>{/each}{#each changes.removed as step (step.id)}<li>
              Removed: {step.title}
            </li>{/each}{#each changes.prerequisites as step (step.id)}<li>
              Changed prerequisites: {step.title}
            </li>{/each}{#each changes.definitions as step (step.id)}<li>
              Changed title or description: {step.title}
            </li>{/each}{#each changes.states as step (step.id)}<li>
              {step.title}: {stateLabel[step.from]} → {stateLabel[step.to]}
            </li>{/each}
        </ul>
        {#if !changes.added.length && !changes.removed.length && !changes.prerequisites.length && !changes.definitions.length && !changes.states.length}<p
          >
            No step changes between these received snapshots.
          </p>{/if}
        <a
          href={routeFor(project, task, comparison.previous)}
          onclick={(event) =>
            onopen(event, routeFor(project, task, comparison!.previous))}
          >View previous received snapshot</a
        >
      {/if}
    </section>
  {:else if !failed}<p role="status">Loading task plan…</p>{/if}
</section>
