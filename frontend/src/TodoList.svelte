<script lang="ts">
  import { featureLink, updateQuery } from "./link-state";
  import { SvelteMap } from "svelte/reactivity";
  import type { Todo, TodoAction, TodoCheck } from "./todos-api";
  let {
    items,
    disabled,
    oncheck,
    onedit,
    onchild,
    ondelete,
    selected = null,
  }: {
    items: Todo[];
    disabled: boolean;
    oncheck: TodoCheck;
    onedit: TodoAction;
    onchild: TodoAction;
    ondelete: TodoAction;
    selected?: string | null;
  } = $props();
  const children = $derived.by(() => {
    const groups = new SvelteMap<string | null, Todo[]>();
    for (const item of items) {
      const group = groups.get(item.parent_id) ?? [];
      group.push(item);
      groups.set(item.parent_id, group);
    }
    return groups;
  });
</script>

{#snippet branch(parent: string | null, depth: number)}
  <ul>
    {#each children.get(parent) ?? [] as item (item.id)}
      <li id={`todo-${item.id}`} class:linked={selected === item.id}>
        <div class="todo-row">
          <label class:completed={item.completed}
            ><input
              type="checkbox"
              checked={item.completed}
              {disabled}
              onchange={(event) => {
                event.currentTarget.checked = item.completed;
                oncheck(item, event.currentTarget);
              }}
            /><span>{item.text}</span></label
          >
          <div class="todo-actions">
            <a
              class="todo-link"
              href={featureLink(`/todos?todo=${item.id}`)}
              aria-label={`Link to ${item.text}`}
              onclick={(event) => {
                event.preventDefault();
                updateQuery({ todo: item.id, mode: null });
              }}>Link</a
            >
            <button
              class="quiet"
              {disabled}
              aria-label={`Edit ${item.text}`}
              onclick={() => onedit(item)}>Edit</button
            >
            {#if depth < 5}<button
                class="quiet"
                {disabled}
                aria-label={`Add child to ${item.text}`}
                onclick={() => onchild(item)}>Add child</button
              >{/if}
            <button
              class="quiet danger"
              {disabled}
              aria-label={`Delete ${item.text}`}
              onclick={() => ondelete(item)}>Delete</button
            >
          </div>
        </div>
        {#if children.has(item.id)}{@render branch(item.id, depth + 1)}{/if}
      </li>
    {/each}
  </ul>
{/snippet}
{@render branch(null, 1)}

<style>
  .linked > .todo-row {
    border-inline-start: 3px solid var(--primary);
    padding-inline-start: 0.5rem;
  }
  .todo-link {
    display: inline-flex;
    align-items: center;
    min-height: 2.75rem;
    padding: 0.25rem 0.5rem;
  }
  ul {
    list-style: none;
    padding-inline-start: 0.75rem;
    margin-block: 0.4rem;
  }
  ul:first-child {
    padding-inline-start: 0;
  }
  li {
    min-width: 0;
  }
  .todo-row {
    padding-block: 0.5rem;
    border-bottom: 1px solid var(--line);
  }
  label {
    display: flex;
    align-items: flex-start;
    gap: 0.5rem;
    min-height: 2.75rem;
    overflow-wrap: anywhere;
  }
  input {
    flex-shrink: 0;
    width: 1.5rem;
    min-width: 1.5rem;
    min-height: 1.5rem;
    margin: 0.3rem 0;
  }
  label span {
    min-width: 0;
  }
  .completed span {
    text-decoration: line-through;
  }
  .todo-actions {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
  }
</style>
