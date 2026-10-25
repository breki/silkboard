<script lang="ts">
  import type { PlanLinkHandler } from "./plan-model";
  import { onMount, tick } from "svelte";
  import type { PlanDetail } from "./plan-api";
  import { graphLayout, stateLabel, stateSymbol } from "./plan-model";
  import StepDetails from "./StepDetails.svelte";
  let {
    detail,
    pinned,
    onopen,
  }: {
    detail: PlanDetail;
    pinned: boolean;
    onopen: PlanLinkHandler;
  } = $props();
  const uid = $props.id();
  let presentation = $state<"graph" | "steps">("graph");
  let selectedId = $state<string | null>(null);
  let mobile = $state(false);
  let detailOpen = $state(false);
  let root = $state<HTMLDivElement>();
  let dialog = $state<HTMLDialogElement>();
  let heading = $state<HTMLHeadingElement>();
  let aside = $state<HTMLElement>();
  let removed = $state("");
  const steps = $derived(detail.snapshot.steps);
  const selected = $derived(steps.find((s) => s.id === selectedId));
  const layout = $derived(graphLayout(steps, mobile ? 1 : 3));
  const byId = $derived(new Map(steps.map((s) => [s.id, s])));
  const active = $derived(steps.filter((s) => s.state === "active"));
  function node(id: string | null) {
    return [
      ...(root?.querySelectorAll<HTMLButtonElement>("button[data-step]") ?? []),
    ].find((button) => button.dataset.step === id);
  }
  function focusSelected() {
    (node(selectedId) ?? heading)?.focus();
  }
  $effect.pre(() => {
    const focused = document.activeElement;
    const focusedId =
      focused instanceof HTMLElement ? focused.dataset.step : undefined;
    const removedNode =
      focusedId && !steps.some((step) => step.id === focusedId);
    const removedDetails =
      selectedId !== null &&
      !steps.some((step) => step.id === selectedId) &&
      (dialog?.open || aside?.contains(focused));
    if (removedNode || removedDetails) void tick().then(() => heading?.focus());
  });
  async function select(id: string, locate = false) {
    selectedId = id;
    removed = "";
    detailOpen = mobile;
    await tick();
    if (locate) {
      node(id)?.scrollIntoView({ block: "nearest", inline: "nearest" });
      node(id)?.focus();
    }
  }
  function closeDetails() {
    detailOpen = false;
    dialog?.close();
    void tick().then(focusSelected);
  }
  $effect(() => {
    if (selectedId !== null && !selected) {
      const lostFocus =
        !!dialog?.open || !!aside?.contains(document.activeElement);
      selectedId = null;
      detailOpen = false;
      removed = "The selected step is not in this snapshot.";
      if (lostFocus)
        void tick().then(() => {
          // Close first: native dialog focus restoration must not override
          // the fallback when the originating step no longer exists.
          dialog?.close();
          heading?.focus();
        });
    }
  });
  $effect(() => {
    const open = mobile && detailOpen && !!selected;
    let cancelled = false;
    void tick().then(() => {
      if (cancelled || !dialog?.isConnected) return;
      if (open && !dialog.open) dialog.showModal();
      else if (!open && dialog.open) dialog.close();
    });
    return () => {
      cancelled = true;
    };
  });
  onMount(() => {
    if (typeof window.matchMedia !== "function") return;
    const media = window.matchMedia("(max-width: 700px)");
    const resize = () => {
      const wasDialog = !!dialog?.open;
      const focusedDetails = !!aside?.contains(document.activeElement);
      mobile = media.matches;
      if (mobile && focusedDetails) detailOpen = true;
      else if (!mobile && wasDialog) {
        detailOpen = false;
        void tick().then(focusSelected);
      }
    };
    resize();
    media.addEventListener("change", resize);
    return () => {
      media.removeEventListener("change", resize);
      if (dialog?.open) dialog.close();
    };
  });
</script>

<div class="plan-toolbar" aria-label="Plan presentation">
  <button
    aria-pressed={presentation === "graph"}
    onclick={() => (presentation = "graph")}>Graph</button
  >
  <button
    aria-pressed={presentation === "steps"}
    onclick={() => (presentation = "steps")}>Steps</button
  >
  {#if active.length === 1}<button
      onclick={() => void select(active[0].id, true)}>Locate active step</button
    >
  {:else if active.length > 1}
    <label class="active-picker"
      >Active step<select
        aria-label="Choose active step"
        value=""
        onchange={(event) => {
          const id = event.currentTarget.value;
          event.currentTarget.value = "";
          if (id) void select(id, true);
        }}
        ><option value="">Choose one of {active.length} active steps</option
        >{#each active as step (step.id)}<option value={step.id}
            >{step.title}</option
          >{/each}</select
      ></label
    >
  {:else}<span class="muted">No step is marked active in this snapshot.</span
    >{/if}
  {#if mobile && selected && !detailOpen}<button
      onclick={() => (detailOpen = true)}>View selected step</button
    >{/if}
</div>
<p class="plan-legend">○ Pending · ● Active · ✓ Done · ! Blocked</p>
{#if removed}<p role="status">{removed}</p>{/if}
<div class="plan-workspace" bind:this={root}>
  <section class="plan-visual">
    <h3 tabindex="-1" bind:this={heading}>Steps and prerequisites</h3>
    {#if presentation === "graph"}
      <p class="muted">
        Arrows lead from a prerequisite to the step that depends on it.
      </p>
      <div
        class="graph-scroll"
        role="region"
        aria-label="Task prerequisite graph"
      >
        <div
          class="graph-canvas"
          style:width={`${layout.width}px`}
          style:height={`${layout.height}px`}
        >
          <svg
            class="graph-edges"
            width={layout.width}
            height={layout.height}
            aria-hidden="true"
          >
            <defs
              ><marker
                id={`${uid}-arrow`}
                viewBox="0 0 10 10"
                refX="9"
                refY="5"
                markerWidth="7"
                markerHeight="7"
                orient="auto-start-reverse"><path d="M0 0L10 5L0 10z" /></marker
              ></defs
            >
            {#each layout.edges as edge (`${edge.from}/${edge.to}`)}<path
                class="prerequisite-edge"
                d={edge.path}
                marker-end={`url(#${uid}-arrow)`}
                data-from={edge.from}
                data-to={edge.to}
              />{/each}
          </svg>
          {#each layout.nodes as position (position.id)}
            {@const step = byId.get(position.id)!}
            <button
              class="graph-node"
              data-step={step.id}
              data-state={step.state}
              aria-pressed={selectedId === step.id}
              aria-label={`${step.title}, ${stateLabel[step.state]}`}
              title={step.title}
              style:left={`${position.x}px`}
              style:top={`${position.y}px`}
              onclick={() => void select(step.id)}
            >
              <span class="node-title">{step.title}</span><span
                class="node-state"
                ><span aria-hidden="true">{stateSymbol[step.state]}</span>
                {stateLabel[step.state]}</span
              >
            </button>
          {/each}
        </div>
      </div>
    {:else}
      <ol class="ordered-steps" aria-label="Steps in prerequisite order">
        {#each layout.nodes as position (position.id)}
          {@const step = byId.get(position.id)!}
          <li>
            <button
              data-step={step.id}
              data-state={step.state}
              aria-pressed={selectedId === step.id}
              aria-label={`${step.title}, ${stateLabel[step.state]}`}
              onclick={() => void select(step.id)}
              ><span aria-hidden="true">{stateSymbol[step.state]}</span>
              {step.title} · {stateLabel[step.state]}</button
            >
            <p class="muted">
              Prerequisites: {step.depends_on.length
                ? step.depends_on.map((id) => byId.get(id)!.title).join(", ")
                : "None"}
            </p>
          </li>
        {/each}
      </ol>
    {/if}
  </section>
  {#if !mobile}<aside
      class="step-side panel"
      aria-label="Step details"
      bind:this={aside}
    >
      {#if selected}<div>
          <StepDetails {detail} step={selected} {pinned} {onopen} />
        </div>{:else}<h3>Step details</h3>
        <p>Choose a step to see its report and prerequisites.</p>{/if}
    </aside>{/if}
</div>
<dialog
  class="step-dialog"
  aria-label="Step details"
  bind:this={dialog}
  oncancel={(event) => {
    event.preventDefault();
    closeDetails();
  }}
  onclose={() => (detailOpen = false)}
>
  {#if mobile && selected && detailOpen}<button
      class="close-step"
      onclick={closeDetails}>Close step details</button
    ><StepDetails {detail} step={selected} {pinned} {onopen} />{/if}
</dialog>
