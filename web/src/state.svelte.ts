// Application state: the open gear tabs.
//
// Per docs/rationale.md#inputs-are-the-only-state, inputs are the only state. Nothing derived is stored, so
// nothing can go stale — every output on screen is recomputed from these by
// Rust on each change.

import {
  defaults,
  defaultLibrary,
  defaultTrain,
  t,
  type CutterRef,
  FIELDS,
  type GearKind,
  type MateRef,
  importLibrary,
  importTrain,
  setLanguage,
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
  /** Which of the three kinds this tab holds. */
  kind: GearKind;
  /** The pinion cutter that shapes it, used only when internal. */
  cutter: CutterRef;
  /** What an eccentric gear runs against, for its commanded centre distance.
   *  Carried for every tab so switching kinds does not lose it. */
  mate: MateRef;
  /** When set, the eccentricity is sized by this centre-distance throw (signed,
   *  mm) and `params.angular_shift` becomes the value Rust solves for. `null`
   *  leaves the amplitude as the direct input. */
  eccentricThrow: number | null;
}

export interface TrainTab {
  id: number;
  name: string;
  train: Train;
  /** Which stages are expanded, by index.
   *
   *  **Not an input**, and deliberately beside `train` rather than in it: it
   *  changes no number, and what is exported is `train` alone, so a view
   *  preference cannot leak into a document that describes a gearbox
   *  (`docs/rationale.md`). It lives on the tab rather than in the panel
   *  because the panel is rebuilt whenever a reader looks at something else,
   *  and coming back to a train with every stage slammed shut is the kind of
   *  small forgetting that makes two tabs tiring to compare. It dies with the
   *  session, like every other thing here that is not the language. */
  open: Record<number, boolean>;
}

let nextId = 1;
let nextTrainId = 1;

// Every value here comes from the core — see `defaults()` in core.ts, and
// docs/corrections.md for what happened when they were written down twice. That is
// why a tab cannot be built before the core is loaded, and why the two lists
// below start empty and are filled by `initialise`.
// The name a tab starts with is the application's word for the thing, so it
// comes from the catalogue like every other word — a tab made while reading
// German is called "Zahnrad". It becomes the user's own the moment they type
// over it, and travels in the exported document as whatever it then says, which
// is why it is read once here rather than re-read on every render.
function freshTab(name = t("ui.gear_default_name")): GearTab {
  const d = defaults().gear;
  return {
    id: nextId++,
    name,
    params: d.params,
    pinDiameter: d.pin_diameter,
    toleranceClass: null,
    chordTolerance: d.chord_tolerance,
    referenceCircles: d.reference_circles,
    kind: "external",
    cutter: d.cutter,
    mate: { teeth: 43, profile_shift: 0, internal: false },
    eccentricThrow: null,
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

  /** Rename tabs still carrying the application's own word for a fresh tab.
   *
   *  A tab's name starts as the catalogue's and becomes the reader's the moment
   *  they type over it — so a name that still *is* the old language's default
   *  was never theirs, and leaving it behind is how an English session ends up
   *  with a Chinese tab in it. A name they typed cannot match, so it is not
   *  touched, which is the whole of the distinction the two cases need. */
  relabelDefaults(from: string, to: string) {
    if (from === to) return;
    for (const tab of this.tabs) if (tab.name === from) tab.name = to;
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

/** Change a tab's type, returning every field the new type does not use to its
 *  default.
 *
 *  **Why this is not just `tab.kind = kind`.** A type-specific input left behind
 *  keeps acting: switching an eccentric gear back to external left its shift
 *  amplitude set, so the gear stayed eccentric with no control on screen to say
 *  so. Which fields belong to which type is already written down once, in
 *  `FIELDS`, so this reads that rather than listing them again — a second list
 *  would be the thing that goes stale when a fourth type arrives.
 *
 *  The cutter and the mate are not reset: they are separate objects rather than
 *  gear parameters, they reach no answer unless their type is active, and a
 *  designer who set a cutter, looked at the gear as an external one and came
 *  back would not thank us for having cleared it. */
export function setKind(tab: GearTab, kind: GearKind) {
  const fallback = defaults().gear.params;
  for (const f of FIELDS) {
    if (f.kinds && !f.kinds.includes(kind)) {
      tab.params = { ...tab.params, [f.key]: fallback[f.key] };
    }
  }
  // The throw sizing is a mode on `angular_shift`, so it goes back with it —
  // a gear that is no longer eccentric is sized by nothing.
  if (kind !== "eccentric") tab.eccentricThrow = null;
  tab.kind = kind;
}

export const workspace = new Workspace();

function freshTrain(name = t("ui.train_default_name")): TrainTab {
  // The first stage open, as a fresh panel has always shown it.
  return { id: nextTrainId++, name, train: defaultTrain(), open: { 0: true } };
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

  /** Rename tabs still carrying the application's own word for a fresh tab.
   *
   *  A tab's name starts as the catalogue's and becomes the reader's the moment
   *  they type over it — so a name that still *is* the old language's default
   *  was never theirs, and leaving it behind is how an English session ends up
   *  with a Chinese tab in it. A name they typed cannot match, so it is not
   *  touched, which is the whole of the distinction the two cases need. */
  relabelDefaults(from: string, to: string) {
    if (from === to) return;
    for (const tab of this.tabs) if (tab.name === from) tab.name = to;
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

  /** Set when the last import failed, so the panel can say why. */
  importError = $state<string | null>(null);

  /** Import creates a **new tab**, as the specification requires: reading a
   *  file never overwrites what is open, and nothing is written back to it. The
   *  file's own name for the train comes with it.
   *
   *  A train naming a material this library does not have still imports — the
   *  file is valid and the library is a separate document. The solve says which
   *  material is missing, where the user can act on it. */
  import(text: string) {
    const r = importTrain(text);
    if ("error" in r) {
      this.importError = r.error;
      return;
    }
    const t: TrainTab = {
      id: nextTrainId++,
      name: r.ok.name,
      train: r.ok.train,
      open: { 0: true },
    };
    this.tabs.push(t);
    this.importError = null;
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

/** Switch language, carrying the names the *application* chose across with it.
 *
 *  One home for the whole of what a language change means, because there are two
 *  ways to ask for one and they must not answer differently: the picker, and
 *  another copy of the application in the same browser changing the stored
 *  preference underneath this one.
 *
 *  A tab's name starts as the catalogue's word and becomes the reader's the
 *  moment they type over it, so a name that still *is* the outgoing default was
 *  never theirs and follows the language like every other label. One they typed
 *  cannot match it and is left alone. */
export function applyLanguage(tag: string) {
  const before = {
    gear: t("ui.gear_default_name"),
    train: t("ui.train_default_name"),
  };
  setLanguage(tag);
  workspace.relabelDefaults(before.gear, t("ui.gear_default_name"));
  trains.relabelDefaults(before.train, t("ui.train_default_name"));
}
