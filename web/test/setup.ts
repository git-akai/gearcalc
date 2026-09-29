// What every test starts from: the wasm core loaded synchronously from disk,
// the catalogue and defaults read, and one gear tab and one train tab open —
// what `App.svelte` does at start-up, without a fetch.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { initSync } from "../src/wasm/gear_wasm.js";
import { loadCore } from "../src/core";
import { library, trains, workspace } from "../src/state.svelte";

// jsdom has no layout and no canvas. The viewport only draws, so both are
// stubbed to do nothing.
globalThis.ResizeObserver ??= class {
  observe() {}
  unobserve() {}
  disconnect() {}
};
const noop = new Proxy({}, { get: () => () => {} });
HTMLCanvasElement.prototype.getContext = (() => noop) as never;

initSync({ module: readFileSync(fileURLToPath(import.meta.resolve("../src/wasm/gear_wasm_bg.wasm"))) });
await loadCore();
library.loadDefaults();
workspace.initialise();
trains.initialise();
