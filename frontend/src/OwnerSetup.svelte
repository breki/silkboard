<script lang="ts">
  import { onMount } from "svelte";
  import { ApiError, setupState, registerOwner } from "./feed-api";
  import { takeSetupLink } from "./setup-link";
  let { takeCredential }: { takeCredential: () => string } = $props();
  let credential = "";
  let phase = $state("checking");
  let password = $state("");
  let confirmation = $state("");
  let notice = $state("");
  let alive = true;
  let active = new AbortController();
  function forget() {
    credential = "";
    password = "";
    confirmation = "";
    active.abort();
    phase = "missing";
  }
  async function check() {
    const signal = active.signal;
    phase = "checking";
    try {
      const state = await setupState(signal);
      if (alive && !signal.aborted)
        phase =
          state === "available" ? (credential ? "ready" : "missing") : state;
    } catch {
      if (alive && !signal.aborted) phase = "unavailable";
    }
  }
  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (phase !== "ready") return;
    if (password !== confirmation) {
      notice = "Passwords do not match. Try again.";
      return;
    }
    const value = password;
    const signal = active.signal;
    password = "";
    confirmation = "";
    notice = "";
    phase = "submitting";
    try {
      await registerOwner(credential, value, signal);
      if (alive && !signal.aborted) {
        credential = "";
        phase = "done";
      }
    } catch (error) {
      if (!alive || signal.aborted) return;
      const status = error instanceof ApiError ? error.status : 0;
      if (status === 401 || status === 410) {
        credential = "";
        phase = status === 410 ? "closed" : "waiting";
      } else {
        phase = "ready";
        notice =
          status === 429
            ? "Too many attempts. Wait a minute and try again."
            : status === 400
              ? "Use a passphrase of 12–1,024 bytes without control characters."
              : "Registration could not be confirmed. Try again; if already completed, sign in.";
      }
    }
  }
  function openLink() {
    forget();
    active = new AbortController();
    notice = "";
    credential = takeSetupLink(window.location, window.history);
    void check();
  }
  onMount(() => {
    credential = takeCredential();
    void check();
    window.addEventListener("pagehide", forget);
    window.addEventListener("hashchange", openLink);
    return () => {
      alive = false;
      window.removeEventListener("pagehide", forget);
      window.removeEventListener("hashchange", openLink);
      forget();
    };
  });
</script>

<section class="panel login" aria-label="Owner setup">
  <h2>Set up your Silkboard</h2>
  {#if phase === "ready"}
    <p>Create the single owner account. Choose a long, unique passphrase.</p>
    {#if notice}<p role="alert">{notice}</p>{/if}
    <form onsubmit={submit}>
      <label for="setup-password">New owner password</label>
      <input
        id="setup-password"
        type="password"
        autocomplete="new-password"
        minlength="12"
        maxlength="1024"
        required
        bind:value={password}
      />
      <label for="setup-confirmation">Confirm password</label>
      <input
        id="setup-confirmation"
        type="password"
        autocomplete="new-password"
        minlength="12"
        maxlength="1024"
        required
        bind:value={confirmation}
      />
      <button class="primary" type="submit">Create owner account</button>
    </form>
  {:else if phase === "done" || phase === "closed"}
    <p role="status">
      {phase === "done"
        ? "Owner registered. Your setup link has been used."
        : "Owner setup is already complete."}
    </p>
    <a href="/">Sign in to Silkboard</a>
  {:else if phase === "missing" || phase === "waiting"}
    <p role="alert">
      {phase === "missing"
        ? "Open the private setup link prepared for you. Reloading this page clears the link."
        : "This setup link is unavailable, expired or no longer valid. Ask the provisioner for a new private link."}
    </p>
    <a href="/">Already registered? Sign in</a>
  {:else if phase === "unavailable"}
    <p role="alert">Unable to check setup. Try again.</p>
    <button onclick={() => void check()}>Retry setup</button>
  {:else}
    <p role="status">
      {phase === "submitting" ? "Creating your account…" : "Checking setup…"}
    </p>
  {/if}
</section>
