<script lang="ts">
  import { onMount } from "svelte";
  import {
    ApiError,
    publisherCredentials,
    issuePublisher,
    revokePublisher,
    type Page,
    type Publisher,
  } from "./feed-api";
  let { onexpire }: { onexpire: () => void } = $props();
  let projectInput = $state("");
  let label = $state("");
  let project = $state("");
  let page = $state<Page<Publisher> | null>(null);
  let after = $state<string | null>(null);
  let issued = $state<{ token: string; label: string; project: string } | null>(
    null,
  );
  let confirming = $state<string | null>(null);
  let busy = $state(false);
  let notice = $state("");
  let saveNotice = $state("");
  let alive = true;
  let active = new AbortController();
  function begin() {
    active.abort();
    active = new AbortController();
    busy = true;
    notice = "";
    confirming = null;
    return active.signal;
  }
  function forget() {
    issued = null;
    saveNotice = "";
    active.abort();
    busy = false;
    confirming = null;
  }
  function failure(error: unknown, fallback: string) {
    if (error instanceof ApiError && error.status === 401) {
      issued = null;
      onexpire();
    } else notice = fallback;
  }
  async function list(
    selected: string,
    cursor: string | null,
    signal: AbortSignal,
  ) {
    const value = await publisherCredentials(selected, cursor, signal);
    if (alive && !signal.aborted) {
      project = selected;
      page = value;
      after = cursor;
    }
  }
  async function load(cursor: string | null = null, selected = projectInput) {
    if (busy || issued) return;
    if (!/^[a-z0-9-]{1,64}$/.test(selected)) {
      notice =
        "Use a project name with 1–64 lowercase letters, digits or hyphens.";
      return;
    }
    const signal = begin();
    page = null;
    try {
      await list(selected, cursor, signal);
    } catch (error) {
      if (alive && !signal.aborted)
        failure(error, "Unable to load credentials. Try again.");
    } finally {
      if (alive && !signal.aborted) busy = false;
    }
  }
  async function issue(event: SubmitEvent) {
    event.preventDefault();
    if (busy || issued) return;
    if (
      !/^[a-z0-9-]{1,64}$/.test(projectInput) ||
      !/^[!-~]{1,256}$/.test(label)
    ) {
      notice =
        "Enter a valid project and an installation label without spaces (up to 256 ASCII characters).";
      return;
    }
    const selected = projectInput,
      installation = label;
    const signal = begin();
    page = null;
    after = null;
    try {
      const token = await issuePublisher(selected, installation, signal);
      if (!alive || signal.aborted) return;
      issued = { token, project: selected, label: installation };
      saveNotice = "";
      label = "";
      try {
        await list(selected, null, signal);
      } catch (error) {
        if (alive && !signal.aborted)
          failure(
            error,
            "Token issued, but the credential list could not refresh. Save this token now; refresh after saving.",
          );
      }
    } catch (error) {
      if (alive && !signal.aborted)
        failure(
          error,
          error instanceof ApiError && error.status === 409
            ? "This installation label already exists, including revoked labels. Choose a new label."
            : "Issuance could not be confirmed. Refresh the list; if the label exists, revoke it and use a new label. Tokens cannot be retrieved.",
        );
    } finally {
      if (alive && !signal.aborted) busy = false;
    }
  }
  async function revoke(installation: string) {
    if (busy || issued) return;
    const selected = project;
    const signal = begin();
    try {
      await revokePublisher(selected, installation, signal);
      if (!alive || signal.aborted) return;
      notice = "Credential revoked. Existing progress is retained.";
      if (page)
        page = {
          ...page,
          items: page.items.map((item) =>
            item.label === installation ? { ...item, revoked: true } : item,
          ),
        };
      try {
        await list(selected, after, signal);
      } catch (error) {
        if (alive && !signal.aborted)
          failure(
            error,
            "Credential revoked, but the list could not refresh. Load credentials to check the latest metadata.",
          );
      }
    } catch (error) {
      if (alive && !signal.aborted)
        failure(
          error,
          "Revocation could not be confirmed. Refresh the list and check its state before trying again.",
        );
    } finally {
      if (alive && !signal.aborted) busy = false;
    }
  }
  async function copy() {
    if (!issued) return;
    const token = issued.token;
    try {
      await navigator.clipboard.writeText(token);
      if (alive && issued?.token === token)
        saveNotice = "Copied. Install it privately; do not paste it into chat.";
    } catch {
      if (alive && issued?.token === token)
        saveNotice =
          "Clipboard unavailable. Select the token and copy it manually, or download the file.";
    }
  }
  function download() {
    if (!issued) return;
    let url = "";
    try {
      url = URL.createObjectURL(
        new Blob([issued.token + "\n"], { type: "text/plain" }),
      );
      const link = document.createElement("a");
      link.href = url;
      link.download = "silkboard-publisher.token";
      link.click();
      saveNotice =
        "Download requested. Store the file privately and install it with mode 0600 on the publishing machine.";
    } catch {
      saveNotice = "Download unavailable. Copy the token instead.";
    } finally {
      if (url) URL.revokeObjectURL(url);
    }
  }
  onMount(() => {
    window.addEventListener("pagehide", forget);
    return () => {
      alive = false;
      window.removeEventListener("pagehide", forget);
      forget();
    };
  });
</script>

<section class="panel token-manager" aria-label="Publisher credentials">
  <h3>Publisher credentials</h3>
  <p>
    Each installation gets its own project-scoped token for publishing progress.
    Tokens grant no feed, repository, VM or updater access.
  </p>
  <form onsubmit={issue}>
    <label for="token-project">Credential project</label>
    <input
      id="token-project"
      bind:value={projectInput}
      maxlength="64"
      placeholder="silkboard"
      required
      disabled={busy || issued !== null}
    />
    <label for="token-label">Installation label</label>
    <input
      id="token-label"
      bind:value={label}
      maxlength="256"
      placeholder="development-vm-codex"
      required
      disabled={busy || issued !== null}
    />
    <div class="page-controls">
      <button class="primary" type="submit" disabled={busy || issued !== null}
        >Issue token</button
      >
      <button
        type="button"
        onclick={() => void load()}
        disabled={busy || issued !== null}>Load credentials</button
      >
    </div>
  </form>
  {#if notice}<p role="alert">{notice}</p>{/if}
  {#if busy}<p role="status">Updating credentials…</p>{/if}
  {#if issued}
    <div class="secret">
      <h4>Save this token now</h4>
      <p>
        For {issued.project} / {issued.label}. This is the only time you can
        retrieve it. Leaving or hiding this panel clears it.
      </p>
      <label for="new-publisher-token">New publisher token</label>
      <textarea
        id="new-publisher-token"
        readonly
        value={issued.token}
        autocomplete="off"
        spellcheck="false"
        rows="3"></textarea>
      <div class="page-controls">
        <button onclick={() => void copy()}>Copy token</button>
        <button onclick={download}>Download token file</button>
        <button
          onclick={() => {
            issued = null;
            saveNotice = "";
          }}>I’ve saved it</button
        >
      </div>
      {#if saveNotice}<p role="status">{saveNotice}</p>{/if}
      <p>
        Install it privately on the publishing machine. Do not paste it into
        chat. Issuance does not install the credential for you.
      </p>
    </div>
  {/if}
  {#if page}
    <h4>Credentials for {project}</h4>
    {#if page.items.length === 0}<p>No credentials on this page.</p>{/if}
    <ul>
      {#each page.items as item (item.label)}
        <li>
          <strong>{item.label}</strong> — {item.revoked ? "Revoked" : "Active"}
          {#if !item.revoked}
            {#if confirming === item.label}
              <p>Revoke this installation’s publishing access?</p>
              <button
                onclick={() => void revoke(item.label)}
                disabled={busy || issued !== null}>Confirm revocation</button
              >
              <button onclick={() => (confirming = null)}
                >Cancel revocation</button
              >
            {:else}
              <button
                onclick={() => (confirming = item.label)}
                disabled={busy || issued !== null}
                aria-label={`Revoke ${item.label}`}>Revoke</button
              >
            {/if}
          {/if}
        </li>
      {/each}
    </ul>
    <div class="page-controls">
      {#if after !== null}<button
          onclick={() => void load(null, project)}
          disabled={busy || issued !== null}>First credentials</button
        >{/if}
      {#if page.next_cursor}<button
          onclick={() => void load(page!.next_cursor, project)}
          disabled={busy || issued !== null}>More credentials</button
        >{/if}
    </div>
  {/if}
</section>

<style>
  .token-manager {
    margin: 1rem 0;
  }
  .secret {
    margin: 1.5rem 0;
    padding: 1rem;
    border: 1px solid #a9d8c8;
    border-radius: 0.4rem;
  }
  textarea {
    width: 100%;
    font: inherit;
    overflow-wrap: anywhere;
  }
  li {
    margin: 0.75rem 0;
    overflow-wrap: anywhere;
  }
</style>
