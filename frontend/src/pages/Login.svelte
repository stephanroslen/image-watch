<script>
  import { getContext } from "svelte";

  const setAuthToken = getContext("setAuthToken");
  const verifyFrontendHash = getContext("verifyFrontendHash");

  let username = "";
  let password = "";

  verifyFrontendHash();

  async function getToken(username, password) {
    try {
      const response = await fetch("/backend/login", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ username, password }),
      });

      const status = response.status;
      const body = await response.text();

      if (status === 200) {
        return body;
      } else {
        return null;
      }
    } catch (err) {
      return null;
    }
  }

  function handleLogin() {
    getToken(username, password).then((token) => {
      if (token === null) {
        return;
      }
      setAuthToken(token);
    });
  }
</script>

<div class="flex items-center justify-center min-h-[calc(100vh-180px)]">
  <div class="w-full max-w-md flex flex-col">
    <div class="flex flex-col gap-4 m-4">
      <label class="flex items-center gap-3 text-xl">
        <svg
          class="h-[1.5em] w-[1.5em] opacity-50"
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
        >
          <g
            stroke-linejoin="round"
            stroke-linecap="round"
            stroke-width="2.5"
            stroke="currentColor"
          >
            <path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"></path>
            <circle cx="12" cy="7" r="4"></circle>
          </g>
        </svg>
        <input
          bind:value={username}
          type="text"
          required
          placeholder="Username"
          class="p-2 flex-1 border text-xl"
        />
      </label>
    </div>

    <div class="flex flex-col gap-4 m-4">
      <label class="flex items-center gap-3 text-xl">
        <svg
          class="h-[1.5em] w-[1.5em] opacity-50"
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 24 24"
        >
          <g
            stroke-linejoin="round"
            stroke-linecap="round"
            stroke-width="2.5"
            stroke="currentColor"
          >
            <path
              d="M2.586 17.414A2 2 0 0 0 2 18.828V21a1 1 0 0 0 1 1h3a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h1a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h.172a2 2 0 0 0 1.414-.586l.814-.814a6.5 6.5 0 1 0-4-4z"
            ></path>
            <circle cx="16.5" cy="7.5" r=".5" fill="currentColor"></circle>
          </g>
        </svg>
        <input
          bind:value={password}
          type="password"
          required
          placeholder="Password"
          class="p-2 flex-1 border text-xl"
        />
      </label>
    </div>

    <div class="flex justify-end m-4">
      <button class="btn btn-secondary text-lg px-6 py-3" onclick={handleLogin}>
        Login
      </button>
    </div>
  </div>
</div>
