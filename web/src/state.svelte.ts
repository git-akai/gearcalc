// Application state: the open gear tabs.
//
// Per DESIGN.md §3.1, inputs are the only state. Nothing derived is stored, so
// nothing can go stale — every output on screen is recomputed from these by
// Rust on each change.

import {
  defaults,
  defaultLibrary,
  defaultTrain,
  type CutterRef,
  importLibrary,
  type ClassRef,
  type GearParams,
  type MaterialLibrary,
  type Train,
} from "./core";

export interface GearTab {
  id: number;
  name: string;
  params: GearParams;
  /** Pin or ball diameter for the over-pins measurement, mm. */
  pinDiameter: number;
  /** null means "whatever the standard defaults to for this gear". */
  toleranceClass: ClassRef | null;
  /** Export accuracy, mm. */
  chordTolerance: number;
  referenceCircles: boolean;
  /** An internal (ring) gear rather than an external one. */
  internal: boolean;
  /** The pinion cutter that shapes it, used only when internal. */
  cutter: CutterRef;
}

export interface TrainTab {
  id: number;
  name: string;
  train: Train;
}

let nextId = 1;
let nextTrainId = 1;

// Every value here comes from the core — see `defaults()` in core.ts, and
// DESIGN.md §12 for what happened when they were written down twice. That is
// why a tab cannot be built before the core is loaded, and why the two lists
// below start empty and are filled by `initialise`.
function freshTab(name = "Gear"): GearTab {
  const d = defaults().gear;
  return {
    id: nextId++,
    name,
    params: d.params,
    pinDiameter: d.pin_diameter,
    toleranceClass: null,
    chordTolerance: d.chord_tolerance,
    referenceCircles: d.reference_circles,
    internal: false,
    cutter: d.cutter,
  };
}

class Workspace {
  tabs = $state<GearTab[]>([]);
  selectedId = $state<number>(1);

  /** The first tab, once the core can say what is in it.
   *
   *  Deliberately not `create()`: that also *selects*, and selecting switches
   *  the main panel. Both lists are initialised at start-up, so whichever ran
   *  second would decide which panel the app opened on. */
  initialise() {
    if (this.tabs.length > 0) return;
    const t = freshTab();
    this.tabs.push(t);
    this.selectedId = t.id;
  }

  get selected(): GearTab {
    return this.tabs.find((t) => t.id === this.selectedId) ?? this.tabs[0];
  }

  select(id: number) {
    this.selectedId = id;
    trains.active = "gear";
  }

  create() {
    const t = freshTab();
    this.tabs.push(t);
    this.select(t.id);
  }

  /** Duplicates the tab, name included, as the specification requires. */
  copy(id: number) {
    const src = this.tabs.find((t) => t.id === id);
    if (!src) return;
    const t: GearTab = {
      ...structuredClone($state.snapshot(src)),
      id: nextId++,
    };
    this.tabs.splice(this.tabs.indexOf(src) + 1, 0, t);
    this.selectedId = t.id;
  }

  /** Deleting the last tab leaves a fresh default one, not an empty screen. */
  remove(id: number) {
    const i = this.tabs.findIndex((t) => t.id === id);
    if (i < 0) return;
    this.tabs.splice(i, 1);
    if (this.tabs.length === 0) {
      this.tabs.push(freshTab());
    }
    if (!this.tabs.some((t) => t.id === this.selectedId)) {
      this.selectedId = this.tabs[Math.min(i, this.tabs.length - 1)].id;
    }
  }
}

export const workspace = new Workspace();

function freshTrain(name = "Geartrain"): TrainTab {
  return { id: nextTrainId++, name, train: defaultTrain() };
}

/** The geartrain tabs.
 *
 *  Deliberately a separate list from the gear tabs rather than one list of a
 *  union type: the two share no fields, and the sidebar shows them under
 *  separate headings anyway. */
class Trains {
  tabs = $state<TrainTab[]>([]);
  selectedId = $state<number>(1);

  /** The first tab, without switching to it — see `Workspace.initialise`. */
  initialise() {
    if (this.tabs.length > 0) return;
    const t = freshTrain();
    this.tabs.push(t);
    this.selectedId = t.id;
  }

  /** Which list the main panel is showing. */
  active = $state<"gear" | "train">("gear");

  get selected(): TrainTab {
    return this.tabs.find((t) => t.id === this.selectedId) ?? this.tabs[0];
  }

  select(id: number) {
    this.selectedId = id;
    this.active = "train";
  }

  create() {
    const t = freshTrain();
    this.tabs.push(t);
    this.select(t.id);
  }

  copy(id: number) {
    const src = this.tabs.find((t) => t.id === id);
    if (!src) return;
    const t: TrainTab = { ...structuredClone($state.snapshot(src)), id: nextTrainId++ };
    this.tabs.splice(this.tabs.indexOf(src) + 1, 0, t);
    this.select(t.id);
  }

  /** Deleting the last tab leaves a fresh one, as for gears. */
  remove(id: number) {
    const i = this.tabs.findIndex((t) => t.id === id);
    if (i < 0) return;
    this.tabs.splice(i, 1);
    if (this.tabs.length === 0) this.tabs.push(freshTrain());
    if (!this.tabs.some((t) => t.id === this.selectedId)) {
      this.selectedId = this.tabs[Math.min(i, this.tabs.length - 1)].id;
    }
  }
}

export const trains = new Trains();

/** The material library.
 *
 *  Loaded from the core rather than defined here, so the shipped values and
 *  their provenance have exactly one home. Importing replaces the whole
 *  library, as the specification requires: the file is the library, and nothing
 *  is written back except by an explicit export. */
class Library {
  materials = $state<MaterialLibrary>({ material: [] });
  /** Set when the last import failed, so the sidebar can say why. */
  error = $state<string | null>(null);
  /** Name of the file the current library came from, if it was imported. */
  origin = $state<string | null>(null);

  /** Called once the wasm core is ready — the defaults live inside it. */
  loadDefaults() {
    this.materials = defaultLibrary();
    this.origin = null;
    this.error = null;
  }

  import(text: string, filename: string) {
    const r = importLibrary(text);
    if ("error" in r) {
      // The previous library survives a bad import. Replacing it with nothing
      // would lose the user's working set over a typo.
      this.error = r.error;
      return;
    }
    this.materials = r.ok;
    this.origin = filename;
    this.error = null;
  }

  get names(): string[] {
    return this.materials.material.map((m) => m.name);
  }
}

export const library = new Library();
