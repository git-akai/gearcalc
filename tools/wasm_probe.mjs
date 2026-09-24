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
// A preset's starting shape by name, off the list the menu renders from.
const preset = (name) => structuredClone(defaults.presets.find((e) => e.preset === name).shape);
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
  relieve: call("relieve", () =>
    defaults.presets.map((e) => {
      const stage = structuredClone(e.shape);
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
  // **The train's graph edited by the core's rules**, each edit recorded,
  // every index the graph's — a part's body read off the part's own list
  // by slot, and its member by its own index. A set inserted at the
  // default pair's output, joined to it by its sun, its cases carried to
  // the new end, the carrier; the carrier held and released again; the
  // pair's second gear moved off the sun's shaft and its shaft joined to
  // the set's ring instead, then back; and a case of each kind added
  // between the ends, their duties switched.
  edit_train: call("edit_train", () => {
    const edit = (train, e) => JSON.parse(w.edit_train(JSON.stringify({ train, edit: e })));
    const parts = () => JSON.parse(w.solve_train(JSON.stringify({ train: t, library }))).parts;
    const body = (k, slot) => parts()[k].shape.bodies[slot - 1].body;
    const member = (k, j) => parts()[k].members[j];
    const axis = (k, a) => parts()[k].axes[a];
    let t = structuredClone(defaults.train);
    const graph = (e) => (t = edit(t, { graph: e }));
    graph({ insert: { shape: preset("planetary"), at: null } });
    const out = [["insert", structuredClone(t)]];
    graph({ hold: body(1, 2) });
    out.push(["hold", structuredClone(t)]);
    graph({ release: body(1, 2) });
    out.push(["release", structuredClone(t)]);
    graph({ move: { member: member(0, 1), to: null } });
    graph({ join: { a: body(0, 2), b: body(1, 3) } });
    out.push(["move_join", structuredClone(t)]);
    // ...and back onto the sun: moved off the ring's shaft, which it
    // shares, and its own joined to the sun's.
    graph({ move: { member: member(0, 1), to: null } });
    graph({ join: { a: body(0, 2), b: body(1, 1) } });
    out.push(["moved_back", structuredClone(t)]);
    t = edit(t, { add_case: "ultimate" });
    t = edit(t, { add_case: "fatigue" });
    out.push(["add_case", structuredClone(t)]);
    t = edit(t, { duty: { case: 4, intermittent: false } });
    t = edit(t, { duty: { case: 2, intermittent: true } });
    out.push(["duty", structuredClone(t)]);
    // **A set edited piece by piece**: a step on its planets (a second
    // planet gear and a ring on it), its first ring taken off — whose body
    // leaves the train and the rest close up, the case entries at the
    // set's carrier following — and a sun on the new step.
    graph({ add_step: { axis: axis(1, 1) } });
    graph({ remove: { member: member(1, 2) } });
    graph({ add_gear: { mate: member(1, 2), on: { new_body: axis(1, 0) }, ring: false } });
    out.push(["set_edited", structuredClone(t)]);
    // **A chain grown and cut back**: a layshaft laid in at the output, a
    // gear on a new axis at its last gear, and that axis taken away again.
    graph({ insert: { shape: preset("layshaft"), at: null } });
    const lay = parts()[2];
    const last = lay.shape.axes.length - 1;
    const onLast = lay.shape.members
      .map((m, j) => [lay.shape.bodies.find((b) => b.body === m.body).axis, j])
      .filter(([a]) => a === last)
      .map(([, j]) => lay.members[j]);
    graph({ add_gear: { mate: onLast[onLast.length - 1], on: "new_axis", ring: false } });
    out.push(["chain_grown", structuredClone(t)]);
    graph({ remove: { axis: t.shape.axes.length - 1 } });
    out.push(["chain_cut", structuredClone(t)]);
    // **A coupling taken off a planocentric and put back**: its shaft goes
    // with the coupling where nothing else names it, and the planet coupled
    // again drives a new one.
    graph({ insert: { shape: preset("planocentric"), at: null } });
    const plano = parts().length - 1;
    graph({ remove: { coupling: parts()[plano].couplings[0] } });
    out.push(["uncoupled", structuredClone(t)]);
    graph({ couple: { body: t.shape.members[member(plano, 0)].body } });
    out.push(["coupled", structuredClone(t)]);
    // **The graph's other edits**: a gear on a new axis at the train's
    // first gear and taken off again; another ratio on the layshaft
    // sharing its input shaft; a set inserted at the train's first body;
    // and a refusal, which crosses as its catalogue key.
    graph({ add_gear: { mate: 0, on: "new_axis", ring: false } });
    out.push(["graph_add_gear", structuredClone(t)]);
    graph({ remove: { member: t.shape.members.length - 1 } });
    out.push(["graph_removed", structuredClone(t)]);
    const layInput = parts()[2].shape.bodies[0].body;
    const layDistance = parts()[2].distances[0];
    graph({ add_ratio: { distance: layDistance, shared: layInput } });
    out.push(["graph_add_ratio", structuredClone(t)]);
    graph({ insert: { shape: preset("planetary"), at: 1 } });
    out.push(["graph_insert_at", structuredClone(t)]);
    const refusal = (e) => {
      try {
        edit(t, { graph: e });
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
  // The default train, and the same train with a set laid in at its pair's
  // output by the core — a chain of two, its cases at the set's carrier.
  solve_train: call("solve_train", () => {
    const t = structuredClone(defaults.train);
    const chained = JSON.parse(
      w.edit_train(
        JSON.stringify({ train: t, edit: { graph: { insert: { shape: preset("planetary"), at: null } } } }),
      ),
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
