<script lang="ts">
  import { ApiError, projects, type Page, type Project } from "./feed-api";
  import { poll } from "./poll";
  import SessionList from "./SessionList.svelte";

  let { onexpire }: { onexpire: () => void } = $props();
  let page = $state<Page<Project> | null>(null);
  let selected = $state("");
  let after = $state<string | null>(null);
  let failed = $state(false);
  let retry = $state(0);
  $effect(() => {
    const cursor = after;
    void retry;
    return poll(async (signal) => {
      try {
        const value = await projects(cursor, signal);
        if (signal.aborted) return;
        page = value;
        failed = false;
        if (!selected) selected = value.items[0]?.id ?? "";
      } catch (error) {
        if (signal.aborted) return;
        if (error instanceof ApiError && error.status === 401) onexpire();
        else failed = true;
      }
    });
  });
  function navigate(cursor: string | null) {
    page = null;
    after = cursor;
  }
</script>

{#if failed}<div class="notice">
    <p role="alert">Unable to update projects. Try again.</p>
    <button onclick={() => retry++}>Retry projects</button>
  </div>{/if}
{#if page}
  {#if page.items.length || selected}
    <div class="project-picker">
      <label for="project">Project</label><select
        id="project"
        bind:value={selected}
      >
        {#if selected && !page.items.some((p) => p.id === selected)}<option
            value={selected}>{selected}</option
          >{/if}
        {#each page.items as project (project.id)}<option value={project.id}
            >{project.id}</option
          >{/each}
      </select>
    </div>
  {:else}<p class="empty">
      No projects yet. Progress will appear after an agent publishes.
    </p>{/if}
  <div class="page-controls">
    {#if after !== null}<button onclick={() => navigate(null)}
        >First projects</button
      >{/if}
    {#if page.next_cursor}<button onclick={() => navigate(page!.next_cursor)}
        >More projects</button
      >{/if}
  </div>
{:else if !failed}<p role="status">Loading projects…</p>{/if}
{#if selected}
  {#key selected}<SessionList project={selected} {onexpire} />{/key}
{/if}
