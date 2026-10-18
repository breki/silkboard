<script lang="ts">
  import OwnerFeed from "./OwnerFeed.svelte";
  import StatusCard from "./StatusCard.svelte";
  import OwnerSetup from "./OwnerSetup.svelte";
  let {
    setupPage = false,
    takeSetupCredential = () => "",
  }: { setupPage?: boolean; takeSetupCredential?: () => string } = $props();
</script>

<main>
  <header class="masthead">
    <p class="eyebrow">Private development dashboard</p>
    <h1>Silkboard</h1>
  </header>
  {#if setupPage}
    <OwnerSetup takeCredential={takeSetupCredential} />
  {:else}
    <OwnerFeed />
    <StatusCard />
  {/if}
</main>
