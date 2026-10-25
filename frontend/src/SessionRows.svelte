<script lang="ts">
  import EventRows from "./EventRows.svelte";
  import type {
    SessionGroup,
    EventToggle,
    GroupToggle,
  } from "./event-hierarchy";
  let {
    group,
    collapsed,
    expanded,
    ontogglegroup,
    ontoggleevent,
    parentName = "",
    nested = false,
    unmatched = false,
  }: {
    group: SessionGroup;
    collapsed: Set<string>;
    expanded: string | null;
    ontogglegroup: GroupToggle;
    ontoggleevent: EventToggle;
    parentName?: string;
    nested?: boolean;
    unmatched?: boolean;
  } = $props();
  const label = $derived(
    unmatched
      ? group.identity.parent_session_id
        ? "Parent outside current results"
        : "Parent not reported"
      : nested
        ? "Subagent"
        : group.children.length
          ? "Parent session"
          : "Session",
  );
  const metadata = $derived(
    `${group.identity.project} · ${group.identity.tool} · ${group.identity.hostname} · ${group.events.length} ${group.events.length === 1 ? "event" : "events"} on this page`,
  );
</script>

<tr class="session-group" class:nested class:unmatched>
  <th colspan="4" scope="row">
    <button
      class="session-toggle"
      aria-expanded={!collapsed.has(group.key)}
      aria-label={`Session group: ${group.name}${nested ? `, subagent of ${parentName}` : unmatched ? `, ${label}` : ""}`}
      onclick={() => ontogglegroup(group.key)}
    >
      <span aria-hidden="true">{collapsed.has(group.key) ? "▸" : "▾"}</span>
      <span class="session-title">{group.name}</span>
      <span class="session-kind">{label}</span>
      <span class="session-meta">{metadata}</span>
    </button>
  </th>
</tr>
{#if !collapsed.has(group.key)}
  <EventRows
    items={group.events}
    {expanded}
    ontoggle={ontoggleevent}
    {nested}
  />
{/if}

<style>
  .session-group th {
    background: var(--panel);
    padding: 0;
  }
  .session-toggle {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.4rem;
    width: 100%;
    text-align: start;
    white-space: normal;
    overflow-wrap: anywhere;
    min-height: 44px;
    border: 0;
    border-radius: 0;
    padding: 0.65rem;
  }
  .session-title {
    font-weight: 650;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .session-kind {
    font-weight: 400;
    font-size: 0.85em;
  }
  .session-meta {
    flex-basis: 100%;
    font-weight: 400;
    font-size: 0.85em;
    color: var(--muted);
  }
  .nested .session-toggle {
    padding-inline-start: 1.5rem;
    border-inline-start: 2px solid var(--primary);
  }
  .unmatched th {
    border-block-start: 2px solid var(--border);
  }
</style>
