import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  base: "./",
  plugins: [svelte()],
  // wasm-bindgen's glue resolves the binary with `new URL(..., import.meta.url)`,
  // so Vite has to treat it as an asset rather than try to parse it.
  assetsInclude: ["**/*.wasm"],
  server: { fs: { allow: [".."] } },
});
