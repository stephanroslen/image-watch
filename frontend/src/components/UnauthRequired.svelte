<script>
  import { getContext } from "svelte";

  let { children, fallback } = $props();

  let authToken = getContext("authToken");
  let navigate = getContext("navigate");

  let isLoggedIn = $derived(authToken() !== null);

  $effect(() => {
    if (isLoggedIn) {
      navigate(fallback);
    }
  });
</script>

{#if !isLoggedIn}
  {@render children()}
{/if}
