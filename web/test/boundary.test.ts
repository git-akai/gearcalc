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
  relieveCase,
  relieveTrain,
  solve,
  solveTrain,
  t,
  type GearRequest,
  type Note,
} from "../src/core";
import { gearLabel } from "../src/members";

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
    detail: t("error.input_out_of_range", { field: "params.module", value: "0", bound: "(0, ∞)" }),
  });
  expect(solve(req)).toEqual({ error: sentence });
  expect(dxf(req)).toEqual({ error: sentence });
  expect(profile(req, 600)).toEqual({ error: sentence });
  const train = defaultTrain();
  train.shape.members[0].gear.teeth = 0;
  const edited = editTrain(train, { add_case: "ultimate" });
  expect(edited?.values.detail).toBe(
    t("error.input_out_of_range", { field: "train.shape.members.0.gear.teeth", value: "0", bound: "[1, ∞)" }),
  );
});

// **A trap is said with the words its panic had, and the core answers the
// next call.** A gear of four billion teeth is a count the table admits and
// a drawing no 32-bit memory holds: the seat list's capacity overflows, a
// panic the browser sees only as "unreachable".
test("a trap is said, not swallowed, and the next call is answered", () => {
  const d = defaults().gear;
  const huge: GearRequest = { params: { ...d.params, teeth: 4_000_000_000 }, chord_tolerance: d.chord_tolerance, reference_circles: false };
  const r = profile(huge, 8);
  expect("error" in r && r.error).toContain("capacity overflow");
  const good: GearRequest = { params: d.params, chord_tolerance: d.chord_tolerance, reference_circles: false };
  expect("ok" in solve(good)).toBe(true);
  expect("ok" in profile(good, 600)).toBe(true);
});
