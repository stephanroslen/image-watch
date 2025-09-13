import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import("@sveltejs/vite-plugin-svelte").SvelteConfig} */
export default {
  adapter: adapter({
    fallback: "index.html",
  }),
  preprocess: vitePreprocess(),
};
