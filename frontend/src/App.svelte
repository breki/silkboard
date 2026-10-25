<script lang="ts">
  import OwnerFeed from "./OwnerFeed.svelte";
  import Header from "./Header.svelte";
  import OwnerSetup from "./OwnerSetup.svelte";
  let {
    setupPage = false,
    statusPage = false,
    plansPage = false,
    eventsPage = false,
    takeSetupCredential = () => "",
  }: {
    setupPage?: boolean;
    statusPage?: boolean;
    plansPage?: boolean;
    eventsPage?: boolean;
    takeSetupCredential?: () => string;
  } = $props();
</script>

<main>
  {#if setupPage}
    <Header showMenu={false} />
    <OwnerSetup takeCredential={takeSetupCredential} />
  {:else}
    <OwnerFeed {statusPage} {plansPage} {eventsPage} />
  {/if}
</main>
