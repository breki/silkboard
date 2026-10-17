<script lang="ts">
  import { onMount } from "svelte";

  interface Status {
    health: "ok";
    version: string;
    release_tag: string;
    commit: string;
  }

  let status = $state<Status | null>(null);
  let failed = $state(false);

  onMount(() => {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 10000);
    async function load() {
      try {
        const response = await fetch("/api/status", {
          cache: "no-store",
          signal: controller.signal,
        });
        if (!response.ok) throw new Error("Status request failed");
        const data = await response.json();
        if (
          data?.health !== "ok" ||
          ![data.version, data.release_tag, data.commit].every(
            (value) => typeof value === "string" && value.length > 0,
          )
        ) {
          throw new Error("Invalid status response");
        }
        status = data;
      } catch {
        failed = true;
      } finally {
        clearTimeout(timeout);
      }
    }
    void load();
    return () => {
      clearTimeout(timeout);
      controller.abort();
    };
  });
</script>

<main>
  <h1>Silkboard</h1>
  <p>Application status</p>
  {#if failed}
    <p role="alert">Unable to load application status. Reload to try again.</p>
  {:else if status}
    <p class="healthy">Healthy</p>
    <p>The application responded successfully when this page loaded.</p>
    <dl>
      <dt>Version</dt>
      <dd>{status.version}</dd>
      <dt>Release tag</dt>
      <dd>{status.release_tag}</dd>
      <dt>Commit</dt>
      <dd>{status.commit}</dd>
    </dl>
  {:else}
    <p role="status">Loading application status…</p>
  {/if}
</main>
