// The front end's tests: components mounted in jsdom against the real core.
//
// Svelte resolves to its server build under Node unless asked for the browser
// one, and `mount` exists only there. The tests live in `test/`, outside
// `tsconfig.json`'s `src/`, so `svelte-check` does not need Node's types.
import { defineConfig, mergeConfig } from "vitest/config";
import vite from "./vite.config.ts";

export default mergeConfig(
  vite,
  defineConfig({
    resolve: { conditions: ["browser"] },
    test: {
      environment: "jsdom",
      include: ["test/**/*.test.ts"],
      setupFiles: ["test/setup.ts"],
    },
  }),
);
