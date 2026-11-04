<script lang="ts">
  import type { Snippet } from "svelte";
  import type { ApplyFilters } from "./list-filters";
  import type { AgentActivity } from "./agents-api";
  import type { ReceivedRange } from "./time-range";
  import FilterPicker from "./FilterPicker.svelte";
  import ProjectSidebar from "./ProjectSidebar.svelte";
  import AgentPicker from "./AgentPicker.svelte";
  import TimeRange from "./TimeRange.svelte";
  let {
    workflows = false,
    agentsView = false,
    onapply,
    onexpire,
    sessionScoped = false,
    disabled = false,
    onclear,
    children,
    heading,
  }: {
    workflows?: boolean;
    agentsView?: boolean;
    onapply: ApplyFilters;
    onexpire: () => void;
    sessionScoped?: boolean;
    disabled?: boolean;
    onclear?: () => void;
    children?: Snippet;
    heading?: Snippet;
  } = $props();
  const view = $derived(
    agentsView ? "agents" : workflows ? "workflows" : "events",
  );
  let project = $state(""),
    agent = $state(""),
    workflowState = $state("");
  let selected = $state<AgentActivity | null>(null);
  let range = $state<ReceivedRange>({});
  function apply() {
    onapply(
      agentsView
        ? {
            project: selected?.project ?? project,
            ...(selected
              ? {
                  hostname: selected.hostname,
                  tool: selected.tool,
                  session_id: selected.session_id,
                }
              : {}),
          }
        : { project, agent, ...(workflows ? { state: workflowState } : range) },
    );
  }
  function chooseProject(value: string) {
    project = value;
    agent = "";
    selected = null;
    apply();
  }
  function clear() {
    project = agent = workflowState = "";
    selected = null;
    range = {};
    onapply({});
    onclear?.();
  }
</script>

<div class="project-layout" class:unscoped={sessionScoped || disabled}>
  {#if !sessionScoped && !disabled}<ProjectSidebar
      {view}
      value={project}
      onselect={chooseProject}
      {onexpire}
    />{/if}
  <div class="project-results">
    {#if heading}{@render heading()}{/if}
    {#if !disabled}<div
        class="list-filters"
        role="group"
        aria-label={agentsView
          ? "Agent filters"
          : workflows
            ? "Workflow filters"
            : "Event filters"}
      >
        {#if !sessionScoped}
          <div class="mobile-project">
            <FilterPicker
              {view}
              kind="project"
              value={project}
              onselect={chooseProject}
              {onexpire}
            />
          </div>
          {#if agentsView}<AgentPicker
              {project}
              value={selected}
              onselect={(value) => {
                selected = value;
                apply();
              }}
              {onexpire}
            />{:else}<FilterPicker
              {view}
              kind="agent"
              value={agent}
              onselect={(value) => {
                agent = value;
                apply();
              }}
              {onexpire}
            />{/if}
        {/if}
        {#if workflows}<fieldset>
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
        {:else if !agentsView}<TimeRange
            value={range}
            onchange={(value) => {
              range = value;
              apply();
            }}
          />{/if}
        <button class="quiet" type="button" onclick={clear}
          >Clear filters</button
        >
        <span class="filter-note muted"
          >{agentsView
            ? "Main sessions only"
            : workflows
              ? "Reported states"
              : "Received time · local timezone"}</span
        >
      </div>{/if}
    {#if children}{@render children()}{/if}
  </div>
</div>

<style>
  .list-filters {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
    margin: 0 0 1.5rem;
  }
  .filter-note {
    margin-left: auto;
    font-size: 0.75rem;
  }
  fieldset {
    border: 0;
    padding: 0;
    margin: 0;
    min-width: 0;
  }
  legend {
    font-size: 0.75rem;
    color: var(--muted);
  }
  .status-options {
    display: flex;
    gap: 0.25rem;
    flex-wrap: wrap;
  }
  @media (min-width: 701px) {
    .mobile-project {
      display: none;
    }
  }
  @media (max-width: 700px) {
    .list-filters {
      align-items: stretch;
    }
    .mobile-project {
      width: 100%;
    }
    .filter-note {
      width: 100%;
      margin-left: 0;
    }
  }
</style>
