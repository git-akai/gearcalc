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
// A preset's starting stage by name, off the list the menu renders from.
const preset = (name) => structuredClone(defaults.stages.find((e) => e.preset === name).stage);
const library = JSON.parse(w.default_materials());
// **A train's cards**: the parts of its one graph, each in its own
// numbering, as every solve deals them — what the panel draws a card from.
const cards = (train) =>
  JSON.parse(w.solve_train(JSON.stringify({ train, library }))).topology.map((s) => s.part.shape);

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
  relieve: call("relieve", () =>
    defaults.stages.map((e) => {
      const stage = structuredClone(e.stage);
      const pin = (a) => (a ? { auto: false, manual: 0.1 } : a);
      // A shape keeps its distance on `distances[0]` and its gears under
      // `members[].gear`.
      for (const d of stage.distances) d.distance = pin(d.distance);
      for (const m of stage.members) m.gear.profile_shift = pin(m.gear.profile_shift);
      const just = { distance: 0 };
      // ...and a figure for the shift relief turns back given, so the
      // seeding is exercised too.
      const figures = [{ freedom: { member: [1, "shift"] }, value: 0.25 }];
      return [e.preset, JSON.parse(w.relieve(JSON.stringify({ shape: stage, just, figures })))];
    }),
  ),
  // **One member of each preset adopted**, including a planetary ring so
  // the cutter travels, and a worm stage's wheel — its worm is refused,
  // which the entry point's own test holds.
  adopt_member: call("adopt_member", () =>
    [
      ["spur", 1],
      ["worm", 1],
      ["planetary", 2],
      ["wolfrom", 0],
    ].map(([k, member]) => {
      const train = { ...structuredClone(defaults.train), shape: preset(k) };
      return [k, JSON.parse(w.adopt_member(JSON.stringify({ train, materials: library, member })))];
    }),
  ),
  // **The train's graph edited by the core's rules**, each edit recorded:
  // a set pushed behind the default pair and joined onward by its sun —
  // one body, the pair's second gear and the sun — its cases carried to
  // the new end, the carrier; the carrier held and released again; the
  // pair's end of the sun's body split off and joined to the set's ring
  // instead, then moved back onto the sun by the select's one rule; and a
  // case of each kind added between the ends. A body is its number, read
  // off the card's list by slot: the set's sun, carrier and ring are
  // slots 1, 2 and 3.
  edit_train: call("edit_train", () => {
    const edit = (train, e) => JSON.parse(w.edit_train(JSON.stringify({ train, edit: e })));
    const body = (train, stage, slot) => cards(train)[stage].bodies[slot - 1].body;
    let t = edit(structuredClone(defaults.train), { push_stage: preset("planetary") });
    const out = [["push_stage", structuredClone(t)]];
    t = edit(t, { hold: body(t, 1, 2) });
    out.push(["hold", structuredClone(t)]);
    t = edit(t, { release: body(t, 1, 2) });
    out.push(["release", structuredClone(t)]);
    t = edit(t, { split: { stage: 0, body: body(t, 1, 1) } });
    t = edit(t, { join: { a: body(t, 0, 2), b: body(t, 1, 3) } });
    out.push(["split_join", structuredClone(t)]);
    t = edit(t, { move_end: { stage: 0, body: body(t, 0, 2), to: body(t, 1, 1) } });
    out.push(["move_end", structuredClone(t)]);
    t = edit(t, { add_case: "ultimate" });
    t = edit(t, { add_case: "fatigue" });
    out.push(["add_case", structuredClone(t)]);
    t = edit(t, { duty: { case: 4, intermittent: false } });
    t = edit(t, { duty: { case: 2, intermittent: true } });
    out.push(["duty", structuredClone(t)]);
    // **A stage edited on its card**: the set at stage 1 gains a step (a
    // second planet gear and a ring on it), loses its first ring — whose
    // body leaves the train and the rest close up, the case entries at the
    // set's carrier following — gains a sun on the new step, and has its
    // sun moved to a body of its own; then a layshaft pushed behind gains
    // a pair and an idler axis and loses them again.
    const stage = (k, e) => edit(t, { stage: { stage: k, edit: e } });
    t = stage(1, { add_step: { axis: 1 } });
    t = stage(1, { remove_member: { member: 2 } });
    t = stage(1, { add_central: { gear: 2, ring: false } });
    t = stage(1, { move_body: { member: 0, body: null } });
    out.push(["stage_epicyclic", structuredClone(t)]);
    t = edit(t, { push_stage: preset("layshaft") });
    t = stage(2, { add_mesh: { distance: 0 } });
    // At the chain's end: the last gear on the card's last axis.
    const lay = cards(t)[2];
    const last = lay.axes.length - 1;
    const onLast = lay.members
      .map((m, j) => [lay.bodies.find((b) => b.body === m.body).axis, j])
      .filter(([a]) => a === last)
      .map(([, j]) => j);
    t = stage(2, { add_axis: { mate: onLast[onLast.length - 1] } });
    out.push(["stage_parallel_added", structuredClone(t)]);
    t = stage(2, { remove_axis: { axis: cards(t)[2].axes.length - 1 } });
    t = stage(2, { remove_mesh: { mesh: cards(t)[2].meshes.length - 1 } });
    out.push(["stage_parallel_removed", structuredClone(t)]);
    // **A coupling taken off a planocentric and put back**: its shaft goes
    // with the coupling where nothing else names it, and the planet coupled
    // again drives a new one.
    t = edit(t, { push_stage: preset("planocentric") });
    const plano = cards(t).length - 1;
    t = stage(plano, { uncouple: { coupling: 0 } });
    out.push(["stage_uncoupled", structuredClone(t)]);
    t = stage(plano, { couple: { body: cards(t)[plano].members[0].body } });
    out.push(["stage_coupled", structuredClone(t)]);
    // **The graph's own edits**, every index the graph's: a gear on a new
    // axis at the train's first gear and taken off again; another ratio on
    // the layshaft sharing its input shaft; a gear moved to a body of its
    // own and joined back; the planocentric's coupling removed and its
    // planet coupled again; a set inserted at the train's first body; and
    // a refusal, which crosses as its catalogue key.
    const graph = (e) => edit(t, { graph: e });
    t = graph({ add_gear: { mate: 0, on: "new_axis", ring: false } });
    out.push(["graph_add_gear", structuredClone(t)]);
    t = graph({ remove: { member: t.shape.members.length - 1 } });
    out.push(["graph_removed", structuredClone(t)]);
    const layParts = cards(t)[2];
    const layInput = layParts.bodies[0].body;
    const layDistance = JSON.parse(w.solve_train(JSON.stringify({ train: t, library }))).topology[2].part.distances[0];
    t = graph({ add_ratio: { distance: layDistance, shared: layInput } });
    out.push(["graph_add_ratio", structuredClone(t)]);
    t = graph({ insert: { stage: preset("planetary"), at: 1 } });
    out.push(["graph_insert_at", structuredClone(t)]);
    const refusal = (e) => {
      try {
        graph(e);
        return null;
      } catch (x) {
        return String(x.message ?? x);
      }
    };
    out.push(["graph_refused", [
      refusal({ join: { a: 1, b: 2 } }),
      refusal({ add_gear: { mate: 0, on: { new_body: 0 }, ring: false } }),
      refusal({ remove: { axis: 9999 } }),
    ]]);
    return out;
  }),
  // **What an edit would do**, previewed on the default train: a gear on a
  // new axis at its second gear (counted, the path kept), its first gear
  // removed (the pair with it, the path lost), a release of nothing held
  // (nothing), and a join of its two bodies (refused, by its key).
  preview_edit: call("preview_edit", () => {
    const train = structuredClone(defaults.train);
    const preview = (edit) =>
      JSON.parse(w.preview_edit(JSON.stringify({ train, materials: library, edit })));
    return [
      preview({ graph: { add_gear: { mate: 1, on: "new_axis", ring: false } } }),
      preview({ graph: { remove: { member: 0 } } }),
      preview({ graph: { release: 1 } }),
      preview({ graph: { join: { a: 1, b: 2 } } }),
    ];
  }),
  // **What can be done to each piece** of the default train — the train, its
  // first gear, its mesh, its first body, its first axis and its distance:
  // each offer's edit and its refusal's key. A stage an insert lays in is
  // named by its preset rather than printed whole, the presets being
  // `defaults`' to record.
  offers: call("offers", () => {
    const train = structuredClone(defaults.train);
    const at = [
      "train",
      { member: 0 },
      { mesh: 0 },
      { body: 1 },
      { axis: 0 },
      { distance: 0 },
    ];
    return at.map((target) => [
      target,
      JSON.parse(w.offers(JSON.stringify({ train, at: target }))).map((o) => ({
        edit: o.preset === null ? o.edit : { insert: { preset: o.preset, at: o.edit.insert.at } },
        refused: o.refused?.key ?? null,
      })),
    ]);
  }),
  // The default train, and the same train with a set pushed behind its pair
  // by the core — a chain of two, its cases at the set's carrier.
  solve_train: call("solve_train", () => {
    const t = structuredClone(defaults.train);
    const chained = JSON.parse(
      w.edit_train(JSON.stringify({ train: t, edit: { push_stage: preset("planetary") } })),
    );
    return [
      ["default", JSON.parse(w.solve_train(JSON.stringify({ train: t, library })))],
      ["chained", JSON.parse(w.solve_train(JSON.stringify({ train: chained, library })))],
    ];
  }),
  // **Over-determined on purpose**: the default train's first case with its
  // reaction at the second gear made a load with both figures given — two
  // speeds on one degree of freedom — and that speed declared the one just
  // touched, so the answer is the relief (the first gear's speed derived,
  // seeded from what the case comes to) rather than a case that needed none.
  relieve_case: call("relieve_case", () => {
    const train = structuredClone(defaults.train);
    const given = (v) => ({ auto: false, manual: v });
    train.load_cases[0].loads[1] = { at: 2, role: "load", torque: given(1), speed: given(100) };
    const just = { load: 1, which: "speed" };
    return JSON.parse(w.relieve_case(JSON.stringify({ train, library, case: 0, just })));
  }),
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
