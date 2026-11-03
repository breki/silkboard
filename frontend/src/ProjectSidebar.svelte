<script lang="ts">
  import { onMount } from "svelte";
  import { ApiError } from "./feed-api";
  import { filterChoices, type FilterView } from "./filter-options";
  import type { SelectChoice } from "./list-filters";
  let {
    view,
    value,
    onselect,
    onexpire,
  }: {
    view: FilterView;
    value: string;
    onselect: SelectChoice;
    onexpire: () => void;
  } = $props();
  let wide = $state(false),
    busy = $state(false),
    failed = $state(false);
  let items = $state<string[]>([]),
    after = $state<string | null>(null),
    next = $state<string | null>(null),
    retry = $state(0);
  onMount(() => {
    if (!window.matchMedia) return;
    const media = window.matchMedia("(min-width: 701px)");
    const changed = () => {
      wide = media.matches;
    };
    changed();
    media.addEventListener("change", changed);
    return () => media.removeEventListener("change", changed);
  });
  $effect(() => {
    if (!wide) return;
    const source = view,
      cursor = after;
    void retry;
    const controller = new AbortController();
    busy = true;
    failed = false;
    void filterChoices(source, "project", "", cursor, controller.signal)
      .then((page) => {
        if (controller.signal.aborted) return;
        items =
          cursor === null
            ? page.items
            : [...new Set([...items, ...page.items])];
        next = page.next_cursor;
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
</script>

<aside class="project-sidebar" aria-label="Projects">
  <p class="sidebar-heading">Projects</p>
  <button
    class="quiet project-choice"
    type="button"
    aria-label="Show all projects"
    aria-pressed={!value}
    onclick={() => onselect("")}>All projects</button
  >
  {#if value && !items.includes(value)}<button
      class="quiet project-choice"
      type="button"
      aria-pressed="true"
      onclick={() => onselect(value)}>{value}</button
    >{/if}
  {#each items as project (project)}<button
      class="quiet project-choice"
      type="button"
      aria-pressed={value === project}
      onclick={() => onselect(project)}>{project}</button
    >{/each}
  {#if busy}<p class="muted" role="status">
      Loading projects…
    </p>{:else if failed}<p role="alert">Unable to load projects.</p>
    <button type="button" onclick={() => retry++}>Retry projects</button
    >{:else if !items.length}<p class="muted">No projects with reports.</p>{/if}
  {#if next}<button
      class="quiet"
      type="button"
      disabled={busy}
      onclick={() => (after = next)}>More projects</button
    >{/if}
  <p class="sidebar-note muted">Private workspace<br />Received reports</p>
</aside>

<style>
  .project-sidebar {
    display: none;
  }
  @media (min-width: 701px) {
    .project-sidebar {
      display: block;
      min-width: 0;
      border-right: 1px solid var(--line);
      padding: 1rem 1rem 1rem 0;
    }
    .sidebar-heading {
      text-transform: uppercase;
      letter-spacing: 0.12em;
      font-size: 0.7rem;
      color: var(--muted);
      padding: 0 0.75rem;
      margin: 0 0 1rem;
    }
    .project-choice {
      width: 100%;
      justify-content: start;
      text-align: left;
      margin: 0.15rem 0;
      min-height: 2.75rem;
    }
    .project-choice[aria-pressed="true"] {
      --button-border: transparent;
      color: var(--primary);
    }
    .project-choice[aria-pressed="true"]::before {
      border-width: 0;
    }
    .sidebar-note {
      font-size: 0.75rem;
      padding: 1rem 0.75rem;
    }
  }
</style>
