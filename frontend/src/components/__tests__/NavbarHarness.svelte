<script>
  import { setContext } from "svelte";
  import Navbar from "../Navbar.svelte";

  export let token = null;
  export let navigateSpy = () => {};
  export let removeAuthToken = () => {};

  setContext("authToken", () => token);
  setContext("navigate", (path, replace = false) =>
    navigateSpy({ path, replace }),
  );
  setContext("removeAuthToken", removeAuthToken);
</script>

<Navbar />
