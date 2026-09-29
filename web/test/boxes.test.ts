// T19.2: one number box, everywhere.
//
// Every box in both panels is a `NumberBox`, which commits only a finite
// number that passes the whole-number check and the core's bound. What a
// reader half-types never reaches the train, so the train never fails at the
// boundary for it and the panel keeps its lists, ports and names.
import { readdirSync, readFileSync } from "node:fs";
import { flushSync } from "svelte";
import { afterEach, expect, test } from "vitest";
import GearPanel from "../src/GearPanel.svelte";
import TrainPanel from "../src/TrainPanel.svelte";
import { defaultTrain, offersAt, editTrain, solveTrain, t, type Train } from "../src/core";
import { freshView, setKind, trains, workspace, type Selection, type TrainTab } from "../src/state.svelte";
import { box, leave, mounted, row, type } from "./dom";

let panel: ReturnType<typeof mounted> | null = null;
afterEach(() => {
  panel?.done();
  panel = null;
});

// An attribute, not a CSS selector: `[type="number"]` styles the box, and
// the gate is about who draws one.
test('`type="number"` is written in NumberBox.svelte alone', () => {
  const dir = `${process.cwd()}/src/`;
  const hits = readdirSync(dir, { recursive: true, encoding: "utf8" })
    .filter((f) => /\.(svelte|ts)$/.test(f) && !f.startsWith("wire") && !f.startsWith("wasm"))
    .filter((f) => /(^|\s)type="number"/m.test(readFileSync(dir + f, "utf8")));
  expect(hits).toEqual(["NumberBox.svelte"]);
});

/** The default pair with each preset chained on at its output, as the add
 *  menu lays one in — every arrangement's pieces, one train each. */
function trainsToTry(): Train[] {
  const base = defaultTrain();
  return offersAt(base, "train")
    .filter((o) => o.refused === null)
    .map((o) => {
      const train = structuredClone(base);
      expect(editTrain(train, o.edit)).toBeNull();
      return train;
    });
}

/** Every `Auto` in `v` turned to the designer's, so its box is typed into. */
function allGiven(v: unknown) {
  if (typeof v !== "object" || v === null) return;
  const o = v as Record<string, unknown>;
  if (typeof o.auto === "boolean" && "manual" in o) o.auto = false;
  for (const x of Object.values(o)) allGiven(x);
}

/** Everything the workspace can show for `train`. */
function selections(train: Train): Selection[] {
  const s = train.shape;
  return [
    ...s.meshes.map((_, mesh) => ({ mesh })),
    ...s.axes.map((_, axis) => ({ axis })),
    ...train.load_cases.map((_, c) => ({ case: c })),
  ];
}

const snapshot = (train: Train) => JSON.stringify(train);

/** A train tab holding `train`, reactive as the application's are. */
function open(train: Train): TrainTab {
  trains.tabs.push({ id: 1000 + trains.tabs.length, name: "t", train, view: freshView() });
  return trains.tabs[trains.tabs.length - 1];
}

test("what a reader half-types never reaches the train", () => {
  const texts = ["", "-", "1,5"];
  let tried = 0;
  for (const given of [false, true]) {
    for (const train of trainsToTry()) {
      if (given) allGiven(train);
      const tab = open(train);
      panel = mounted(TrainPanel, { tab });
      for (const sel of selections(tab.train)) {
        tab.view.selection = sel;
        flushSync();
        const boxes = [
          ...panel.target.querySelectorAll<HTMLInputElement>('input[type="number"]:not(:disabled)'),
        ];
        for (const input of boxes) {
          const before = snapshot(tab.train);
          for (const text of input.inputMode === "numeric" ? [...texts, "17.5", "-3"] : texts) {
            type(input, text);
            expect(snapshot(tab.train), `${JSON.stringify(sel)} "${text}"`).toBe(before);
            const out = solveTrain(tab.train);
            expect(out.failure?.note.key).not.toBe("ui.train_boundary_failed");
            expect(out.ports.length).toBeGreaterThan(0);
            expect(out.names.length).toBeGreaterThan(0);
            tried++;
          }
          leave(input);
        }
      }
      panel.done();
      panel = null;
    }
  }
  // Boxes × texts across every preset, both ways round; a sweep that
  // reached nothing would pass for the wrong reason.
  expect(tried).toBeGreaterThan(1000);
}, 600_000);

test("turning automatic off keeps the full solved figure, not the rounded one", () => {
  const tab = open(defaultTrain());
  panel = mounted(TrainPanel, { tab });
  let sharper = 0;
  for (const sel of selections(tab.train)) {
    tab.view.selection = sel;
    flushSync();
    const rows = [...panel.target.querySelectorAll<HTMLLabelElement>("label.auto")].filter(
      (l) => l.querySelector("input:disabled") && l.querySelector(".sw.auto button"),
    );
    for (const l of rows) {
      const shown = l.querySelector("input")!.value;
      if (shown === "") continue;
      (l.querySelector(".sw.auto button") as HTMLButtonElement).click();
      flushSync();
      const now = l.querySelector("input")!.value;
      // What the box holds now reads back as itself, and is the figure it
      // showed rounded to four places.
      expect(Math.abs(Number(now) - Number(shown))).toBeLessThanOrEqual(0.5e-4);
      if (now !== shown) sharper++;
    }
  }
  expect(sharper).toBeGreaterThan(0);
});

test("a material override survives an emptied box, and × restores the library's", () => {
  const tab = open(defaultTrain());
  panel = mounted(TrainPanel, { tab });
  tab.view.selection = { mesh: 0 };
  flushSync();
  const density = () => row(panel!.target, "ui.train_density");
  const overrides = () => tab.train.shape.members[tab.train.shape.meshes[0].a].gear.material_overrides;
  const library = density().querySelector("input")!.value;
  type(density().querySelector("input")!, "7900");
  expect(overrides().density).toBe(7900);
  type(density().querySelector("input")!, "");
  expect(overrides().density).toBe(7900);
  leave(density().querySelector("input")!);
  (density().querySelector("button.clear") as HTMLButtonElement).click();
  flushSync();
  expect(overrides().density).toBeNull();
  expect(density().querySelector("input")!.value).toBe(library);
});

test("clearing the throw box keeps the throw as the input", () => {
  workspace.create();
  const tab = workspace.selected!;
  panel = mounted(GearPanel, { tab });
  setKind(tab, "eccentric");
  flushSync();
  const throwRow = row(panel.target, "ui.gear_axis_distance_throw");
  (throwRow.querySelector("button") as HTMLButtonElement).click();
  flushSync();
  expect(tab.throwIsInput).toBe(true);
  const held = tab.eccentricThrow;
  type(box(panel.target, "ui.gear_axis_distance_throw"), "");
  expect(tab.throwIsInput).toBe(true);
  expect(tab.eccentricThrow).toBe(held);
  expect(t("ui.validation_required")).toBe(
    row(panel.target, "ui.gear_axis_distance_throw").querySelector("small.err:not(.hidden)")?.textContent?.trim(),
  );
});
