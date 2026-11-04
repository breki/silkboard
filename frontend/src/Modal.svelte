<script lang="ts">
  import type { Snippet } from "svelte";
  let {
    open,
    title,
    onclose,
    children,
  }: { open: boolean; title: string; onclose: () => void; children: Snippet } =
    $props();
  const id = $props.id();
  let dialog = $state<HTMLDialogElement>();
  $effect(() => {
    if (open && dialog && !dialog.open) {
      dialog.showModal();
      dialog.querySelector<HTMLElement>("[data-initial-focus]")?.focus();
    } else if (!open && dialog?.open) dialog.close();
  });
  function keyboard(event: KeyboardEvent) {
    if (!dialog?.open) return;
    if (event.key === "Escape") {
      event.preventDefault();
      onclose();
      return;
    }
    if (event.key !== "Tab") return;
    const targets = [...dialog.querySelectorAll<HTMLElement>("*")].filter(
      (node) =>
        node.matches("button, input, select, a[href]") &&
        !node.matches(":disabled") &&
        node.getClientRects().length,
    );
    const first = targets[0],
      last = targets.at(-1);
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last?.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first?.focus();
    }
  }
</script>

<svelte:window onkeydown={keyboard} />
<dialog
  bind:this={dialog}
  aria-labelledby={id}
  oncancel={(event) => {
    event.preventDefault();
    onclose();
  }}
>
  <div class="modal-heading">
    <h2 {id}>{title}</h2>
    <button
      class="quiet modal-close"
      type="button"
      aria-label={`Close ${title.toLowerCase()}`}
      onclick={onclose}>×</button
    >
  </div>
  {#if open}{@render children()}{/if}
</dialog>

<style>
  dialog {
    width: min(30rem, calc(100vw - 1.5rem));
    max-height: calc(100dvh - 2rem);
    padding: 1.5rem;
    border: 1px solid var(--line);
    border-radius: 0.9rem;
    background: var(--panel);
    color: var(--text);
    box-shadow: 0 1.5rem 5rem var(--shadow);
  }
  dialog::backdrop {
    background: var(--backdrop);
  }
  .modal-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }
  h2 {
    font-size: 1.3rem;
    margin: 0;
  }
  .modal-close {
    font-size: 1.5rem;
    flex-shrink: 0;
  }
  @media (max-width: 40rem) {
    dialog {
      padding: 1.25rem 1rem;
    }
  }
</style>
