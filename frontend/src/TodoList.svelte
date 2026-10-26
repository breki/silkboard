<script lang="ts">
  import { SvelteMap } from "svelte/reactivity";
  import type { Todo, TodoAction, TodoCheck } from "./todos-api";
  let {
    items,
    disabled,
    oncheck,
    onedit,
    onchild,
    ondelete,
  }: {
    items: Todo[];
    disabled: boolean;
    oncheck: TodoCheck;
    onedit: TodoAction;
    onchild: TodoAction;
    ondelete: TodoAction;
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
      <li>
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
