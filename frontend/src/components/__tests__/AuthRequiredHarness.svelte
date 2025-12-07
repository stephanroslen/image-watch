<script>
  import { setContext } from "svelte";
  import AuthRequired from "../AuthRequired.svelte";

  export let token = null;
  export let fallback = "/login";
  export let content = "";
  export let navigateSpy = () => {};

  setContext("authToken", () => token);
  setContext("navigate", (path, replace = false) =>
    navigateSpy({ path, replace }),
  );
</script>

<AuthRequired {fallback}>
  <p>{content}</p>
</AuthRequired>
