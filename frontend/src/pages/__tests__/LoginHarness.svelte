<script>
  import { setContext } from "svelte";
  import Login from "../Login.svelte";

  export let setAuthToken = () => {};
  export let verifyFrontendHash = () => {};

  setContext("setAuthToken", setAuthToken);
  setContext("verifyFrontendHash", verifyFrontendHash);
</script>

<Login />
