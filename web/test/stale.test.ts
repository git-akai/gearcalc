// web#0: the gear tab's boxes show what the tab holds, always.
//
// The panel kept a second copy of each box's text, written only on input, so
// anything else that changed a parameter — a change of kind, a neighbour
// moving a bound — left the box, or its complaint, describing a gear that
// was not the one drawn and exported.
import { flushSync } from "svelte";
import { afterEach, beforeEach, expect, test } from "vitest";
import GearPanel from "../src/GearPanel.svelte";
import { setKind, workspace, type GearTab } from "../src/state.svelte";
import { box, complaint, mounted, type } from "./dom";

let tab: GearTab;
let panel: ReturnType<typeof mounted>;

beforeEach(() => {
  workspace.create();
  tab = workspace.selected!;
  panel = mounted(GearPanel, { tab });
});
afterEach(() => panel.done());

const DEDENDUM = "ui.gear_field_dedendum";
const ROOT_RADIUS = "ui.gear_field_root_radius";
const AMPLITUDE = "ui.gear_field_angular_shift";

test("a change of kind that resets a field shows the reset value", () => {
  type(box(panel.target, DEDENDUM), "1.4");
  expect(tab.params.dedendum).toBe(1.4);
  setKind(tab, "internal");
  flushSync();
  setKind(tab, "external");
  flushSync();
  expect(box(panel.target, DEDENDUM).value).toBe(String(tab.params.dedendum));
  expect(tab.params.dedendum).not.toBe(1.4);
});

test("the eccentric amplitude shows its reset after a round trip through external", () => {
  setKind(tab, "eccentric");
  flushSync();
  type(box(panel.target, AMPLITUDE), "0.3");
  expect(tab.params.angular_shift).toBe(0.3);
  setKind(tab, "external");
  flushSync();
  setKind(tab, "eccentric");
  flushSync();
  expect(box(panel.target, AMPLITUDE).value).toBe(String(tab.params.angular_shift));
});

test("an accepted value is flagged when a neighbour tightens its bound", () => {
  type(box(panel.target, DEDENDUM), "1");
  type(box(panel.target, ROOT_RADIUS), "0.5");
  expect(tab.params.root_radius).toBe(0.5);
  expect(complaint(panel.target, ROOT_RADIUS)).toBeNull();
  type(box(panel.target, DEDENDUM), "1.25");
  expect(complaint(panel.target, ROOT_RADIUS)).not.toBeNull();
});

test("a refused value's complaint clears when a neighbour widens its bound", () => {
  type(box(panel.target, DEDENDUM), "1.25");
  type(box(panel.target, ROOT_RADIUS), "0.5");
  expect(complaint(panel.target, ROOT_RADIUS)).not.toBeNull();
  type(box(panel.target, DEDENDUM), "1");
  expect(complaint(panel.target, ROOT_RADIUS)).toBeNull();
  // What the box shows is what the gear has.
  expect(box(panel.target, ROOT_RADIUS).value).toBe(String(tab.params.root_radius));
});
