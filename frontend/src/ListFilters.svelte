<script lang="ts">
  import type { ApplyFilters } from "./list-filters";
  let {
    workflows = false,
    onapply,
  }: { workflows?: boolean; onapply: ApplyFilters } = $props();
  let project = $state("");
  let agent = $state("");
  let from = $state("");
  let to = $state("");
  let workflowState = $state("");
  let error = $state("");
  function apply(event: SubmitEvent) {
    event.preventDefault();
    const start = from ? new Date(from).getTime() : undefined;
    const end = to ? new Date(to).getTime() : undefined;
    if (
      (start !== undefined && !Number.isFinite(start)) ||
      (end !== undefined && !Number.isFinite(end)) ||
      (start !== undefined && end !== undefined && start > end)
    ) {
      error = "Choose a valid time range with From before To.";
      return;
    }
    error = "";
    onapply({
      project: project.trim(),
      agent: agent.trim(),
      ...(workflows ? { state: workflowState } : { from: start, to: end }),
    });
  }
  function clear() {
    project = agent = from = to = workflowState = error = "";
    onapply({});
  }
</script>

<form
  class="list-filters"
  aria-label={workflows ? "Workflow filters" : "Event filters"}
  onsubmit={apply}
>
  <label
    >Project<input
      bind:value={project}
      maxlength="64"
      pattern="[a-z0-9-]+"
      placeholder="All projects"
    /></label
  >
  <label
    >Agent<input
      bind:value={agent}
      maxlength="513"
      placeholder="Tool or host"
    /></label
  >
  {#if workflows}
    <label
      >Workflow status<select bind:value={workflowState}
        ><option value="">All statuses</option
        >{#each ["pending", "active", "blocked", "completed"] as status (status)}<option
            value={status}>{status}</option
          >{/each}</select
      ></label
    >
  {:else}
    <label
      >From<input
        type="datetime-local"
        bind:value={from}
        min="1970-01-01T00:00"
      /></label
    >
    <label
      >To<input
        type="datetime-local"
        bind:value={to}
        min="1970-01-01T00:00"
      /></label
    >
    <span class="muted">Received time · local timezone</span>
  {/if}
  <button type="submit">Apply filters</button>
  <button type="button" onclick={clear}>Clear filters</button>
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
  input,
  select {
    max-width: 100%;
    min-width: 0;
  }
</style>
