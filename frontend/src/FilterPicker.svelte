<script lang="ts">
  import { tick } from "svelte";
  import { ApiError } from "./feed-api";
  import {
    filterChoices,
    type FilterView,
    type ChoiceKind,
  } from "./filter-options";
  import type { SelectChoice } from "./list-filters";
  let {
    view,
    kind,
    value,
    onselect,
    onexpire,
  }: {
    view: FilterView;
    kind: ChoiceKind;
    value: string;
    onselect: SelectChoice;
    onexpire: () => void;
  } = $props();
  const label = $derived(kind === "project" ? "Project" : "Agent");
  const plural = $derived(kind === "project" ? "projects" : "agents");
  const id = $derived(`${view}-${kind}-choices`);
  let open = $state(false);
  let search = $state("");
  let after = $state<string | null>(null);
  let next = $state<string | null>(null);
  let items = $state<string[]>([]);
  let busy = $state(false);
  let failed = $state(false);
  let retry = $state(0);
  let container = $state<HTMLDivElement>();
  let trigger = $state<HTMLButtonElement>();
  let input = $state<HTMLInputElement>();
  const display = (choice: string) =>
    kind === "agent" ? choice.replace(" ", " · ") : choice;
  $effect(() => {
    if (!open) return;
    const query = search;
    const cursor = after;
    const source = view;
    const field = kind;
    void retry;
    const controller = new AbortController();
    if (cursor === null) items = [];
    busy = true;
    failed = false;
    next = null;
    const timer = setTimeout(async () => {
      try {
        const page = await filterChoices(
          source,
          field,
          query,
          cursor,
          controller.signal,
        );
        if (controller.signal.aborted) return;
        items =
          cursor === null
            ? page.items
            : [...new Set([...items, ...page.items])];
        next = page.next_cursor;
      } catch (error) {
        if (controller.signal.aborted) return;
        if (error instanceof ApiError && error.status === 401) onexpire();
        else failed = true;
      } finally {
        if (!controller.signal.aborted) busy = false;
      }
    }, 200);
    return () => {
      clearTimeout(timer);
      controller.abort();
    };
  });
  async function toggle() {
    if (open) {
      open = false;
      return;
    }
    search = "";
    after = null;
    open = true;
    await tick();
    input?.focus();
  }
  function choose(choice: string) {
    onselect(choice);
    open = false;
    trigger?.focus();
  }
  function outside(event: Event) {
    if (
      open &&
      event.target instanceof Node &&
      !container?.contains(event.target)
    )
      open = false;
  }
  function escape(event: KeyboardEvent) {
    if (open && event.key === "Escape") {
      event.preventDefault();
      open = false;
      trigger?.focus();
    }
  }
</script>

<svelte:window onpointerdown={outside} onfocusin={outside} onkeydown={escape} />
<div class="filter-picker" bind:this={container}>
  <span>{label}</span>
  <button
    type="button"
    aria-label={`${label}: ${value ? display(value) : `All ${plural}`}`}
    aria-expanded={open}
    aria-controls={id}
    bind:this={trigger}
    onclick={() => void toggle()}
    >{value ? display(value) : `All ${plural}`}</button
  >
  {#if open}
    <section class="picker-options" {id} aria-label={`${label} choices`}>
      <label
        >Search {plural}<input
          bind:this={input}
          value={search}
          maxlength="513"
          oninput={(event) => {
            search = event.currentTarget.value;
            after = null;
          }}
        /></label
      >
      <button
        class="quiet"
        type="button"
        aria-pressed={!value}
        onclick={() => choose("")}>All {plural}</button
      >
      <ul aria-label={`Available ${plural}`}>
        {#each items as choice (choice)}<li>
            <button
              class="quiet"
              type="button"
              aria-pressed={value === choice}
              onclick={() => choose(choice)}>{display(choice)}</button
            >
          </li>{/each}
      </ul>
      {#if busy}<p role="status">Loading {plural}…</p>{:else if failed}<p
          role="alert"
        >
          Unable to load {plural}.
        </p>
        <button type="button" onclick={() => retry++}>Retry {plural}</button
        >{:else if !items.length}<p>No matching {plural}.</p>{/if}
      {#if next}<button
          type="button"
          disabled={busy}
          onclick={() => (after = next)}>More {plural}</button
        >{/if}
    </section>
  {/if}
</div>

<style>
  .filter-picker {
    display: grid;
    gap: 0.25rem;
    position: relative;
    min-width: 0;
    max-width: 100%;
  }
  button {
    overflow-wrap: anywhere;
    white-space: normal;
  }
  .picker-options {
    position: absolute;
    inset-block-start: 100%;
    inset-inline-start: 0;
    z-index: 5;
    width: min(20rem, calc(100vw - 3rem));
    max-height: 65vh;
    overflow: auto;
    padding: 0.75rem;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    box-shadow: 0 0.5rem 1.5rem var(--shadow);
  }
  label {
    display: grid;
    gap: 0.25rem;
  }
  input {
    width: 100%;
    box-sizing: border-box;
  }
  ul {
    list-style: none;
    padding: 0;
    margin: 0.5rem 0;
  }
  li button {
    width: 100%;
    text-align: start;
  }
  @media (max-width: 600px) {
    .picker-options {
      position: fixed;
      inset-inline: 1rem;
      inset-block-start: 20vh;
      width: auto;
    }
  }
</style>
