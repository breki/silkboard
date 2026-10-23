<script lang="ts">
  import { onMount } from "svelte";
  import { ApiError, ownerSession, signIn, signOut } from "./feed-api";
  import EventFeed from "./EventFeed.svelte";
  import PublisherTokens from "./PublisherTokens.svelte";
  import Header from "./Header.svelte";
  import StatusCard from "./StatusCard.svelte";
  import { poll } from "./poll";
  let { statusPage = false }: { statusPage?: boolean } = $props();
  let managing = $state(false);

  type Phase =
    | "checking"
    | "signed-out"
    | "signing-in"
    | "signed-in"
    | "signing-out"
    | "logout-failed"
    | "unavailable"
    | "disabled";
  let phase = $state<Phase>("checking");
  let password = $state("");
  let notice = $state("");
  let alive = true;
  let active = new AbortController();
  $effect(() => {
    if (!statusPage || phase !== "signed-in") return;
    return poll(async (signal) => {
      try {
        await ownerSession(signal);
      } catch (error) {
        if (
          !signal.aborted &&
          error instanceof ApiError &&
          error.status === 401
        )
          expired();
      }
    });
  });
  function begin() {
    active.abort();
    active = new AbortController();
    return active.signal;
  }
  async function check() {
    const signal = begin();
    phase = "checking";
    try {
      await ownerSession(signal);
      if (!signal.aborted && alive) phase = "signed-in";
    } catch (error) {
      if (signal.aborted || !alive) return;
      phase =
        error instanceof ApiError && error.status === 401
          ? "signed-out"
          : error instanceof ApiError && error.status === 404
            ? "disabled"
            : "unavailable";
    }
  }
  function expired() {
    managing = false;
    begin();
    password = "";
    notice = "Your session expired. Sign in again.";
    phase = "signed-out";
  }
  async function login(event: SubmitEvent) {
    event.preventDefault();
    const value = password;
    password = "";
    notice = "";
    phase = "signing-in";
    const signal = begin();
    try {
      await signIn(value, signal);
      if (!signal.aborted && alive) await check();
    } catch (error) {
      if (signal.aborted || !alive) return;
      notice =
        error instanceof ApiError && error.status === 429
          ? "Too many sign-in attempts. Wait a minute and try again."
          : error instanceof ApiError && error.status === 401
            ? "Password not accepted. Try again."
            : "Unable to sign in. Try again.";
      phase = "signed-out";
    }
  }
  async function logout() {
    managing = false;
    const signal = begin();
    phase = "signing-out";
    notice = "";
    try {
      await signOut(signal);
      if (!signal.aborted && alive) phase = "signed-out";
    } catch (error) {
      if (signal.aborted || !alive) return;
      phase =
        error instanceof ApiError && error.status === 401
          ? "signed-out"
          : "logout-failed";
    }
  }
  onMount(() => {
    void check();
    return () => {
      alive = false;
      active.abort();
    };
  });
</script>

<Header
  owner={phase === "signed-in"
    ? {
        managing,
        onmanage: () => (managing = !managing),
        onlogout: () => void logout(),
      }
    : undefined}
/>
{#if phase === "signed-in" && managing}<PublisherTokens
    onexpire={expired}
  />{/if}
{#if !statusPage || phase === "logout-failed"}
  <section class="feed" aria-label="Progress feed">
    {#if phase === "signed-in"}
      <EventFeed onexpire={expired} />
    {:else if phase === "signed-out"}
      <div class="panel login">
        <h2>Sign in to your progress feed</h2>
        <p>Follow what your development agents are working on.</p>
        {#if notice}<p role="alert">{notice}</p>{/if}
        <form onsubmit={login}>
          <label for="owner-password">Owner password</label><input
            id="owner-password"
            type="password"
            autocomplete="current-password"
            maxlength="1024"
            bind:value={password}
            required
          />
          <button class="primary" type="submit">Sign in</button>
        </form>
      </div>
    {:else if phase === "logout-failed"}
      <div class="panel">
        <p role="alert">
          Sign-out could not be confirmed. Your feed is hidden. Try again.
        </p>
        <button onclick={() => void logout()}>Retry sign out</button>
      </div>
    {:else if phase === "disabled"}
      <div class="panel">
        <p>Progress feed is not enabled on this server.</p>
        <button onclick={() => void check()}>Check again</button>
      </div>
    {:else if phase === "unavailable"}
      <div class="panel">
        <p role="alert">Unable to check your sign-in. Try again.</p>
        <button onclick={() => void check()}>Retry sign in</button>
      </div>
    {:else}
      <p role="status">
        {phase === "signing-in"
          ? "Signing in…"
          : phase === "signing-out"
            ? "Signing out…"
            : "Checking your sign-in…"}
      </p>
    {/if}
  </section>
{/if}

{#if statusPage}
  {#if notice}<p role="alert">
      {notice} <a href="/">Return to sign in</a>
    </p>{/if}
  <StatusCard />
{/if}
