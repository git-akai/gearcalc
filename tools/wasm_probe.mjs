// Every entry point of the WebAssembly boundary, called, with its answer
// printed as JSON.
//
// This exists because of a gap the payload work found: `check_bindings.sh`
// checks the boundary's *shape*, the golden corpus checks `gear-core`'s
// *values* through the CLI, and **nothing ever ran the compiled artifact**.
// A build step that rewrites the module — `wasm-opt`, a toolchain bump, a
// linker flag — could therefore ship a payload that computes differently, or
// not at all, with every check green.
//
// Printed rather than asserted: `tools/check_wasm.sh` decides what to compare
// it against, exactly as `check_golden.sh` decides for the CLI.
//
// **The count is the coverage claim.** `check_wasm.sh` reads `entries` back and
// fails if the crate exports an entry point this file does not call, so a new
// one cannot arrive untested — the same rule `gear-cli`'s `COMMANDS` table
// follows, where the table *is* the dispatch.
import { createRequire } from "module";

const require = createRequire(import.meta.url);
const w = require(process.argv[2]);

// Floats to full precision. `JSON.stringify` prints the shortest round-tripping
// form, which is a *different* string for two doubles that differ in the last
// bit only when the shorter one is unavailable — so a comparison of the default
// rendering can miss a one-ULP move. Seventeen digits cannot.
const exact = (_k, v) =>
  typeof v === "number" && Number.isFinite(v) && !Number.isInteger(v) ? v.toPrecision(17) : v;

const called = [];
const call = (name, f) => {
  called.push(name);
  try {
    return { ok: f() };
  } catch (e) {
    // An entry point that *refuses* is answering; the message is the answer and
    // is compared like any other. A probe that treated a refusal as a crash
    // could not cover the guards at all.
    return { refused: String(e.message ?? e) };
  }
};

const defaults = JSON.parse(w.defaults());
const library = JSON.parse(w.default_materials());

// A plain external gear: the tab's own default, with the eccentric throw
// dropped, since a throw is a question about a mate and this is one gear.
const gear = { ...defaults.gear };
delete gear.eccentric_throw;
const gearJson = JSON.stringify(gear);

// An internal gear, cut by the shaper the same defaults name.
const ring = {
  params: { ...defaults.gear.params, teeth: 60 },
  cutter: defaults.gear.cutter,
  pin_diameter: defaults.gear.pin_diameter,
  chord_tolerance: defaults.gear.chord_tolerance,
};
const ringJson = JSON.stringify(ring);

const trainDoc = { name: "probe", train: defaults.train };

const out = {
  version: call("version", () => w.version()),
  defaults: call("defaults", () => defaults),
  languages: call("languages", () => JSON.parse(w.languages())),
  resolve_language: call("resolve_language", () =>
    ["en", "de-CH", "pt-BR", "zh-Hant-TW", "xx"].map((t) => w.resolve_language(t)),
  ),
  // The catalogue is 391 messages in five languages; the *count and a digest of
  // the keys* is what a payload check wants, not 2,000 lines of prose that
  // `check_strings.py` already owns.
  strings: call("strings", () =>
    JSON.parse(w.languages()).map((l) => {
      const c = JSON.parse(w.strings(l.code ?? l));
      return [l.code ?? l, Object.keys(c).length];
    }),
  ),
  default_materials: call("default_materials", () => library),
  solve_gear: call("solve_gear", () => JSON.parse(w.solve_gear(gearJson))),
  gear_profile: call("gear_profile", () => Array.from(w.gear_profile(gearJson, 24))),
  export_dxf: call("export_dxf", () => w.export_dxf(gearJson)),
  solve_ring: call("solve_ring", () => JSON.parse(w.solve_ring(ringJson))),
  ring_profile: call("ring_profile", () => Array.from(w.ring_profile(ringJson, 24))),
  export_ring_dxf: call("export_ring_dxf", () => w.export_ring_dxf(ringJson)),
  // **Over-determined on purpose, on every kind**, so the answer is the relief
  // rather than a stage that needed none. Every input the kind has is pinned
  // and the first one is declared as the freedom just touched, which is the one
  // that must survive.
  relieve_stage: call("relieve_stage", () =>
    ["spur_stage", "worm_stage", "planetary_stage", "hula_stage"].map((k) => {
      const stage = structuredClone(defaults[k]);
      const pin = (a) => (a ? { auto: false, manual: 0.1 } : a);
      if (stage.centre_distance) stage.centre_distance = pin(stage.centre_distance);
      for (const g of stage.gears ?? []) g.profile_shift = pin(g.profile_shift);
      for (const m of ["sun", "planet", "ring"]) {
        if (stage[m]) stage[m].profile_shift = pin(stage[m].profile_shift);
      }
      const just = stage.centre_distance ? "centre_distance" : { member: [0, "shift"] };
      // ...and a figure for the shift relief turns back given on a hula
      // stage, so the seeding is exercised too.
      const figures = [{ freedom: { member: [1, "shift"] }, value: 0.25 }];
      return [k, JSON.parse(w.relieve_stage(JSON.stringify({ stage, just, figures })))];
    }),
  ),
  // **A set arranged at the head of a train and behind a pair** — a drive
  // on its sun in the first, the chain moved to its sun in the second — so
  // both halves of the gesture are recorded.
  arrange_stage: call("arrange_stage", () => {
    const t = structuredClone(defaults.train);
    const set = structuredClone(defaults.planetary_stage);
    const head = { ...t, stages: [set, t.stages[0]] };
    const behind = { ...t, stages: [t.stages[0], set] };
    return [
      ["head", JSON.parse(w.arrange_stage(JSON.stringify({ train: head, stage: 0, driven: 1, held: 2 })))],
      ["behind", JSON.parse(w.arrange_stage(JSON.stringify({ train: behind, stage: 1, driven: 1, held: 2 })))],
    ];
  }),
  // **One member of each kind adopted**, including a planetary ring so the
  // cutter travels, and a worm stage's wheel — its worm is refused, which
  // the entry point's own test holds.
  adopt_member: call("adopt_member", () =>
    [
      ["spur_stage", 1],
      ["worm_stage", 1],
      ["planetary_stage", 2],
      ["hula_stage", 0],
    ].map(([k, member]) => {
      const train = { ...structuredClone(defaults.train), stages: [defaults[k]] };
      return [k, JSON.parse(w.adopt_member(JSON.stringify({ train, materials: library, stage: 0, member })))];
    }),
  ),
  solve_train: call("solve_train", () =>
    JSON.parse(w.solve_train(JSON.stringify({ train: defaults.train, library }))),
  ),
  export_train: call("export_train", () => w.export_train(JSON.stringify(trainDoc))),
  import_train: call("import_train", () =>
    JSON.parse(w.import_train(w.export_train(JSON.stringify(trainDoc)))),
  ),
  export_materials: call("export_materials", () => w.export_materials(JSON.stringify(library))),
  import_materials: call("import_materials", () =>
    JSON.parse(w.import_materials(w.export_materials(JSON.stringify(library)))),
  ),
};

console.log(JSON.stringify({ entries: called.sort(), out }, exact, 1));
