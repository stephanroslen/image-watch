<script>
  import { setContext } from "svelte";
  import Router from "../Router.svelte";
  import Route from "../Route.svelte";

  export let initialPath = "/images";
  export let onNavigate = () => {};

  let currentPath = initialPath;

  setContext("currentRoute", () => currentPath);
  setContext("navigate", (path, replace = false) => {
    onNavigate({ path, replace });
    currentPath = path;
  });
</script>

<Router>
  <Route route="/images" default>
    <p>Images page</p>
  </Route>
  <Route route="/login">
    <p>Login page</p>
  </Route>
</Router>
