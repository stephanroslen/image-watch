<script>
  import { setContext } from "svelte";
  import Images from "../Images.svelte";

  export let token = "token";
  export let removeAuthToken = () => {};
  export let verifyFrontendHash = () => {};

  setContext("authToken", () => token);
  setContext("removeAuthToken", removeAuthToken);
  setContext("verifyFrontendHash", verifyFrontendHash);
</script>

<Images />
