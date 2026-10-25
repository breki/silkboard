<script lang="ts">
  import { onMount, tick } from "svelte";
  import { parsePlanRoute } from "./plan-model";
  import PlanList from "./PlanList.svelte";
  import TaskPlan from "./TaskPlan.svelte";
  import "./plans.css";
  let { onexpire }: { onexpire: () => void } = $props();
  let path = $state(window.location.pathname + window.location.search);
  const route = $derived(parsePlanRoute(path));
  let listScroll = 0;
  function changed() {
    const previous = route;
    path = window.location.pathname + window.location.search;
    const next = parsePlanRoute(path);
    if (
      previous.kind === "task" &&
      next.kind === "task" &&
      previous.project === next.project &&
      previous.task === next.task
    )
      return;
    const destination = path;
    void tick().then(() => {
      if (path === destination)
        window.scrollTo(0, route.kind === "list" ? listScroll : 0);
    });
  }
  function open(event: MouseEvent, target: string) {
    if (
      event.button !== 0 ||
      event.metaKey ||
      event.ctrlKey ||
      event.shiftKey ||
      event.altKey
    )
      return;
    event.preventDefault();
    if (parsePlanRoute(target).kind === "invalid") return;
    if (route.kind === "list") listScroll = window.scrollY;
    window.history.pushState(null, "", target);
    changed();
  }
  function selectSnapshot(target: string) {
    window.history.pushState(null, "", target);
    path = window.location.pathname + window.location.search;
  }
  onMount(() => {
    const previous = window.history.scrollRestoration;
    window.history.scrollRestoration = "manual";
    window.addEventListener("popstate", changed);
    return () => {
      window.removeEventListener("popstate", changed);
      window.history.scrollRestoration = previous;
    };
  });
</script>

<div hidden={route.kind !== "list"}>
  <PlanList active={route.kind === "list"} {onexpire} onopen={open} />
</div>
{#if route.kind === "task"}
  {#key `${route.project}/${route.task}`}
    <TaskPlan
      project={route.project}
      task={route.task}
      sequence={route.sequence}
      {onexpire}
      onopen={open}
      onsequence={selectSnapshot}
    />
  {/key}
{:else if route.kind === "invalid"}
  <section class="panel">
    <h2>Invalid task-plan link</h2>
    <a href="/plans" onclick={(event) => open(event, "/plans")}
      >Return to Task plans</a
    >
  </section>
{/if}
