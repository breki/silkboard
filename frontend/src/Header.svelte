<script lang="ts">
  import { featureLink, locationState } from "./link-state";
  const address = locationState;
  import ThemeSelector from "./ThemeSelector.svelte";
  let {
    owner,
    showMenu = true,
    currentPage,
  }: {
    owner?: { managing: boolean; onmanage: () => void; onlogout: () => void };
    showMenu?: boolean;
    currentPage?: "agents" | "events" | "plans" | "todos";
  } = $props();
  let open = $state(false);
  let container = $state<HTMLDivElement>();
  let trigger = $state<HTMLButtonElement>();
  function close(refocus = false) {
    open = false;
    if (refocus) trigger?.focus();
  }
  function dismissOutside(event: Event) {
    if (
      open &&
      event.target instanceof Node &&
      !container?.contains(event.target)
    )
      close();
  }
  function escape(event: KeyboardEvent) {
    if (open && event.key === "Escape") {
      event.preventDefault();
      close(true);
    }
  }
</script>

<svelte:window
  onpointerdown={dismissOutside}
  onfocusin={dismissOutside}
  onkeydown={escape}
/>
<header class="masthead" class:with-navigation={showMenu}>
  <h1>
    <span class="brand-mark" aria-hidden="true">s</span>Silkboard<span
      class="private-label"
      aria-hidden="true">Private</span
    >
  </h1>
  {#if showMenu}
    <nav class="primary-navigation" aria-label="Primary navigation">
      <a
        href={featureLink("/", $address.search)}
        aria-current={currentPage === "agents" ? "page" : undefined}>Agents</a
      >
      <a
        href={featureLink("/events", $address.search)}
        aria-current={currentPage === "events" ? "page" : undefined}>Events</a
      >
      <a
        href={featureLink("/plans", $address.search)}
        aria-current={currentPage === "plans" ? "page" : undefined}>Workflows</a
      >
      <a
        href={featureLink("/todos", $address.search)}
        aria-current={currentPage === "todos" ? "page" : undefined}>Todos</a
      >
    </nav>
    <div class="navigation" bind:this={container}>
      <button
        class="menu-trigger quiet"
        aria-label="Navigation menu"
        aria-expanded={open}
        aria-controls="main-navigation"
        bind:this={trigger}
        onclick={() => (open = !open)}
      >
        <svg
          viewBox="0 0 24 24"
          width="24"
          height="24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          aria-hidden="true"><path d="M3 6h18M3 12h18M3 18h18" /></svg
        >
      </button>
      {#if open}
        <nav id="main-navigation" aria-label="Main navigation">
          <a href={featureLink("/status", $address.search)}
            >Application status</a
          >
          <ThemeSelector />
          {#if owner}
            <button
              onclick={() => {
                close(true);
                owner.onmanage();
              }}
              >{owner.managing
                ? "Close credential manager"
                : "Manage publisher credentials"}</button
            >
            <button
              onclick={() => {
                close(true);
                owner.onlogout();
              }}>Sign out</button
            >
          {/if}
        </nav>
      {/if}
    </div>
  {:else}
    <ThemeSelector />
  {/if}
</header>
