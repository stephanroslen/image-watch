<script>
  import { setContext } from "svelte";
  import Route from "../Route.svelte";
  import Router from "../Router.svelte";

  let currentPath = "/images";

  setContext("currentRoute", () => currentPath);
  setContext("navigate", (path) => {
    currentPath = path;
  });
</script>

<Router>
  <Route route="/images" default>
    <p>Images page</p>
  </Route>
  <Route route="/login" default>
    <p>Login page</p>
  </Route>
</Router>
