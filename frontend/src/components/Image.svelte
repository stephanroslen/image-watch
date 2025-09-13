<script>
  import { getContext, onMount, onDestroy } from "svelte";

  const {
    src,
    alt = "",
    class: className = "",
    loading: loadingProp,
  } = $props();
  const loading =
    loadingProp === "lazy" || loadingProp === "eager" ? loadingProp : "eager";
  let token = true;

  let imageSrc = $state("");
  let imgElement;
  let observer;

  const authToken = getContext("authToken");
  const removeToken = getContext("removeToken");

  async function loadImage() {
    if (!src || !token) return;
    try {
      let token = authToken();
      const res = await fetch(src, {
        headers: { Authorization: "Bearer " + token },
      });
      const blob = await res.blob();
      imageSrc = URL.createObjectURL(blob);
    } catch (e) {
      console.error("Error loading ", src, ":", e);
      if (e.status === 401) {
        removeToken();
      }
    }
  }

  function handleIntersect(entries) {
    if (entries[0].isIntersecting) {
      loadImage();
      observer.disconnect();
    }
  }

  onMount(() => {
    if (!imgElement) return;
    observer = new IntersectionObserver(handleIntersect, { threshold: 0.1 });
    observer.observe(imgElement);
  });

  onDestroy(() => {
    observer?.disconnect();
    if (imageSrc) URL.revokeObjectURL(imageSrc);
  });
</script>

<img bind:this={imgElement} src={imageSrc} {alt} class={className} {loading} />
