<script lang="ts">
  import type { PlanLinkHandler, PlanSelector } from "./plan-model";
  import { onMount } from "svelte";
  import { ApiError, type Page } from "./feed-api";
  import { planHistory, type PlanSummary } from "./plan-api";
  import { routeFor } from "./plan-model";
  import { eventTimestamp } from "./event-format";
  let {
    project,
    task,
    onexpire,
    onopen,
    onrevision,
  }: {
    project: string;
    task: string;
    onexpire: () => void;
    onopen: PlanLinkHandler;
    onrevision: PlanSelector;
  } = $props();
  let opened = $state(false);
  let kind = $state<"history" | "revisions">("history");
  let before = $state<string | null>(null);
  let page = $state<Page<PlanSummary> | null>(null);
  let failed = $state(false);
  let busy = $state(false);
  let retry = $state(0);
  let visible = $state(!document.hidden);
  onMount(() => {
    const visibility = () => (visible = !document.hidden);
    document.addEventListener("visibilitychange", visibility);
    return () => document.removeEventListener("visibilitychange", visibility);
  });
  $effect(() => {
    if (!opened || !visible) return;
    const mode = kind,
      cursor = before;
    void retry;
    const controller = new AbortController();
    busy = true;
    void planHistory(project, task, mode, cursor, controller.signal)
      .then((result) => {
        if (!controller.signal.aborted) {
          page = result;
          failed = false;
        }
      })
      .catch((error) => {
        if (controller.signal.aborted) return;
        if (error instanceof ApiError && error.status === 401) onexpire();
        else failed = true;
      })
      .finally(() => {
        if (!controller.signal.aborted) busy = false;
      });
    return () => controller.abort();
  });
  function change(mode: "history" | "revisions") {
    if (mode === kind) return;
    kind = mode;
    before = null;
    page = null;
  }
</script>

<section class="plan-history">
  <button aria-expanded={opened} onclick={() => (opened = !opened)}
    >Received history and revisions</button
  >
  {#if opened}
    <div class="plan-toolbar">
      <button
        aria-pressed={kind === "history"}
        onclick={() => change("history")}>Received reports</button
      ><button
        aria-pressed={kind === "revisions"}
        onclick={() => change("revisions")}>Definition revisions</button
      ><button class="quiet" disabled={busy} onclick={() => retry++}
        >Refresh history</button
      >
    </div>
    {#if failed}<p role="alert">
        Unable to load received history. Missing reports and a failed history
        request are different.
      </p>
      <button onclick={() => retry++}>Retry history</button>{/if}
    {#if page}
      {#if !page.items.length}<p>No received history on this page.</p>{/if}
      <ul
        aria-label={kind === "history"
          ? "Received workflow reports"
          : "Received workflow revisions"}
      >
        {#each page.items as item (kind === "history" ? item.sequence : item.plan_revision)}
          <li>
            <div>
              {#if kind === "revisions"}<button
                  onclick={() => onrevision(item.plan_revision)}
                  >Revision {item.plan_revision}</button
                >{:else}<a
                  href={routeFor(project, task, item.sequence)}
                  onclick={(event) =>
                    onopen(event, routeFor(project, task, item.sequence))}
                  >Report received {eventTimestamp(item.received_at).full}</a
                >{/if}
              <span class="muted">
                · Revision {item.plan_revision} · {item.counts.done} of {item
                  .counts.total} marked done</span
              >
            </div>
            <p class="muted">
              Reported by {item.reporter.tool} on {item.reporter
                .hostname}{#if item.reporter.session_name}
                · {item.reporter.session_name}{/if}
            </p>
          </li>
        {/each}
      </ul>
      <div class="plan-pagination">
        {#if before !== null}<button
            disabled={busy}
            onclick={() => {
              before = null;
              page = null;
            }}>Latest received history</button
          >{/if}{#if page.next_cursor}<button
            disabled={busy}
            onclick={() => {
              before = page!.next_cursor;
              page = null;
            }}>Older received history</button
          >{/if}
      </div>
    {:else if !failed}<p role="status">Loading received history…</p>{/if}
  {/if}
</section>
