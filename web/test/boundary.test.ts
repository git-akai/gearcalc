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
