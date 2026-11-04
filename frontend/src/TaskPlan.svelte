<script lang="ts">
  import type { PlanLinkHandler, PlanSelector } from "./plan-model";
  import {
    locationState,
    readView,
    updateQuery,
    workflowLink,
  } from "./link-state";
  const address = locationState;
  import { onMount, untrack } from "svelte";
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
        return "That workflow was not found.";
      } catch (probe) {
        if (probe instanceof ApiError && probe.status === 401) {
          onexpire();
          return "";
        }
        if (probe instanceof ApiError && probe.status === 404)
          return "Workflows are not available on this server.";
      }
    }
    return "Unable to update this workflow. Displayed reports may be out of date.";
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
  let comparisonKey: string | null = null,
    comparisonRead: AbortController | null = null;
  let wasComparing = $state(false);
  $effect(() => {
    const requested = readView("workflow", $address.search).value.compare;
    const target = shown?.summary.sequence;
    const ready = sequence === null || sequence === target;
    if (!requested && wasComparing)
      untrack(() => {
        comparisonRead?.abort();
        comparison = null;
        comparisonEmpty = false;
        extraFailure = "";
        comparisonKey = null;
      });
    wasComparing = requested;
    if (requested && ready && target && comparisonKey !== target) {
      comparisonKey = target;
      untrack(() => void comparePrevious());
    }
  });
  async function comparePrevious() {
    if (!shown) return;
    const target = shown;
    const controller = beginExtra();
    comparisonRead = controller;
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

<section class="task-plan" aria-label="Workflow">
  <nav aria-label="Workflow breadcrumbs">
    <a
      href={workflowLink("/plans", $address.search)}
      onclick={(event) => onopen(event, "/plans")}>Workflows</a
    >
    / {project}
  </nav>
  {#if failed}<div class="notice">
      <p role="alert">{failed}</p>
      <button disabled={loading} onclick={() => retry++}>Retry workflow</button>
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
          href={workflowLink(routeFor(project, task), $address.search)}
          onclick={(event) => onopen(event, routeFor(project, task))}
          >Return to Current workflow</a
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
      <p class="muted">
        These are reported states, not verified results or a live connection
        status.
      </p>
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
            href={workflowLink(routeFor(project, task), $address.search)}
            onclick={(event) => onopen(event, routeFor(project, task))}
            >Current workflow</a
          >{/if}
      </p>
      {#if shown.summary.history_incomplete}<p class="notice">
          Some reports were not received. Received history is incomplete; this
          snapshot does not reconstruct missing updates.
        </p>{/if}
    </div>
    <PlanGraph
      detail={shown}
      {pinned}
      pending={sequence !== null && shown.summary.sequence !== sequence}
      {onopen}
    />
    <PlanHistory
      {project}
      {task}
      {onexpire}
      {onopen}
      onrevision={(revision) => void chooseRevision(revision)}
    />
    <section class="received-comparison">
      {#if wasComparing}<button
          class="quiet"
          onclick={() => updateQuery({ compare: null })}
          >Close comparison</button
        >{/if}
      <button
        disabled={extraBusy}
        onclick={() => {
          if (wasComparing) void comparePrevious();
          else updateQuery({ compare: "previous" });
        }}>Compare with previous received report</button
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
          href={workflowLink(
            routeFor(project, task, comparison.previous),
            $address.search,
          )}
          onclick={(event) =>
            onopen(event, routeFor(project, task, comparison!.previous))}
          >View previous received snapshot</a
        >
      {/if}
    </section>
  {:else if !failed}<p role="status">Loading workflow…</p>{/if}
</section>
