<script lang="ts">
  import { locationState, readView } from "./link-state";
  import LinkError from "./LinkError.svelte";
  const address = locationState;
  import OwnerFeed from "./OwnerFeed.svelte";
  import Header from "./Header.svelte";
  import OwnerSetup from "./OwnerSetup.svelte";
  let {
    setupPage = false,
    unknownPage = false,
    statusPage = false,
    plansPage = false,
    eventsPage = false,
    todosPage = false,
    takeSetupCredential = () => "",
  }: {
    setupPage?: boolean;
    unknownPage?: boolean;
    statusPage?: boolean;
    plansPage?: boolean;
    eventsPage?: boolean;
    todosPage?: boolean;
    takeSetupCredential?: () => string;
  } = $props();
</script>

<main>
  {#if unknownPage}<Header />
    <section class="panel">
      <h2>Page not found</h2>
      <p>This application link is unavailable.</p>
      <a href="/">Open Agents</a>
    </section>
  {:else if setupPage}
    <Header showMenu={false} />
    {#if readView("setup", $address.search).valid}<OwnerSetup
        takeCredential={takeSetupCredential}
      />{:else}<LinkError />{/if}
  {:else}
    <OwnerFeed {statusPage} {plansPage} {eventsPage} {todosPage} />
  {/if}
</main>
