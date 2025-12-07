<script>
  import { setContext } from "svelte";
  import UnauthRequired from "../UnauthRequired.svelte";

  export let token = null;
  export let fallback = "/images";
  export let content = "";
  export let navigateSpy = () => {};

  setContext("authToken", () => token);
  setContext("navigate", (path, replace = false) =>
    navigateSpy({ path, replace }),
  );
</script>

<UnauthRequired {fallback}>
  <p>{content}</p>
</UnauthRequired>
