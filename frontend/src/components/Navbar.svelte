<script>
  import { getContext } from "svelte";

  const authToken = getContext("authToken");
  const navigate = getContext("navigate");
  const removeAuthToken = getContext("removeAuthToken");

  function handleLogout() {
    (async () => {
      try {
        const response = await fetch("/backend/logout", {
          method: "POST",
          headers: {
            "Content-Type": "text/plain",
            Authorization: "Bearer " + authToken(),
          },
        });

        const status = response.status;
        const body = await response.text();
      } catch (err) {}
    })().then(removeAuthToken());
  }

  let isLoggedIn = $derived(authToken() !== null);
</script>

<div class="navbar bg-primary text-primary-content">
  <div class="flex-1 pl-4">
    <h1 class="text-2xl font-bold accent-primary">Image Watch</h1>
  </div>
  <div class="flex-none pr-4">
    <ul class="menu menu-horizontal px-1">
      {#if isLoggedIn}
        <li>
          <button
            class="btn btn-ghost text-xl"
            onclick={() => navigate("/images")}
            >Images
          </button>
        </li>
        <li>
          <button class="btn btn-ghost text-xl" onclick={() => handleLogout()}
            >Logout
          </button>
        </li>
      {:else}
        <li>
          <button
            class="btn btn-ghost text-xl"
            onclick={() => navigate("/login")}
            >Login
          </button>
        </li>
      {/if}
    </ul>
  </div>
</div>
