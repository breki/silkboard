<script lang="ts">
  import { onMount } from "svelte";
  import { ApiError, ownerSession, signIn, signOut } from "./feed-api";
  import ProjectFeed from "./ProjectFeed.svelte";
  import PublisherTokens from "./PublisherTokens.svelte";
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

<section class="feed" aria-label="Agent progress">
  {#if phase === "signed-in"}
    <div class="section-heading">
      <h2>Agent progress</h2>
      <button onclick={() => void logout()}>Sign out</button>
    </div>
    <button onclick={() => (managing = !managing)}
      >{managing
        ? "Close credential manager"
        : "Manage publisher credentials"}</button
    >
    {#if managing}<PublisherTokens onexpire={expired} />{/if}
    <ProjectFeed onexpire={expired} />
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
      <h2>Agent progress</h2>
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
