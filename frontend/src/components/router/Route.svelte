<script>
  import { getContext } from "svelte";

  const { route, children, default: isDefaultRoute = false } = $props();

  const selectedRoute = getContext("selectedRoute");
  getContext("registerRoute")(route);
  if (isDefaultRoute) {
    getContext("setDefaultRoute")(route);
  }

  let isActive = $derived(route === selectedRoute());
</script>

{#if isActive}
  {@render children()}
{/if}
