<script>
  import device from "current-device";
  import { fade } from "svelte/transition";

  let images = $state([]);
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

  const wsProtocol = location.protocol === "https:" ? "wss:" : "ws:";
  const ws = new WebSocket(`${wsProtocol}//${location.host}/ws`);

  let reloader = (event) => {
    setTimeout(() => {
      location.reload();
    }, 10000);
  };

  ws.onclose = reloader;
  ws.onerror = reloader;

  function insertSorted(img) {
    let i = 0;
    while (i < dummy_images.length && dummy_images[i].timestamp > img.timestamp)
      i++;
    dummy_images = [...dummy_images.slice(0, i), img, ...dummy_images.slice(i)];
  }

  ws.addEventListener("message", (event) => {
    const start = Date.now();

    const data = JSON.parse(event.data);

    if (data.removed) {
      const removedSet = new Set(data.removed);
      dummy_images = dummy_images.filter((img) => !removedSet.has(img.name));
    }

    for (const [name, timestamp] of data.added ?? []) {
      insertSorted({ name, timestamp });
    }

    images = dummy_images;

    console.log(`Time elapsed: ${Date.now() - start} ms`);
  });
</script>

<main>
  <h1 class="text-center">Image Watch</h1>

  <div
    class="w-[90%] mx-auto grid gap-4"
    class:grid-cols-1={grid_columns === 1}
    class:grid-cols-2={grid_columns === 2}
    class:grid-cols-3={grid_columns === 3}
  >
    {#each images as img (img.name)}
      <div
        class="relative group rounded-xl overflow-hidden"
        in:fade={{ duration: 300 }}
        out:fade={{ duration: 100 }}
      >
        <img
          src={`/data/${img.name}`}
          alt={img.name}
          class="w-full h-full object-cover transition duration-300 group-hover:brightness-75"
        />
        <div
          class="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition"
        >
          <span
            class="block text-white text-lg font-bold bg-black/20 px-2 rounded"
            >{img.name}</span
          >
        </div>
      </div>
    {/each}
  </div>
</main>

<style lang="postcss">
  @reference "tailwindcss";
</style>
