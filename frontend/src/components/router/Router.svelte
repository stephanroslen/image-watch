<script>
  import { getContext, setContext } from "svelte";

  let knownRoutes = $state([]);
  let defaultRoute = $state(null);

  function registerRoute(route) {
    knownRoutes.push(route);
  }

  function setDefaultRoute(route) {
    if (defaultRoute) {
      throw new Error("Default route already set");
    } else {
      defaultRoute = route;
    }
  }

  setContext("registerRoute", registerRoute);
  setContext("setDefaultRoute", setDefaultRoute);

  const currentRoute = getContext("currentRoute");
  const navigate = getContext("navigate");

  let selectedRoute = $derived(
    knownRoutes.find((route) => route === currentRoute()) || defaultRoute,
  );

  $effect(() => {
    if (selectedRoute !== currentRoute()) {
      navigate(selectedRoute, true);
    }
  });

  setContext("selectedRoute", () => selectedRoute);

  const { children } = $props();
</script>

{@render children()}
