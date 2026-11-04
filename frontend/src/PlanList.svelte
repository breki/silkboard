<script lang="ts">
  import ListFilters from "./ListFilters.svelte";
  import type { ListFilters as Filters } from "./list-filters";
  let filters = $state<Filters>({});
  function applyFilters(value: Filters) {
    navigate(null);
    filters = value;
  }
  import type { PlanLinkHandler } from "./plan-model";
  import { ApiError, type Page } from "./feed-api";
  import { listPlans, type PlanSummary } from "./plan-api";
  import { routeFor } from "./plan-model";
  import { eventTimestamp } from "./event-format";
  import { poll } from "./poll";
  let {
    active,
    onexpire,
    onopen,
  }: {
    active: boolean;
    onexpire: () => void;
    onopen: PlanLinkHandler;
  } = $props();
  let page = $state<Page<PlanSummary> | null>(null);
  let before = $state<string | null>(null);
  let failed = $state(false);
  let unsupported = $state(false);
  let busy = $state(false);
  let retry = $state(0);
  $effect(() => {
    if (!active) return;
    const cursor = before;
    const selected = filters;
    void retry;
    return poll(async (signal) => {
      busy = true;
      try {
        const result = await listPlans(cursor, signal, selected);
        if (signal.aborted) return;
        page = result;
        failed = false;
        unsupported = false;
      } catch (error) {
        if (signal.aborted) return;
        if (error instanceof ApiError && error.status === 401) onexpire();
        else {
          failed = true;
          unsupported = error instanceof ApiError && error.status === 404;
        }
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

<section class="plans" aria-label="Workflows">
  <ListFilters workflows onapply={applyFilters} {onexpire}>
    {#snippet heading()}
      <h2>Workflows</h2>
      <p class="muted">
        All projects · Last reported states · Updates while this page is
        visible.
      </p>
    {/snippet}

    {#if failed}
      <div class="notice">
        <p role="alert">
          {unsupported
            ? "Workflows are not available on this server."
            : "Unable to update workflows. Displayed reports may be out of date."}
        </p>
        <button onclick={() => retry++}>Retry workflows</button>
      </div>
    {/if}
    {#if page}
      {#if !page.items.length}
        <p>
          {Object.values(filters).some(
            (value) => value !== undefined && value !== "",
          )
            ? "No workflows match these filters on this page."
            : before === null
              ? "No workflow reports have arrived yet."
              : "No workflows on this page. Return to the latest reports."}
        </p>
      {:else}
        <table class="plan-table" aria-label="Workflows from all projects">
          <colgroup
            ><col class="plan-time" /><col class="plan-project" /><col /><col
              class="plan-state-column"
            /></colgroup
          >
          <thead
            ><tr
              ><th scope="col">Last report</th><th scope="col">Project</th><th
                scope="col">Workflow</th
              ><th scope="col">Reported state</th></tr
            ></thead
          >
          <tbody>
            {#each page.items as item (`${item.project}/${item.task_id}`)}
              {@const stamp = eventTimestamp(item.received_at)}
              <tr
                ><td
                  ><time
                    datetime={new Date(item.received_at).toISOString()}
                    title={stamp.full}
                    ><span>{stamp.clock}</span><span>{stamp.day}</span></time
                  ></td
                >
                <td>{item.project}</td><td
                  ><a
                    href={routeFor(item.project, item.task_id)}
                    onclick={(event) =>
                      onopen(event, routeFor(item.project, item.task_id))}
                    >{item.title}</a
                  ></td
                >
                <td
                  ><strong class="reported-state" data-state={item.state}
                    >{item.state}</strong
                  ><span class="muted"
                    >{item.counts.done}/{item.counts.total} marked done</span
                  >{#if item.counts.active}<span
                      >{item.counts.active} active</span
                    >{/if}{#if item.counts.blocked}<span
                      >{item.counts.blocked} blocked</span
                    >{/if}</td
                >
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
      <nav class="plan-pagination" aria-label="Workflow pages">
        {#if before !== null}<button
            disabled={busy}
            onclick={() => navigate(null)}>Latest workflows</button
          >{/if}
        {#if page.next_cursor}<button
            disabled={busy}
            onclick={() => navigate(page!.next_cursor)}>Older workflows</button
          >{/if}
      </nav>
    {:else if !failed}<p role="status">Loading workflows…</p>{/if}
  </ListFilters>
</section>
