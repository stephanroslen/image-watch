<script>
  import { getContext } from "svelte";

  let { children, fallback } = $props();

  const authToken = getContext("authToken");
  const navigate = getContext("navigate");

  let isLoggedIn = $derived(authToken() !== null);

  $effect(() => {
    if (isLoggedIn) {
      navigate(fallback, true);
    }
  });
</script>

{#if !isLoggedIn}
  {@render children()}
{/if}
