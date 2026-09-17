# Geartrain Model Refactor — Handoff

**Scope:** replace archetype-driven epicyclic handling with a single netlist-style graph model and one linear solver. Covers Wolfrom, planetary differential (meshed planet sets), simple planetary, and anything else built from the same primitives, including trains with mobility > 1 (multi-input differentials, power splits, free members).

**Status:** design only. No implementation assumed. Migration sequencing at the end.

---

## 1. Premise

Every train in scope is the same object seen three ways. Write each mesh in the frame of the member carrying its axes and the epicyclic character disappears, leaving an ordinary ratio constraint:

```
N_i · (ω_i − ω_f) + σ · N_j · (ω_j − ω_f) = 0
```

- `f` — the **frame member** of that mesh (the member in whose rotating frame both gear axes are stationary)
- `σ` — mesh sign: `+1` external/external, `−1` external/internal

This covers sun–planet, ring–planet, planet–planet, and fixed-axis gearing (`f` = housing). Consequences:

- **There is no such thing as a sun, a ring, or a planet.** There is a gear with a tooth count, a mesh sign, and a frame. Sun/ring/planet are geometry and drawing concerns and belong in the presentation layer as tags, never in the kinematic model.
- **Wolfrom, planetary differential, and simple planetary are not types.** They are three tick patterns in the same mesh graph.
- **"Fixed" and "free" are not structural.** They are boundary conditions applied after the graph is solved for its nullspace.

If the refactor commits to this, the tool never needs a topology enumeration, a per-archetype formula table, or a branch per configuration. The current archetype code becomes test fixtures rather than logic.

---

## 2. Data model — five primitives

```jsonc
{
  "members": [
    // anything with a single angular velocity: carriers, central gears' shafts,
    // planet shafts, the housing. Housing is an ordinary member, pinned by net.
    { "id": "C1",  "label": "carrier" },
    { "id": "S",   "label": "sun" },
    { "id": "R1",  "label": "fixed ring" },
    { "id": "R2",  "label": "output ring" },
    { "id": "P_A", "label": "planet shaft A" },
    { "id": "GND", "label": "housing" }
  ],

  "gears": [
    // carriedBy = member whose frame the axis is fixed in
    // ownedBy   = shaft it rotates with
    { "id": "s",  "teeth": 24, "internal": false, "carriedBy": "GND", "ownedBy": "S"   },
    { "id": "r1", "teeth": 72, "internal": true,  "carriedBy": "GND", "ownedBy": "R1"  },
    { "id": "r2", "teeth": 75, "internal": true,  "carriedBy": "GND", "ownedBy": "R2"  },
    { "id": "a1", "teeth": 24, "internal": false, "carriedBy": "C1",  "ownedBy": "P_A" },
    { "id": "a2", "teeth": 25, "internal": false, "carriedBy": "C1",  "ownedBy": "P_A" }
  ],

  "meshes": [
    // frame and sigma are STORED, with defaults derived on creation.
    { "a": "s",  "b": "a1", "frame": "C1", "sigma":  1 },
    { "a": "r1", "b": "a1", "frame": "C1", "sigma": -1 },
    { "a": "r2", "b": "a2", "frame": "C1", "sigma": -1 }
  ],

  "nets": [
    // rigid coupling classes over members. Coaxial output couplings,
    // locked clutches, and grounding are all the same mechanism.
    { "id": "NET_GND", "members": ["GND"] },
    { "id": "NET_IN",  "members": ["S"]   }
  ],

  "conditions": [
    // per net: ground | drive | free
    { "net": "NET_GND", "type": "ground" },
    { "net": "NET_IN",  "type": "drive", "omega": 1 }
  ]
}
```

Notes on the two stored-with-default fields, both of which matter:

- **`sigma` explicit.** The default `internal ⇒ −1` rule holds for parallel-axis spur/helical only. Bevel side-gear pairs, face gears, and worm sets break it. One stored field per mesh keeps bevel differentials in scope later without touching the solver.
- **`frame` explicit.** Default it to the deeper `carriedBy` of the two gears (central gear axes are coincident with the carrier's axis of rotation, so they are stationary in the carrier frame). Multi-carrier and nested-carrier trains make the inference ambiguous; store the resolved value and let the UI show it.

Everything else — planet count, phasing, module, profile shift, face width, bearings, bearing spans, material — hangs off these as annotations and **must not enter the solve**.

---

## 3. Solver

Assemble one row per constraint over the vector of member angular velocities `ω`:

| Source | Row |
|---|---|
| mesh | `N_i·ω_i + σ·N_j·ω_j − (N_i + σ·N_j)·ω_f = 0` |
| net coupling | `ω_i − ω_j = 0` for each pair in the class |
| ground condition | `ω_i = 0` |
| drive condition | `ω_i = value` |

Use exact rational arithmetic (BigInt fraction). Ratios are quotients of integers; float comparison will produce spurious "not equal" results in the recognizer, the closure checks, and regression tests.

Two structural facts that come free and should be used as internal assertions:

1. The all-ones vector satisfies every mesh row and every coupling row. **Lock-up is always in the nullspace.** If it isn't, the assembler has a sign or frame bug.
2. Mobility `m = dim(null(A_structural))`, where `A_structural` is mesh + coupling rows only, computed **over the connected component of interest**. Members with no incident constraint inflate `m` spuriously — detect and report them as disconnected rather than counting them.

The tool then requires exactly `m` independent boundary conditions. It never asks "which is input, which is output." It solves and reports, and every pair of nets has a defined ratio.

---

## 4. Mobility > 1 — the multi-input / multi-output case

This is where the current archetype approach fails hardest and where the refactor pays off most, so it gets its own treatment.

### 4.1 Under-determined is an answer, not an error

With `k < m` boundary conditions, do **not** refuse. Solve for the affine family and present it as superposition. The user picks which nets are the free parameters (default: the un-conditioned drives); every other member's speed comes back as an exact linear combination:

```
ω_out = a·ω_in1 + b·ω_in2        a, b exact rationals
```

That expression *is* the differential equation for the train. For a two-input summing differential it is the entire useful output, and it is strictly more informative than any single ratio. The UI should show the coefficient table alongside the speed table and let the user scrub either parameter to see the speeds move.

### 4.2 Multi-input and multi-output are the same kinematic object

Two drives and one load (summing differential) and one drive with two loads (power split) produce **identical constraint systems**. The distinction is entirely in the torque/load boundary conditions, not the kinematic ones. The model should not attempt to distinguish them, and the UI should not ask the user to declare "input" vs "output" as a structural property. Direction of power flow is a *result*, derived in the torque layer (§4.4), and it can reverse across the operating range — which is exactly the recirculating-power condition worth warning about.

### 4.3 A "free" member is a member with no torque on it

Free is not a third kind of kinematic condition; it is the absence of one, plus the torque statement `τ = 0`. That is why a free member leaves residual DOF: the missing constraint is supplied by load, not geometry. The UI should say this rather than silently treating free as a solvable state.

### 4.4 Torque layer (ideal, optional but cheap)

For a lossless train, power conservation gives `Σ τ_i ω_i = 0` for **every** `ω` in the nullspace. Therefore:

```
τ ∈ null(A_structural)^⊥ = rowspace(A_structural)
```

dim = `M − m`. Free members contribute `τ_i = 0` rows. This is a second linear solve on the transpose of the matrix already assembled — no new model, no new data. It yields:

- Reaction torque at ground (sizing the housing interface)
- Torque split at a power-split node, fully determined by geometry with only magnitude free
- Circulating power detection: compute `τ_i·ω_i` per member and per mesh; a mesh carrying power far exceeding the external input is the recirculation flag

**Caveat to state plainly in the UI:** this is the ideal case. Real mesh efficiency is sign-dependent (which member drives which), so introducing losses breaks linearity and requires iterating on an assumed power-flow direction. Ship the ideal version; gate the lossy version behind a clearly separate mode.

### 4.5 Visualization above 2 DOF

For `m = 2` the nullspace is 2-D, one basis vector is lock-up, and normalizing against it assigns **every member a position on a line** — the lever diagram. Node positions *are* the nullspace coordinates; pinning a node *is* applying a boundary condition; torque balance on the lever *is* the §4.4 solve. It is not a decorative overlay, it is the solution rendered.

It also makes pathologies legible: a high-reduction Wolfrom shows two nodes nearly coincident, which is visually the same fact as its sensitivity and its efficiency problem.

For `m ≥ 3` the lever becomes a plane and stops being intuitive. Fallback: let the user ground or drive down to 2 DOF and show the lever for that slice, with the grounded nodes marked. Do not attempt a 3-D lever.

---

## 5. Layer separation

Three layers, three panels, three edit frequencies. Keep them apart.

| Layer | Contents | Edit frequency |
|---|---|---|
| **Topology** | members, gears (as graph nodes), meshes, nets | rare |
| **Sizing** | tooth counts, module, profile shift, planet count, face width | frequent |
| **Conditions** | ground / drive / free per net | constant |

Topology determines the *form* of the ratio. Sizing determines its *value* and whether the train physically closes. Conditions determine the *operating mode*.

**A geometrically infeasible train is still kinematically well-defined and must still solve.** Refusing to report a ratio because center distances don't yet close would be the single most obstructive behaviour the tool could have — that state is the normal working state during Wolfrom sizing. Closure failures are advisories, never gates.

The stated requirement that any member can be fixed or free is a property of layer 3 alone: a per-net toggle with instant re-solve, no structural edit.

---

## 6. Input surface — the mesh grid

Rows are central members. Columns are planet shafts, subdivided into the gears on each shaft. A cell is a mesh; `σ` is inferred from the row's internal flag, so the cell is a checkbox.

```
                    │ shaft A     │  gutter  │ shaft B     │
                    │  a1    a2   │          │  b1    b2   │
  sun    S          │  ●          │          │             │
  ring   R1         │        ●    │          │             │
  ring   R2         │             │          │  ●          │
```

Between adjacent columns sits a **gutter toggle** with three states:

- **independent** — separate planet shafts
- **rigid** — same shaft (compound planet: two gears, one `ownedBy`)
- **meshed** — planet-to-planet mesh, `frame` = the shared carrier

That single control is the entire difference between Wolfrom and planetary differential. Flipping it reverses the sense of the second half and the ratio updates live. The interface gesture and the kinematic change are the same move, which makes the relationship between the two architectures discoverable rather than documented.

The nine Wolfrom variants in scope (each half meshing sun+ring, ring only, or sun only) are tick patterns. Simple planetary is one column, two rows ticked. Adding rows and columns reaches Ravigneaux and Simpson with no model change.

The grid is, structurally, the constraint matrix. The artifact the user edits and the artifact the solver consumes are the same object.

---

## 7. Validation — advisory, ordered by cost of being wrong

1. **Mobility report.** `k < m` → present the family (§4.1). `k > m` → locked or inconsistent; name the redundant or conflicting conditions.
2. **Disconnected members.** Flag rather than silently inflating `m`.
3. **Geometric closure.** A compound planet meshing two different central pairs imposes matched center distances. Report the residual in mm; offer absorption via profile shift or per-mesh module. Most real Wolfrom designs live or die here.
4. **Assembly condition.** Integer condition for `k` equally spaced planets; report the nearest satisfying counts.
5. **Planet–planet clearance.** Tip clearance between adjacent planets, and for the meshed-planet case, interference between the two planet sets.
6. **Health warnings.** Basic-train ratio magnitude; recirculating power (§4.4); planet spin speed relative to input; **sensitivity** — near-singular ratios where ±1 tooth swings the output by orders of magnitude, which is the normal Wolfrom operating regime and should be surfaced as a computed dR/dN rather than a footnote.

---

## 8. Refactor sequencing

**Phase 0 — characterize.** Before touching anything, dump the existing tool's ratio output across every configuration it currently supports, for a spread of tooth counts and every fixed/free permutation. This becomes the golden regression set. The existing archetype formulas stop being code and become expected values.

**Phase 1 — core as a side-by-side library.** Implement primitives + assembler + rational solver as a pure module with no UI dependency. Run it against the Phase 0 set. Discrepancies are almost always frame or `σ` assignment; the lock-up assertion (§3) localizes them fast.

**Phase 2 — adapter.** Map every legacy saved configuration onto the graph model. Versioned file format with a one-way importer. Legacy presets survive as **saved graphs**, not as types.

**Phase 3 — UI swap.** Mesh grid + condition toggles + lever. Presets instantiate primitives and then get out of the way. Optionally run a **recognizer** that pattern-matches the current graph against canonical archetypes and displays the name as a caption only — never as state, never affecting the solve. Canonicalize the graph (sorted, normalized ids) so recognition is an exact structural comparison.

**Phase 4 — delete.** Remove archetype branches, per-type formulas, and any type enum. If any of it is still needed, Phase 0 was incomplete.

**Phase 5 — torque layer** (§4.4), then the inverse problem: given a target ratio, search integer tooth counts satisfying closure and assembly simultaneously. That search is the actual hard part of Wolfrom design and it sits directly on the same five primitives.

---

## 9. Extension surface

Free, with no model change:

- multi-stage compound trains (more carriers, joined by nets)
- fixed-axis stages (carrier in the ground net)
- idlers (planet–planet mesh on a grounded carrier)
- stepped and double-planet arrangements
- clutch/brake schedules — a set of condition presets over one unchanged graph, i.e. a transmission gear table
- multi-input differentials and power splits (§4)

Requires the hooks already specified:

- **bevel / face / worm sets** — explicit `σ`, plus an axis-orientation field when non-parallel axes enter scope
- **nested carriers** — explicit `frame`

Out of scope and worth stating so it isn't half-built: backlash, compliance, dynamics, gear tooth stress. All are annotations on the same graph if they ever land.

---

## 10. Open questions for the receiver

1. **How does the existing tool represent a configuration?** If it's an archetype enum plus a tooth-count tuple, Phase 2 is mechanical. If fixed/free is baked into the archetype identity (e.g. separate types for sun-in-ring-fixed vs ring-in-sun-fixed), the adapter needs to split identity into topology + conditions, and the preset count collapses substantially — worth flagging to whoever owns the preset library.
2. **Does the sizing layer need to carry real geometry** (module, profile shift, center-distance solving, efficiency), or is this ratio exploration? The layer separation holds either way, but geometry-carrying promotes closure from validator to first-class solver.
3. **Interface shape** — the mesh grid assumes GUI. A DSL serialization of the same model is straightforward, but the gutter-toggle argument loses its force in text. If both, the grid should be a view over the DSL rather than a parallel representation.
4. **How far does multi-DOF need to go in v1?** Presenting the superposition family (§4.1) is cheap. The torque layer (§4.4) is one extra transpose solve. Lossy power flow is meaningfully harder and should be scoped separately.
