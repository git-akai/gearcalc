// A request that cannot cross into the core — a number that is not finite —
// is refused under the front end's catalogue key on every path, naming the
// box by its label and its piece by the panel's name for it, never by a JSON
// leaf or an array index.
import { expect, test } from "vitest";
import {
  adoptMember,
  defaultTrain,
  defaults,
  dxf,
  editTrain,
  offersAt,
  previewEdit,
  profile,
  guarded,
  relieveCase,
  relieveTrain,
  solve,
  solveTrain,
  t,
  type GearRequest,
  type Note,
} from "../src/core";
import { gearLabel } from "../src/members";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { initSync } from "../src/wasm/gear_wasm.js";

const KEY = "ui.train_boundary_failed";

/** The default pair with one gear's addendum not a number. */
function broken() {
  const train = defaultTrain();
  train.shape.members[1].gear.addendum = NaN;
  const names = solveTrain(defaultTrain()).names;
  const field = `${gearLabel(names, 1)} · ${t("ui.train_addendum")}`;
  return { train, sentence: t("ui.boundary_not_finite", { field }) };
}

function named(n: Note | null, sentence: string) {
  expect(n?.key).toBe(KEY);
  expect(n?.values.detail).toBe(sentence);
  // No wire name and no index leaks into what the reader sees.
  expect(n?.values.detail).not.toMatch(/members|addendum|\[|\d+\.\d+/);
}

test("a solve names the field", () => {
  const { train, sentence } = broken();
  named(solveTrain(train).failure?.note ?? null, sentence);
});

test("an edit, a dry run, the offers and relief each say it, and change nothing", () => {
  const { train, sentence } = broken();
  const before = JSON.stringify(train, (_, x) => (Number.isNaN(x) ? "NaN" : x));
  named(editTrain(train, { add_case: "ultimate" }), sentence);
  named(relieveTrain(train, null), sentence);
  named(relieveCase(train, 0, null), sentence);
  named(offersAt(train, "train").failure, sentence);
  const offer = offersAt(defaultTrain(), "train").offers.find((o) => o.refused === null)!;
  named(previewEdit(train, { graph: offer.edit }).unsolved, sentence);
  expect(JSON.stringify(train, (_, x) => (Number.isNaN(x) ? "NaN" : x))).toBe(before);
});

test("the gear tab's solve, export and adopt say it in the same words", () => {
  const d = defaults().gear;
  const req: GearRequest = { params: { ...d.params, module: NaN }, chord_tolerance: d.chord_tolerance, reference_circles: false };
  const sentence = t(KEY, {
    detail: t("ui.boundary_not_finite", { field: t("ui.gear_field_module") }),
  });
  expect(solve(req)).toEqual({ error: sentence });
  expect(dxf(req)).toEqual({ error: sentence });
  const { train, sentence: member } = broken();
  expect(adoptMember(train, 0)).toEqual({ error: t(KEY, { detail: member }) });
});

// **A value the core's table refuses is said by the field it names**, in the
// catalogue's words — on the gear tab's solve, its drawing and its export,
// and a train's edit — never a raw note or nothing.
test("a value that describes nothing is refused in the catalogue's words", () => {
  const d = defaults().gear;
  const req: GearRequest = { params: { ...d.params, module: 0 }, chord_tolerance: d.chord_tolerance, reference_circles: false };
  const sentence = t(KEY, {
    detail: t("error.input_out_of_range", {
      field: "params.module",
      value: "0",
      bound: "[0.001, 1.7014118346046923e38]",
    }),
  });
  expect(solve(req)).toEqual({ error: sentence });
  expect(dxf(req)).toEqual({ error: sentence });
  expect(profile(req, 600)).toEqual({ error: sentence });
  const train = defaultTrain();
  train.shape.members[0].gear.teeth = 0;
  const edited = editTrain(train, { add_case: "ultimate" });
  expect(edited?.values.detail).toBe(
    t("error.input_out_of_range", { field: "train.shape.members.0.gear.teeth", value: "0", bound: "[1, 4294967295]" }),
  );
});

// **Five thousand traps are said, and the core answers the next call.** A
// trap unwinds nothing, so each leaves the frames it pushed on the module's
// stack; without the stack put back, a few hundred of them leave no stack
// and every call after reads out of bounds. The trap is the module's own —
// an entry handed a string past the end of its memory — since no input a
// caller can send traps it any more: a huge count is refused by name.
test("five thousand traps are said, and the next call is answered", () => {
  const exports = initSync({ module: readFileSync(fileURLToPath(import.meta.resolve("../src/wasm/gear_wasm_bg.wasm"))) });
  const trap = () => {
    const retptr = exports.__wbindgen_add_to_stack_pointer(-16);
    exports.solve_gear(retptr, 0xfffffff0, 64);
  };
  let said = 0;
  for (let i = 0; i < 5000; i++) {
    expect(() => guarded(trap)).toThrow();
    said++;
  }
  expect(said).toBe(5000);
  const d = defaults().gear;
  const good: GearRequest = { params: d.params, chord_tolerance: d.chord_tolerance, reference_circles: false };
  expect("ok" in solve(good)).toBe(true);
  expect("ok" in profile(good, 600)).toBe(true);
});

// **A huge count is refused naming it, never trapped**: the drawing of a
// gear whose every tooth is past the output budget, refused from its size
// before it is drawn. The size and the budget are the core's figures, read
// back from what it said; the words around them and the field are checked.
test("a drawing past its budget is refused by name", () => {
  const d = defaults().gear;
  const huge: GearRequest = { params: { ...d.params, teeth: 300_000_000 }, chord_tolerance: d.chord_tolerance, reference_circles: false };
  const r = profile(huge, 8);
  const said = "error" in r ? r.error : "";
  const [size, budget] = said.match(/(\d+)\D+(\d+)\D*$/)?.slice(1) ?? ["", ""];
  expect(said).toBe(
    t(KEY, { detail: t("error.output_past_budget", { field: "params.teeth", value: "300000000", size, budget }) }),
  );
  expect("ok" in solve(huge)).toBe(true);
});
