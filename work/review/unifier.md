# unifier — design review of gear-core toward universal models

Branch audit-ablation @ 03f9823. Production code only; test modules skipped. I checked each claim against audit/plan.md, workstreams T03/T05/T06/T07/T08/T10/T11/T12/T14/T15 and ledger.json.

Prototype script: `notes/unifier_blt_flow.py` (pure Python, shares no code with the crate).

## 0. Verdict

The audit's "one signed model per phenomenon" direction is right for **generation** (σ, κ). It is too timid in three places, where one structural idea would replace several separately planned tasks:

1. **Flow.** A block-triangular (BLT) decomposition makes the power flow exact and linear on trees. It replaces T11.4's seeded iteration and fallback. (U2)
2. **Closure.** One decomposition of the closure Jacobian gives the plan, the relief matching, the closed-form absorbers and the search components. (U3)
3. **Load.** One load field gives bending share, efficiency and contact stress, for line and point contact alike. (U4)

It is right **not** to merge line and point contact geometry.

## 1. Generation: rack, shaper, ring

### Parameters

One generator needs four inputs:
- the workpiece side σ = ±1;
- the tool's pitch curvature κ_c = 1/r′_c, with the rack at 0;
- the corner depth b_c and the corner round ρ;
- the phase.

σ and κ are **orthogonal**. The code conflates them: "is internal" is read off "has a cutter" (`Member.ring: Option<Cutter>`, shape.rs:470-479; lens-unification#2).

A single domain predicate follows from these parameters: **κ_c + σ·κ_w > 0**, where κ_w = 1/r_w is the workpiece's pitch curvature. It replaces four rules:
- `RingTooSmall` (mesh.rs:260);
- gear.rs:919;
- the cutter-teeth clamp (ring.rs:283-289, CLAMP_CUTTER_TEETH_REDUCED);
- the rule that "a rack cannot cut a ring": κ_c = 0 with σ = −1 fails the predicate.

The same predicate, applied to the two meshing pitch circles, is Mesh::new's ring-size rule. (U7)

### What differs today

| Concern | Rack (external) | Shaper (ring) |
|---|---|---|
| corner centre / trochoid | tooth.rs:539-545 (straight-line corner path) | shaper.rs:227-280 (circular corner path) |
| point and tangent | strength.rs:299-318 | shaper.rs:287-322 |
| curvature | strength.rs:337-352 | shaper.rs:340-383 (both end in tooth.rs:796) |
| θ-slope | tooth.rs:562-575 | none |
| junction | tooth.rs:583-645 (closed form, or a march+Brent crossing when undercut) | ring.rs:528-560 (closed-form radius, then a march+Brent for travel; ring#11) |
| severing / root end | tooth.rs:647-720 | ring.rs:563-575 |
| thickness clamp | tooth.rs:823-846 (on the tooth) | ring.rs:240-256 (the same constants, on the space: s = m(π/2 + 2σx tan α)) |
| sections / sampling / profile | tooth.rs:725-795 | ring.rs:771-850 |
| involute flank | strength.rs:357-400 | ring.rs:680-745 |
| outline | gear.rs:544-640 | ring.rs:827 |
| cut simulation | verify.rs:129 | verify.rs:540 |
| dispatch | `BuiltMember` shape.rs:2365-2426 (20 arms) | `auto::Cut` auto.rs:1068-1145 (10 arms), `ToothOutline` ×2 strength.rs:508/573 |

With RollingCorner (T03.13) plus σ-generic flank and section helpers, the following go:
- about 600 production lines of ring.rs (171-850);
- strength.rs:299-400;
- `BuiltMember`, `auto::Cut::{ByRack, ByShaper}` and one `ToothOutline` impl;
- `as_gear` (shape.rs:2805);
- later, RingMesh (ring.rs:849-1101, T05.12).

### The only genuinely new mathematics

The undercut crossing and the severing criterion for a **pinion** cutter on an external workpiece. The rack's versions are at tooth.rs:583-720. Everything else is already σ-generic in `ShaperCut`.

### Skeptical notes

- T03.15 says to commit the merge only together with the shaper-cut-external feature. The split already causes **live** rule-4 bugs (U1), so steps 1–2 (σ helpers, one outline walker) and U1 should land now.
- Keep `verify.rs` as the instrument: a σ-generic simulation, but built from the basic tool (T03.1).
- Keep as data keyed by σ, not as branches:
  - the ISO rim fits EXTERNAL_RIM / INTERNAL_RIM (strength.rs:1152/1160);
  - the 30°/60° tangent angles (strength.rs:145/172).

  They are transcribed standards.

## 2. Mesh: parallel against crossed

### Is parallel helical the Σ → 0 limit of the screw model?

**Pointwise**, yes: the sliding vector, the curvature across, the normal and the ratio all converge. The Hertz ellipse's lengthwise curvature tends to 0, so the contact goes to a line.

**Not uniformly**:
- **Centre-distance law.** Crossed is a = a_ref + m_n·Σx; parallel uses inv α_w. They differ at second order in the shift. The branch is shape.rs:539-557. The seam is recorded in reference.md:946-949, but its size is not.
  - The crossed law holds only while the contact slides Δa/Σ along the face. That slide leaves any finite face as Σ → 0, so the true limit is a face-bounded boundary layer (T07.2, T07.4).
- **Dimension.** The crossed path is 1-D along the line of action. The parallel field of action is 2-D (ε_α × ε_β). The crossed "line length" is min b_i/cos β_bi (shape.rs:2564-2567), not the parallel total contact-line length.
- **Play.** shape.rs:2631-2643 against mesh.rs backlash: the point form floors at 0 (added#59).
- **Efficiency.** Differs at O(μ²), plus a pitch-point fallback (shape.rs:2491; lens-unification#7).
- **Rating seams.** Peak pressure 5 %, pitch pressure 1.5 % (reference.md:1152-1168; lens-unification#8). Widths jump 0.77 → 10 mm (T08.12). Bending is absent for point contact (shape.rs:3214).

### Recommendation

**Do not** build one geometric model: a general TCA of two involute helicoids with face bounds.
- It would replace ISO closed forms with a numerical engine.
- The Σ → 0 limit is a boundary layer.
- The Coulomb seam at zero slip is physical either way.

**Do** unify the **interface** at core level. Add a `Path` trait over `ContactPath` and `CrossedPath`:
- `positions()` and `single_pair()`;
- `curvatures_at`;
- `force_at(torque, drive, μ)`;
- `line_length_at`;
- `play(a, clearance)`;
- `flank_interference`.

Keep the one Σ = 0 dispatch at the constructor. This is T14.8, with the methods moved into the core instead of `shape.rs`.

### Correction to lens-unification#7's verdict

The verdict keeps two efficiency models partly because "the closed form carries the optimality condition used by the efficiency search". That premise is false: `efficient_split` and `split_residual` have only test callers. auto.rs:1468 itself says the search does not use them.

The conclusion (keep the closed form) survives on cost alone: `contact::efficiency` is called per search trial (auto.rs:1271, shape.rs:2485). (U6)

## 3. Rating

### U4 — one load field

`LoadField { ε_α, ε_β, weighting }` answers, for any instant or position:
- the share of F carried by each pair in contact;
- the contact-line length L(t) each pair has.

It would be the one source for three consumers that today use different rules:
- **bending share**: `load_share`, contact.rs:892 (T06.5);
- **efficiency weighting**: 1/L(t) (T06.8);
- **contact stress**: F/L at the rating position, where today the load is F at b/cos β_b (strength.rs:1846).

The contact-stress case is what strength#6 (Z_ε omitted, +12–33 %) is about. The audit only records that as a bias (T08.3); it gets no structural fix. At ε_β ≥ 1 there is no single-pair point, and the field's max of w/L gives the rating position directly.

For a point contact the field degenerates to pairs along the line of action, with L taken from the Hertz patch (T08.11).

This closes or subsumes: strength#6, added#28 (patch length), T07.19 (single-pair definition), T06.5, T06.8 and the crossed half of T06.8.

### U5 — one rater

`PointBuilt::rate` (shape.rs:2552-2629) and `strength::contact_stress` (strength.rs:1821-1890) are the same algorithm: Hertz at {pitch, single-pair bounds} with a force and a length. `contact_stress` already takes `lengthwise_curvature`.

Make it one function over `Path` + `LoadField` + T08.11's `Patch`. Bending for point contacts follows once the patch spans the face (T08.12).

## 4. Kinematics, flow, relief and search

### U2 — flow by block-triangular decomposition (supersedes T11.4's method)

**Structure.**
- Rows: bodies with a known torque. Columns: mesh torques.
- A max matching, then Tarjan's strongly connected components, gives the blocks in dependency order.

**Why 1×1 blocks are exact.** In a 1×1 block from a member-body row, the coefficient is z or η·z. Its sign does not depend on the direction, so sign(c_k), and with it the direction, is forced. That makes the block closed-form.

**Where enumeration remains.** Only inside irreducible blocks, which are exactly the power-circulating loops (a Wolfrom's two ring meshes form a 2-block). The work is Σ 2^{b_i}, set by topology.

**Caveat.** A frame (carrier) row with an internal mesh has coefficient −(z_a + η z_b). Its sign can flip at low η, so it is enumerated (cost 2).

**Evidence** (prototype, 2,998 random cases, η ∈ [0.3, 0.995]):

| case | blocks | result |
|---|---|---|
| chains of 1–7 pairs | all 1×1 | exact, 2M trials |
| compound sets | all 1×1 | exact |
| planetary | (1,1) or (2) | matches |
| Wolfrom | (1,2) | 178 cases with more than one consistent branch |

- Against the 2^M enumeration it agreed on 2,997 of 2,998.
- The one mismatch was a tie at total efficiency 0: two branches, same efficiency.
- Across blocks with more than one candidate, a DFS over the candidates plus the global filters reproduces the max-efficiency choice.

**Consequences.**
- No seed and no fallback heuristic, unlike T11.4.
- T11.1's cap becomes a cap on block size.
- T11.3's rule applies inside a block.
- Rank is decided structurally per block, which is the sound part of kinematics-flow#7.
- Twin-layshaft indeterminacy (added2#8) is a structurally singular block, and can be named as a mesh loop.

**Related.** `kinematics::System::torques` (kinematics.rs:509) is test-only: the flow rebuilds Kᵀ·diag(η) in f64 (`per_unit`, flow.rs:193-212). Build the flow rows from `System`'s `MeshRow`s.

### U3 — closure: one Jacobian, one decomposition

**Unknowns.** Member shifts x, and running distances a_d.

**Rows.** Per mesh, x_a + σ x_b = g_k(a_d), where g is `shift_sum_for` (closed form) or the linear crossed law. Size readings are extra rows.

Its sparsity, decomposed by Dulmage–Mendelsohn / BLT, gives five things at once:
1. **Admissibility.** A perfect matching exists (T10.9's Kuhn matching).
2. **The plan's roles.** The block order gives given, reaches and absorbs, with no dependence on list order (T10.5, shape-a#1). Today this is plan_held's two loops (shape.rs:1222-1336).
3. **Closed-form absorbers.** 1×1 blocks are solved by `shift_sum_for` (T10.6, shape-a#12).
4. **Where a root-find remains.** Only irreducible blocks, such as a planet in two meshes on one free distance.
5. **Search dof and components.** The null space gives the dof (T12.2, T12.3, T12.10). This replaces the union-finds at shape.rs:1996 and :2060 (T14.7).

**Implementation.** One `structure.rs` (matching, strongly connected components, components) serves:
- U2;
- U3;
- `Shape::parts`;
- kinematics' elimination order, which is relevant to T11.7 but is a **hypothesis**, not measured.

**Not worth it.**
- Merging the numeric types: exact rationals for motion, f64 for closure. Share only the structure.
- An LP or LCP formulation for the flow: BLT is exact and simpler.
- A closed form for the search objective.

## 5. Matches on a kind (production only; counts from a scripted census)

| Kind | Count, by file | Removable by a parameter? |
|---|---|---|
| `BuiltMember` Rack/Ring | 20, shape.rs | **Yes**: σ/κ generator (§1) |
| `BuiltContact` Line/Point | 16, shape.rs; `.line()` 4 | Move into methods on `Path` (T14.8/U5). The enum stays at one constructor |
| `auto::Cut` ByRack/ByShaper/Pinned | 10 auto.rs, 3 shape.rs | **Yes**, with Pinned kept as a flag (T03.15) |
| `ring.is_some()` as σ | 9 shape, 5 edits, 3 arrangements, 1 mod | **Yes**: a signed count or `side` field. Includes U1 (shape.rs:1120, 1131, 1769) and arrangements.rs:518 (the 1 + σ(1 − x) rule, not listed by lens-unification#10) |
| `MeshKind::` variants | 8 shape, 5 mesh (the definition), 2 ring, 1 gear | shape.rs:690, 1057 via `sign()` (lens-unification#10); 1527/1576 are genuine ring queries |
| `is_crossed` / shaft angle == 0 | 8 shape, 1 edits | Keep one dispatch; the rest go to `Path` methods (539, 1182, 1429, 1460, 2141, 2847, 4123) |
| `CriticalSection` × `RootStressModel` | 12 strength, 1 shape | Collapse to one `BendingModel` (T08.8) |
| `screw::Flank` | 6, screw.rs | Delete for `Drive` (T07.18) |
| `Section` | 10 tooth, 8 ring, 5 strength | Piecewise curve type; keep. Halves with §1 |
| `LoadSharing` | 6 | User option; keep. Becomes a `LoadField` weighting (U4) |
| `Drive` | 34 (flow 8, contact 9, screw 8, …) | Representational; `flow.rs`'s `(on_a, on_b)` match could be `sign()`. Low value |
| `MeshSide` | 24 | Index; keep |
| `MemberRole`, plan `Role`, `Central` | 74 | Naming and elimination roles; keep (U3 derives `Role`) |
| `PresetFamily` / `Family` | 8, arrangements/mod | Read off the shape (T14.18) |

## 6. Findings

Each was checked against the ledger, and none is covered there unless the entry says otherwise.

**U1 (medium, high confidence). Rings silently ignore member options.**
- **Where.** shape.rs:1120-1122 returns early for a ring, so `no_sharp_tip` and `min_tip_width` (`addendum_asked`, mod.rs:1237) are silently ignored. shape.rs:1131-1140 gives rings no undercut floor, and shape.rs:1769 bounds a ring's search by `admissible_ranges`, while external members use `searchable_shift`.
- **Why.** These are the same rule written three ways by kind, and a ticked option does nothing. The σ-generic "what the tool permits" query cures it.
- **Ledger.** No match for `shape.rs:1120`, `addendum_asked`+ring, or `search_floor: None`. T05.13 covers the ring's own shift range, not these.

**U2 (medium).** Flow by BLT (§4). The prototype evidence is above. It supersedes T11.4's method and absorbs T11.1, T11.3, T11.12 and kinematics-flow#7.

**U3 (medium).** The closure decomposition (§4). It unifies T10.5, T10.6, T10.9, T12.2, T12.3, T14.7 and part of T12.10.

**U4 (medium).** `LoadField` (§3). It gives strength#6 a structural fix, where the audit only records the bias.

**U5 (low).** The duplicated rater (§3).

**U6 (low, audit wrong).** The premise of lens-unification#7's verdict (§2).

**U7 (low).** The domain predicate κ_c + σκ_w > 0 (§1).

**U8 (medium, medium confidence). Planet neighbour clearance.**
- **Where.** shape.rs:3647 uses `2·a·sin(π/N) − max tip diameter`.
- **Problem 1.** It assumes equal spacing even when `assembly()` says spacing is unequal.
- **Problem 2.** It never checks instances on another replicated axis of the same carrier, such as MeshedPlanets or Ravigneaux.
- **Fix.** Pairwise gaps from the positions T10.18 gives.
- **Ledger.** Not in the ledger. T10.19's "generic spacing note" is adjacent; check it before filing.

**U9 (low).** A dead exact-zero branch and a divide–multiply round trip: flow.rs:158-162 recovers c from c·z_a, and z_a ≠ 0 always. Store c.

**U10 (low).** `ContactPoint::efficiency` has an absolute ε threshold on a length (contact.rs:820-828). One bad sample voids `CrossedPath::efficiency` (screw.rs:1189) and feeds NaN to the locking Brent solve. This matters more once T15.16 lowers the sample count.

**U11 (low).** An unnamed tolerance: `slack = (end − start)·1e-9` in `load_fraction` (contact.rs:174). It can go once T06.1 cuts the path exactly.

**U12 (low).** The worm preset's `axial_clearance = 0.04` mm (arrangements.rs:714) is absolute, not scaled by module, and has no basis. It goes into T15.1's single home for defaults.

## 7. Migration (green at every step) and the tasks each step subsumes

**Step 0.** Bit-identity harness: corpus plus the tests/common grid. Then T03.1 and T16.3/T16.14.

**Step 1. `structure.rs`** (matching, strongly connected components, components). Swap in the union-finds and `Shape::parts`' partition. Proof: partition laws unchanged.
- Subsumes T14.7 and lens-architecture#11.

**Step 2. Flow by BLT,** with the enumeration moved to a `#[cfg(test)]` oracle. Proof: T11.4's oracle law, plus an op-count gate Σ 2^{b_i}.
- Subsumes T11.1, T11.3, T11.4, T11.12 (flow), T11.15, kinematics-flow#7 and lens-architecture#7.
- Names a mesh loop (added2#8).
- Also do U9.

**Step 3. Kinematics.** T11.5, then elimination in BLT order.
- Subsumes T11.7 if the hypothesis holds; otherwise T11.7 as written.

**Step 4. Closure Jacobian decomposition,** after T10.3 and T10.4.
- Subsumes T10.5, T10.6, T10.9, T12.2, T12.3, T12.6's single application point, shape-a#1, shape-a#12 and added2#17.
- Proof: T10.5's and T10.9's permutation and relief laws.

**Step 5. Contact.**
- T08.11 `Patch`, then a core `Path` trait (T14.8 relocated).
- Then U5, then `LoadField` (U4) with T06.5 as its first weighting.
- Then T06.8 for both contacts, then T07.19 and added#28, then T08.12.
- Subsumes lens-unification#8's peak-pressure half and strength#6's model half.

**Step 6. Generation.**
1. T03.13 RollingCorner (κ), then T03.14 and T05.2.
2. T05.17.1 (σ helpers) and T05.17.2 (one outline walker; gear-outline#0).
3. T05.14 / T03.15.1 (`Mesh` from rack data, removing `as_gear`), plus U7.
4. **U1 fix here.**
5. Pinion-cutter undercut and severing, then T05.17.3–4 / T03.15.2–4: collapse `BuiltMember`, `Cut` and `ToothOutline`, and make `check_cut` σ-generic.
6. T05.12 (RingMesh reduced to `tip_room`), T14.10, T03.15.5 (feature).

Subsumes lens-unification#2, ring#9, lens-architecture#8's member half, lens-unification#9 and #10.

**Step 7, independent track: screw.** T07.6, then T07.7, then T07.18. Keep the Σ = 0 dispatch.

### Not worth doing

| Candidate | Why not |
|---|---|
| General TCA for line and point contact | Boundary layer at Σ → 0; loses the ISO closed forms |
| One efficiency model | Search hot path; O(μ²) = 1e-4 |
| LP/LCP for the flow | BLT is exact and simpler |
| Trait objects for the contact enum | Two variants are enough |
| Merging rim-fit and tangent-angle data | They are transcribed standards |
| Merging `System` and the closure numerics | Rationals vs f64 |
| `MeshSide` / `Drive` / `Section` enums | Representational |
