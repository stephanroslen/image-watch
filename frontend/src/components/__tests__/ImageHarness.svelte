<script>
  import { setContext } from "svelte";
  import Image from "../Image.svelte";

  export let token = "token";
  export let removeToken = () => {};
  export let src = "/backend/data/example.jpg";
  export let alt = "Example";
  export let loading;
  export let className = "";

  setContext("authToken", () => token);
  setContext("removeToken", removeToken);
</script>

<Image {src} {alt} class={className} {loading} />
