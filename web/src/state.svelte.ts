// Application state: the open gear tabs.
//
// Per DESIGN.md §3.1, inputs are the only state. Nothing derived is stored, so
// nothing can go stale — every output on screen is recomputed from these by
// Rust on each change.

import { defaultParams, type ClassRef, type GearParams } from "./core";

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
}

let nextId = 1;

function freshTab(name = "Gear"): GearTab {
  return {
    id: nextId++,
    name,
    params: { ...defaultParams },
    pinDiameter: 1.75,
    toleranceClass: null,
    chordTolerance: 0.001,
    referenceCircles: true,
  };
}

class Workspace {
  tabs = $state<GearTab[]>([freshTab()]);
  selectedId = $state<number>(1);

  get selected(): GearTab {
    return this.tabs.find((t) => t.id === this.selectedId) ?? this.tabs[0];
  }

  select(id: number) {
    this.selectedId = id;
  }

  create() {
    const t = freshTab();
    this.tabs.push(t);
    this.selectedId = t.id;
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
