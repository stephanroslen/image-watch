<script>
  import AuthRequired from "./components/AuthRequired.svelte";
  import Navbar from "./components/Navbar.svelte";
  import Images from "./pages/Images.svelte";
  import Login from "./pages/Login.svelte";
  import Route from "./components/router/Route.svelte";
  import Router from "./components/router/Router.svelte";
  import UnauthRequired from "./components/UnauthRequired.svelte";

  import { setContext } from "svelte";

  let currentRoute = $state(window.location.pathname);

  let authToken = $state(localStorage.getItem("auth_token"));

  setContext("currentRoute", () => currentRoute);

  function navigate(path) {
    history.pushState({}, "", path);
    currentRoute = path;
  }

  function removeAuthToken() {
    authToken = null;
    localStorage.removeItem("auth_token");
  }

  function setAuthToken(token) {
    authToken = token;
    localStorage.setItem("auth_token", token);
    navigate("/images");
  }

  setContext("setAuthToken", setAuthToken);
  setContext("removeAuthToken", removeAuthToken);
  setContext("authToken", () => authToken);
  setContext("navigate", navigate);
</script>

<main>
  <Navbar />
  <Router>
    <Route route="/images" default>
      <AuthRequired fallback="/login">
        <Images />
      </AuthRequired>
    </Route>
    <Route route="/login">
      <UnauthRequired fallback="/images">
        <Login />
      </UnauthRequired>
    </Route>
  </Router>
</main>

<style lang="postcss">
  @reference "tailwindcss";
</style>
