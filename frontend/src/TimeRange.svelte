<script lang="ts">
  import Modal from "./Modal.svelte";
  import {
    localFields,
    parseLocalEndpoint,
    periods,
    presetRange,
    type Period,
    type ReceivedRange,
  } from "./time-range";
  let {
    value,
    onchange,
    // eslint-disable-next-line no-unused-vars -- Callback parameter describes the contract.
  }: { value: ReceivedRange; onchange: (value: ReceivedRange) => void } =
    $props();
  const id = $props.id();
  let open = $state(false);
  let trigger = $state<HTMLButtonElement>();
  let fromDate = $state(""),
    fromTime = $state(""),
    toDate = $state(""),
    toTime = $state("");
  let error = $state("");
  let period = $state<Period | "custom">("all");
  $effect(() => {
    if (value.from === undefined && value.to === undefined) period = "all";
  });
  const label = $derived(
    value.from === undefined && value.to === undefined
      ? "All time"
      : period !== "custom" && period !== "all"
        ? periods.find((p) => p.id === period)!.label
        : `${value.from === undefined ? "Any time" : `${localFields(value.from).date} ${localFields(value.from).time}`} → ${value.to === undefined ? "Any time" : `${localFields(value.to).date} ${localFields(value.to).time}`}`,
  );
  const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  function show() {
    const from =
      value.from === undefined
        ? { date: "", time: "" }
        : localFields(value.from);
    const to =
      value.to === undefined ? { date: "", time: "" } : localFields(value.to);
    fromDate = from.date;
    fromTime = from.time;
    toDate = to.date;
    toTime = to.time;
    error = "";
    open = true;
  }
  function close() {
    open = false;
    trigger?.focus();
  }
  function select(selected: Period) {
    period = selected;
    onchange(presetRange(selected, Date.now()));
    close();
  }
  function apply(event: SubmitEvent) {
    event.preventDefault();
    try {
      const from = parseLocalEndpoint(fromDate.trim(), fromTime.trim()),
        to = parseLocalEndpoint(toDate.trim(), toTime.trim());
      if (from !== undefined && to !== undefined && from > to)
        throw new Error("Reversed range");
      period = "custom";
      onchange({ from, to });
      close();
    } catch {
      error =
        "Enter valid dates and 24-hour times, with From before To. The last valid time range stays active.";
    }
  }
</script>

<button
  type="button"
  class="range-trigger"
  bind:this={trigger}
  aria-label={`Received time: ${label}`}
  aria-haspopup="dialog"
  onclick={show}
  ><span aria-hidden="true">◷</span>{label}<span aria-hidden="true">⌄</span
  ></button
>
<Modal {open} title="Received time" onclose={close}>
  <p class="muted range-intro">
    Choose a recent period or enter a precise range.
  </p>
  <div class="range-presets" aria-label="Recent periods">
    {#each periods as item (item.id)}<button
        type="button"
        aria-pressed={period === item.id &&
          (item.id !== "all" ||
            (value.from === undefined && value.to === undefined))}
        onclick={() => select(item.id)}>{item.label}</button
      >{/each}
  </div>
  <form onsubmit={apply} novalidate>
    <fieldset>
      <legend>Custom range</legend>
      <div class="range-endpoint">
        <span>From</span><label for={`${id}-from-date`}>Date</label><label
          for={`${id}-from-time`}>Time</label
        ><input
          id={`${id}-from-date`}
          aria-label="From date"
          placeholder="YYYY-MM-DD"
          bind:value={fromDate}
          autocomplete="off"
        /><input
          id={`${id}-from-time`}
          aria-label="From time"
          placeholder="HH:mm"
          bind:value={fromTime}
          autocomplete="off"
          inputmode="numeric"
        />
      </div>
      <div class="range-endpoint">
        <span>To</span><label for={`${id}-to-date`}>Date</label><label
          for={`${id}-to-time`}>Time</label
        ><input
          id={`${id}-to-date`}
          aria-label="To date"
          placeholder="YYYY-MM-DD"
          bind:value={toDate}
          autocomplete="off"
        /><input
          id={`${id}-to-time`}
          aria-label="To time"
          placeholder="HH:mm"
          bind:value={toTime}
          autocomplete="off"
          inputmode="numeric"
        />
      </div>
    </fieldset>
    <p class="muted">
      Received time · {zone}. Dates: YYYY-MM-DD. Times: 24-hour HH:mm. Leave
      both fields of an endpoint empty for no limit.
    </p>
    {#if error}<p role="alert">{error}</p>{/if}
    <div class="range-actions">
      <button type="button" onclick={close}>Cancel</button><button
        class="primary"
        type="submit">Apply range</button
      >
    </div>
  </form>
</Modal>

<style>
  .range-trigger {
    justify-content: space-between;
    gap: 0.75rem;
    text-align: left;
  }
  .range-intro {
    margin: 1rem 0;
  }
  .range-presets {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }
  .range-presets button {
    width: 100%;
    justify-content: start;
  }
  fieldset {
    border: 0;
    border-top: 1px solid var(--line);
    padding: 1rem 0 0;
    margin: 0;
    min-width: 0;
  }
  legend {
    font-size: 0.8rem;
    color: var(--muted);
    padding-right: 0.75rem;
  }
  .range-endpoint {
    display: grid;
    grid-template-columns: 2.5rem minmax(0, 1fr) 6rem;
    gap: 0.35rem 0.5rem;
    margin-bottom: 1rem;
  }
  .range-endpoint > span {
    grid-row: span 2;
    align-self: center;
    font-size: 0.85rem;
  }
  .range-endpoint label {
    font-weight: 400;
    font-size: 0.75rem;
    color: var(--muted);
  }
  .range-endpoint input {
    min-width: 0;
  }
  .range-actions {
    display: flex;
    justify-content: end;
    gap: 0.5rem;
    padding-top: 1rem;
    border-top: 1px solid var(--line);
  }
  @media (max-width: 40rem) {
    .range-endpoint {
      grid-template-columns: 2rem minmax(0, 1fr) 5rem;
    }
    .range-endpoint input {
      font-size: 0.8rem;
      padding-inline: 0.4rem;
    }
  }
</style>
