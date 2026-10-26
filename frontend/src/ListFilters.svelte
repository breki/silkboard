<script lang="ts">
  import type { ApplyFilters } from "./list-filters";
  import FilterPicker from "./FilterPicker.svelte";
  let {
    workflows = false,
    onapply,
    onexpire,
    sessionScoped = false,
    onclear,
  }: {
    workflows?: boolean;
    onapply: ApplyFilters;
    onexpire: () => void;
    sessionScoped?: boolean;
    onclear?: () => void;
  } = $props();
  let project = $state("");
  let agent = $state("");
  let from = $state("");
  let to = $state("");
  let workflowState = $state("");
  let error = $state("");
  let validFrom = $state<number | undefined>();
  let validTo = $state<number | undefined>();
  function apply(updateTime = false) {
    if (updateTime) {
      const start = from ? new Date(from).getTime() : undefined;
      const end = to ? new Date(to).getTime() : undefined;
      if (
        (start !== undefined && (!Number.isFinite(start) || start < 0)) ||
        (end !== undefined && (!Number.isFinite(end) || end < 0)) ||
        (start !== undefined && end !== undefined && start > end)
      ) {
        error =
          "Choose a valid time range with From before To. The last valid time range stays active.";
        return;
      }
      error = "";
      validFrom = start;
      validTo = end;
    }
    onapply({
      project,
      agent,
      ...(workflows
        ? { state: workflowState }
        : { from: validFrom, to: validTo }),
    });
  }
  function clear() {
    project = agent = from = to = workflowState = error = "";
    validFrom = validTo = undefined;
    onapply({});
    onclear?.();
  }
</script>

<form
  class="list-filters"
  aria-label={workflows ? "Workflow filters" : "Event filters"}
  onsubmit={(event) => event.preventDefault()}
>
  {#if !sessionScoped}
    <FilterPicker
      view={workflows ? "workflows" : "events"}
      kind="project"
      value={project}
      onselect={(value) => {
        project = value;
        apply();
      }}
      {onexpire}
    />
    <FilterPicker
      view={workflows ? "workflows" : "events"}
      kind="agent"
      value={agent}
      onselect={(value) => {
        agent = value;
        apply();
      }}
      {onexpire}
    />
  {/if}
  {#if workflows}
    <fieldset>
      <legend>Workflow status</legend>
      <div class="status-options">
        {#each ["", "pending", "active", "blocked", "completed"] as status (status)}<button
            type="button"
            aria-pressed={workflowState === status}
            onclick={() => {
              workflowState = status;
              apply();
            }}
            >{status
              ? status[0].toUpperCase() + status.slice(1)
              : "All statuses"}</button
          >{/each}
      </div>
    </fieldset>
  {:else}
    <label
      >From<input
        type="datetime-local"
        value={from}
        min="1970-01-01T00:00"
        onchange={(event) => {
          from = event.currentTarget.value;
          apply(true);
        }}
      /></label
    >
    <label
      >To<input
        type="datetime-local"
        value={to}
        min="1970-01-01T00:00"
        onchange={(event) => {
          to = event.currentTarget.value;
          apply(true);
        }}
      /></label
    >
    <span class="muted">Received time · local timezone</span>
  {/if}
  <button class="quiet" type="button" onclick={clear}>Clear filters</button>
  {#if error}<p role="alert">{error}</p>{/if}
</form>

<style>
  .list-filters {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    align-items: end;
    margin-block: 1rem;
  }
  label {
    display: grid;
    gap: 0.25rem;
  }
  input {
    max-width: 100%;
    min-width: 0;
  }
  fieldset {
    border: 0;
    padding: 0;
    margin: 0;
    min-width: 0;
  }
  .status-options {
    display: flex;
    gap: 0.25rem;
    flex-wrap: wrap;
  }
</style>
