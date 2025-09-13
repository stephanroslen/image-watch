<script>
  import device from "current-device";
  import { fade } from "svelte/transition";

  import { getContext } from "svelte";

  import Image from "../components/Image.svelte";

  let images = $state([]);
  let connected = $state(false);

  const token = getContext("authToken");
  const removeAuthToken = getContext("removeAuthToken");
  const verifyFrontendHash = getContext("verifyFrontendHash");

  let dummy_images = [];

  let type = device.type;
  let orientation = $state(device.orientation);
  device.onChangeOrientation(
    (newOrientation) => (orientation = newOrientation),
  );

  let grid_columns = $derived(
    type === "mobile"
      ? orientation === "landscape"
        ? 2
        : 1
      : type === "tablet"
        ? orientation === "landscape"
          ? 3
          : 2
        : 3,
  );

  let ws;
  const originalReconnectTimeout = 500;
  let reconnectTimeout = originalReconnectTimeout;
  const maxReconnectTimeout = 3000;
  const keepaliveInterval = 60000;

  let connectScheduled = false;

  async function keepalive() {
    try {
      const response = await fetch("/backend/keepalive", {
        method: "GET",
        headers: {
          Authorization: "Bearer " + token(),
        },
      });

      const status = response.status;

      return status;
    } catch (err) {
      return null;
    }
  }

  function connect() {
    const wsProtocol = location.protocol === "https:" ? "wss:" : "ws:";
    ws = new WebSocket(`${wsProtocol}//${location.host}/backend/ws`, [
      "bearer",
      token(),
    ]);

    ws.addEventListener("open", () => {
      connected = true;
      dummy_images = [];
      reconnectTimeout = originalReconnectTimeout;
      verifyFrontendHash();
      scheduleKeepalive();
    });

    ws.addEventListener("close", () => {
      closeAndErrorHandler();
    });

    ws.addEventListener("error", () => {
      closeAndErrorHandler();
    });

    ws.addEventListener("message", (event) => {
      const data = JSON.parse(event.data);

      if (data.removed) {
        const removedSet = new Set(data.removed);
        dummy_images = dummy_images.filter((img) => !removedSet.has(img.name));
      }

      for (const [name, timestamp] of data.added ?? []) {
        insertSorted({ name, timestamp });
      }

      images = dummy_images;
    });

    function closeAndErrorHandler() {
      connected = false;
      keepalive().then((result) => {
        if (result === 401) {
          removeAuthToken();
        } else {
          scheduleReconnect();
        }
      });
    }

    function scheduleReconnect() {
      if (connectScheduled) return;

      connectScheduled = true;

      setTimeout(() => {
        connectScheduled = false;
        connect();
      }, reconnectTimeout);

      reconnectTimeout = Math.min(reconnectTimeout * 2, maxReconnectTimeout);
    }

    function scheduleKeepalive() {
      if (connected) {
        setTimeout(() => {
          keepalive().then((result) => {
            if (result === 401) {
              removeAuthToken();
            } else {
              scheduleKeepalive();
            }
          });
        }, keepaliveInterval);
      }
    }

    function insertSorted(img) {
      let i = 0;
      while (
        i < dummy_images.length &&
        dummy_images[i].timestamp > img.timestamp
      )
        i++;
      dummy_images = [
        ...dummy_images.slice(0, i),
        img,
        ...dummy_images.slice(i),
      ];
    }
  }

  connect();
</script>

<main>
  {#if !connected}
    <div class="fixed bottom-0 right-0 m-4 z-50">
      <div role="alert" class="alert alert-error">
        <span class="loading loading-spinner loading-xs"></span>
        <span>WebSocket connection lost - reconnecting!</span>
      </div>
    </div>
  {/if}

  <div class="p-4">
    <div
      class="w-[95%] mx-auto grid gap-4 bg-neutral-content text-neutral-content rounded-xl p-4 min-h-[calc(100vh-180px)] place-content-start justify-items-center"
      class:grid-cols-1={grid_columns === 1}
      class:grid-cols-2={grid_columns === 2}
      class:grid-cols-3={grid_columns === 3}
    >
      {#each images as img (img.name)}
        <div
          class="rounded-xl"
          in:fade={{ duration: 300 }}
          out:fade={{ duration: 100 }}
        >
          <div class="tooltip tooltip-info tooltip-bottom" data-tip={img.name}>
            <Image
              src={`/backend/data/${img.name}`}
              alt={img.name}
              class="w-full transition duration-300"
              loading="lazy"
            />
          </div>
        </div>
      {/each}
    </div>
  </div>
</main>
