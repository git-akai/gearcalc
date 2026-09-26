## T10 — Train graph solve: closure, relief, assembly and the per-member figures

**Why.** The graph solve decides several things locally that only hold globally. Examples: a backlash band put together mesh by mesh, a triangle of distances never closed, relief counted group by group, plan roles assigned in list order, the assembly rule pooled per axis, and a member's reversal taken from how many meshes it has. Each local rule agrees with the global answer on the simple presets and breaks as soon as independent pieces interact. The shipped MeshedPlanets preset reports a minimum path play of 0.046° where the tolerance box reaches 0 (the true band is three times wider). One more ring tooth (`meshed_planets(24,[18,18],97,3)`) gives a set whose three axes cannot be placed at all, and it still solves with a ratio and clear layouts. Typing one box on a fully pinned preset breaks the solve in 18 of 122 cases, and reversing the order of relief's groups changes which inputs stay given in 173 of 322 cases. The cure throughout is to state each rule over the whole it governs: per distance, per carrier, per mesh group, per case. Then add the law that permuting what should not matter (distances, meshes, members, groups) changes nothing.

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T10.1 Path backlash band folded per distance | train-mod-a#0, mesh-contact#0, train-mod-b#1, lens-feature-gaps#0, added2#12 | high | S | — |
| T10.2 Refuse carried axes that cannot be placed (triangle closure) | shape-a#0, shape-b#0 | high | M | — |
| T10.3 One size-reading entry per mesh group; one helix per group | train-mod-a#3, shape-a#9 | high | M | — |
| T10.4 Thickness coefficient: one freedom per mesh group | added2#55, shape-b#9, train-mod-a#4, added2#18, shape-a#14, added3#2 (unverified) | medium | S | — |
| T10.5 `plan_held` as one constraint-graph pass; order-invariance laws | shape-a#1, added#45 | medium | M | T12.6 |
| T10.6 One-mesh absorber in closed form, clamp not refuse | added2#17, shape-a#12 | medium | S | — |
| T10.7 An automatic clearance is one derived gap per distance | shape-a#4 | medium | S | — |
| T10.8 `chosen_at` closes the point at a plan that was evaluated | added3#19 (unverified) | medium | S | — |
| T10.9 Relief as one global matching over the solve's own relations | train-mod-a#2, added2#11, train-mod-a#9 | medium | L | T10.3, T10.4, T10.5, T10.7 |
| T10.10 One tip-room rule for sizing and report | shape-a#5, lens-continuity#4, added2#117, train-mod-a#10 | medium | M | — |
| T10.11 Size readings round-trip: hand, domain, overlap figure | train-mod-a#5, train-mod-a#6, train-mod-a#7, shape-a#10 | medium | M | T10.3 |
| T10.12 Minimum face width inverted mesh by mesh | shape-b#6, train-mod-a#1 | medium | S | — |
| T10.13 Per-case flank map from the flow: reversal and cycles | train-mod-b#4, shape-b#4, shape-b#5 | medium | M | — |
| T10.14 Exact tooth-cycle counts; intermittent planet law | train-mod-b#2, lens-tests-train#0 | medium | S | T10.13 |
| T10.15 Worm wheel torque is the delivered torque | shape-b#10 | medium | S | — |
| T10.16 Report bottom clearance on every mesh | added#60 | medium | S | — |
| T10.17 Stated floor on an automatic face width | added2#7, train-mod-b#14 | low | M | T10.12 |
| T10.18 Assembly as one congruence rule per carrier | shape-b#3, shape-b#2, added2#2, added3#0 (unverified) | low | M | — |
| T10.19 A Wolfrom preset that assembles; generic spacing note | shape-b#1, added2#3, added3#1 (unverified) | medium | S | T10.18 |
| T10.20 `LoadRole` doc: a declared Free is the lock question | train-mod-b#9 | low | S | — |

### T10.1 Path backlash band folded per distance
**Change.** In `train/mod.rs` `backlash_at`, compute each mesh's `coefficient(k, read, from)` once rather than once per band point, which is 3× the play solves today. Map each mesh in `row_play`'s flattened order to its global distance (part index plus the part-local `Shape::distance_of`). For each distance d form S_d(t) = Σ_{k on d} c_k·row_play_k[t] for t ∈ {−, 0, +}. Then report nominal = Σ_d S_d(0), min = Σ_d min_t S_d(t) and max = Σ_d max_t S_d(t). Meshes on one distance stay correlated, which keeps a set's stationary sun/ring nominal. Independent distances now stack to their worst case. Add `Backlash::of_ends([lo, mid, hi])` and rebuild `banded` on it. Delete the `banded(0.0, 1.0, 1.0, |t| …)` index trick. In `shape.rs` write `row_play = [-tol_minus, 0, tol_plus].map(|dx| play_of(k, running + dx))` and delete `at_band`. Rewrite the "every distance at the same end" sentences at `shape.rs:3543`, `docs/reference.md:2569-2571` and `docs/corrections.md:398`.
**Proof.** Add a law over every preset, every arrangement and chained preset pairs. At every one of the 3^D corners, set `tolerance_plus[d] = s_d·tol` and `tolerance_minus = 0` so the solve's plus point is that corner, then re-solve. The band must contain the corner's nominal play, and each end must be reached by some corner. A second law: widening any tolerance never narrows the band. Fixtures that fail today:
- MeshedPlanets at its defaults: [0.000000, 0.139563]°, reported today as [0.046162, 0.093401].
- 17/43 plus 20/ring-60 at ±0.05: [−0.061296, 0.141740]°, reported today as [0.003781, 0.076663].
- Spur+Planocentric at ±0.02: [0, 0.197206]°.

Planetary must stay at 0.041724° at every tolerance.
**Notes.** Once the band is a sum of independent sources, a tooth-thickness deviation can join it as one more source (T21.3 [lens-feature-gaps#3]). The tight end on a point contact clamping at zero is T06.3 [added#59].

### T10.2 Refuse carried axes that cannot be placed
**Change.** After the distances close (in `rate`'s layout pass or right after `closed`), handle every distance joining two axes carried by one carrier. Compute cos φ = (r₁² + r₂² − d₁₂²)/(2 r₁ r₂) from the running distances. Refuse |cos φ| > 1 with a named `TrainError`, for example `AxesCannotBePlaced { distance }`, and add its key to all five catalogues (rule 5: no shape). Refuse a longer cycle of distances in one frame on its closure residual. Report φ in `LayoutReport` as the stagger angle. With the positions known, extend the layout check to the tip gaps between planets on different axes (P2ⱼ against P1ⱼ₊₁, and P2 against the sun), which are unchecked today. Delete the residual at `docs/state.md:636` and the sentence at `:1194`.
**Proof.** A grid law over `meshed_planets` and `ravigneaux` sun and ring counts: every Ok result satisfies |r₁−r₂| ≤ d₁₂ ≤ r₁+r₂ on its running distances, and every breach is refused. Placing the axes at the reported φ must reproduce d₁₂. Negative fixtures that solve today: `meshed_planets(24,[18,18],97,3)` (cos φ = 1.0097) and `ravigneaux([18,64],[22,18],62,3)`. `meshed_planets(24,[18,18],96,3)` must still solve at φ = 2.94°.
**Notes.** The shipped MeshedPlanets is exactly collinear at nominal (24+36+36 = 96) and closes only through its clearances, so it is one tooth from failing. Consider a preset with margin. The long-term shape makes φ the parameter and derives d₁₂ from it, which turns the check into φ's admissible range.

### T10.3 One size-reading entry per mesh group; one helix per group
**Change.** In `Shape::freedoms` (`shape.rs:4048-4081`), push `entry(readings_for(k))` once per mesh group with `given_at_most 1`. Today it is pushed only for the first mesh on a distance (`n == 0`). A later mesh group on a shared distance (Layshaft's pairs 2–3 and 4–5, Compound's second pair) currently has no entry. List that group's size as a column of its own relation, and delete the false comment at `shape.rs:4066-4071`. In `helix_angles` (`shape.rs:821-875`), skip sizing a mesh when any member of its group already has a helix, not only its own two. If a second pinned-distance mesh in the group asks a different helix, refuse with a named error (for example `SizeOverConstrained { meshes }`) and not `Mesh(Incompatible)`.
**Proof.** Law: after `relieved`, at most one Helix/PitchDiameter/Overlap reading stands per mesh group, over every preset and arrangement. Fixtures that fail today:
- Layshaft and Compound: type member 2's pitch diameter, then Helix = 10. The solve must come to 10°; today it comes to 15° with no note.
- Layshaft and Compound: type Helix, then the mate's pitch diameter. This must solve; today it gives "cannot mesh".
- `line(&[20,30,25,35]).size_free()` with distances 0 and 2 given must give the named refusal, not `Incompatible`. That relief then repairs it is T10.9's law.

### T10.4 Thickness coefficient: one freedom per mesh group
**Change.** In `Shape::freedoms`, replace the per-mesh ThicknessMod groups (`shape.rs:4088-4097`) with one group per `mesh_groups()` component, exactly as module is declared: `given_at_most 1`, order = the component's members, `just` spared. In `thickness_mods` (`shape.rs:1047-1077`), propagate by BFS from the single given member instead of mesh-list order. A component whose mesh graph has an odd cycle of external meshes forces k = 1, so there a given k ≠ 1 must raise a note or a refusal. Correct `Member::thickness_mod`'s doc (`shape.rs:172-177`), `docs/rationale.md:69-72` and `docs/state.md:764-770`.
**Proof.** Law over every preset, every arrangement, and a line and triangle fixture with random given k's: after relief every mesh holds its rule to 1e-12 (k_a + k_b = 2 external, k_a = k_b internal), and permuting `shape.meshes` leaves `thickness_mods` unchanged. Fixtures that fail today:
- `planetary(12,30,72,3)` with the ring's k at 1.1 gives ks [1.0, 1.0, 1.1], or [1.0, 1.1, 1.1] with the meshes swapped. The planet shift is −0.0628 in one order and −0.2786 in the other.
- Idler with k₀ = 1.1 and k₂ = 0.9 leaves mesh (1,2) summing to 1.8.

**Notes.** This removes the illegitimate fixed points behind most of added2#11's ThicknessMod differences. T16.29 can then remove a test that cannot fail [lens-tests-train#8].

### T10.5 `plan_held` as one constraint-graph pass
**Change.** Replace the two loops of `plan_held` (`shape.rs:1255-1296`) with one pass per held distance. The unknowns are the free members, and each mesh is an edge x_a + sign·x_b = S_m. In each component the root is the member in the most meshes (ties go to the lowest index). BFS gives every other member `Reaches(edge)`, and roles are never overwritten. An edge between two already-decided members becomes a closure check recorded in the `Plan` and verified by `closed()`. Give every member a search moves, directly or through a sum, the same undercut floor, taken once from `pair::undercut_bound`. Today `Reaches` maps to x_min and `Free` to max(x_min, 0) (`shape.rs:2221-2235`), so which member may go negative depends on list order.
**Proof.** Law: permuting `shape.meshes`, and swapping a/b within a mesh where the kind allows, leaves every shift, running distance and note unchanged. Compare shifts, not only notes. Sweep every preset plus the fixture p(21), r(22) on axis A, q(30) on B, with p–q and r–q on distance 0 held at 26.2 and r–s(25) on distance 1. Today the order [p–q, r–q] gives `distance_not_reached` and −0.0578 mm clearance, and the four orderings give three designs. Pair law: `gear-cli shifts 17 43` and `43 17` at a given 30.02 mm give mirrored shifts. Today one is reached and the other refused.
**Notes.** Which floor applies is T12.6's decision (x_min, with non-negativity an explicit option); both tasks rewrite shape.rs:2221-2235, so T12.6 lands first and this task applies its floor to every member in one place.

### T10.6 One-mesh absorber in closed form
**Change.** In `Shape::absorb` (`shape.rs:1416-1498`), an absorber with coefficient 0 in either of the two meshes leaves the other mesh's running distance fixed. Its shift then follows from `shift_sum_reaching` plus one linear step. Share that step with `closed()`'s reaches branch (`shape.rs:1350-1390`) as one helper. Apply no admissible-range cap: an out-of-range shift goes through the member builder's clamp and note, as a reaching shift already does. Keep the bracketed Newton only for a member in both meshes, bracketed by the involute domain alone, and delete the ±5 fallback once both bounds are shown finite (ablate-constants-rating#2 calls it dead). When `absorb` fails, the refusal names the member whose shift was sought, not "these tooth counts cannot be assembled".
**Proof.** The closed form matches today's Newton to ≤ 1e-12 on every preset (measured 4.4e-15), and `check_golden` does not change. Sweep `planetary(12,30,72,3)` with sun and planet given at settled+dx and the ring absorbing. Today this refuses from dx = 0.65. It must solve with a clamp note, with the two meshes' running distances equal to 1e-9 until the involute domain ends.
**Notes.** T02.7 covers the same misleading refusal text where the shifts merely disagree [added3#3 (unverified)].

### T10.7 An automatic clearance is one derived gap per distance
**Change.** Add `clearance_used(d)`. Under auto it returns the derived c = running − nominal₀, which every later mesh on the distance reaches through `kind.nominal_of(running, c)`. Otherwise it returns the manual value. Use it at `shape.rs:841, 1206, 1354, 2855`, so `clearance.manual` is never read while auto is set. Do not use the zero-clearance absorb (equal nominals): it interferes on the ring mesh.
**Proof.** Law over every preset: changing `clearance.manual` while auto is set changes no output bit, and every mesh on a distance reports the same signed gap. It fails today on `planetary(12,30,72,3)` with the distance at 21.195279: the hidden 0.02 gives the ring mesh −0.089° backlash and `clearance_negative`, while the field shows +0.070.
**Notes.** `docs/reference.md:420-426` still says a set's clearance is always given (T18.9 [added2#14]). Rewrite it to state this rule.

### T10.8 `chosen_at` closes at a plan that was evaluated (unverified)
**Change.** Lands as T12.1 in Phase 1, which carries the change and its law. `the_four_hula_studies_are_the_ones_this_code_prints` must stay green there. T10.5, which rewrites the plan later, is held to the same law.
**Proof.** T12.1's law.

### T10.9 Relief as one global matching over the solve's own relations
**Target shape.** Relief asks one question: can every relation the solve closes be matched to a distinct automatic input it actually uses?
- Rows: one per mesh (distance closure), one per distance (distance and clearance), and one per mesh group (size).
- Columns: only what the solve reads for that row. That is D, C and the mesh's own two shifts, plus the group's size only where the solve sizes it (both shifts pinned, nothing stated). This is best derived from the plan's roles (given, free, reaches, absorbs) so that relief and solve cannot diverge.
- A state is admissible when a maximum bipartite matching (Kuhn is enough at these sizes) covers every row.
- Relief: while the matching is short, turn automatic the least-precious given input that lengthens it, under one declared total precedence across groups with `just` last. Never relieve an input whose restoration still matches.
- The loop ends because the matching grows strictly at each step. Keep the genuine count rules (module, pressure angle, `always_automatic`) as counts.

**Migration.**
1. Land T10.3 and T10.4, which remove the fixed points that are illegitimate today.
2. Add the matching as a pure checker plus a test that records which of today's relieved states it rejects. It must find exactly probe -d's failures, among them Planetary with D typed (x_s the only automatic shift) and MeshedPlanets with D1 typed.
3. Switch `relieved()` to the matching and add the laws below in the same commit. The default presets give nothing, so `check_golden` and `check_wasm` do not move.
4. Delete the per-mesh geometric FreedomGroups, the stale planetary second-pass comment (`mod.rs:2412`, `2422-2426`) and the heading "Settled, whatever order the groups come in".

**Proof.** Laws:
- Start from every preset fully pinned. For every freedom f, nudge f, relieve with `just = f` and solve. The typed value must equal the solved figure to 1e-6, with no `distance_not_reached` or `clearance_negative`. This fails in 18 of 122 cases today.
- Every permutation of the relations gives the same toggles. Reversing the groups changes the given set in 173 of 322 cases today.
- No given input is relieved when restoring it still matches. Today the declared order over-relieves in 200 of 237 differing cases, for example Planetary keeps 10 given where 12 are admissible.
- `line(&[20,30,25,35]).size_free()` with two pinned distances relieves to a solvable state (shape-a#9's relief half).

### T10.10 One tip-room rule for sizing and report
**Change.** Replace `tip_room`'s `min(far_gap − c, tip_margin)` (`shape.rs:1538-1546`) and `TipRoom::at`'s verdict (`mod.rs:579-589`) with one admissibility helper that both the sizing and the report call:
- Room = min(far_gap − c, crossing gap − c), with the crossing measured in mm between the tip corners (or c converted at r_a1), so the asked clearance governs both. Today the crossing is held to exactly 0.
- The verdict is `crossing < 0 || far_gap < c`. Today the far gap never enters it.
- Read the room at running − tolerance_minus, as T05.3 proposes [added#66]. Land it in the same change so the helper is written once.
- Make `Distance::tip_clearance` an `Option<f64>`, with None meaning "not asked", to remove the hidden mode switch at exactly 0. Rewrite its doc (`shape.rs:274-280`).
- Report which condition bound the distance.
- When the tips replace a typed clearance, raise a note giving the asked and the reached value.

**Proof.**
- `gear-cli hula 18 c`: today every c ≤ 0.26 gives the same offset (0.746026 mm) with the tip margin at +0.0000°. After the change, at every c > 0 the built far gap is ≥ c, the crossing gap is ≥ c, and the outputs change strictly with c.
- For Some(gap), the distance is continuous in gap on [0, 0.5]. Today planocentric 20/28 steps from 3.980 to 3.971973 at 0⁺, and 12/13 is refused at 0 but solves at 1e-12.
- A stated planocentric 30/32 at 0.96 must report tip interference. Its far gap is −0.0141 mm and it is reported clear today.
- Planocentric with clearance 0.03 typed gives the note (0.03 asked, 0.02 reached). The relief law's `tips_hold` skip becomes "honoured or noted", and the test comment's "floor" becomes "ceiling" (added2#10).

### T10.11 Size readings round-trip
**Change.**
- Pitch diameter: carry the hand, β = sign·acos(z m_n/d), where sign is the member's current hand (+1 when 0). Refuse d ≤ z m_n on parallel meshes as crossed meshes already do, and refuse d ≤ 0 under its own key rather than `OutsideInvoluteDomain`. Code: `shape.rs:3991-3997`.
- Overlap: make `Figure(Overlap(k))` the minimum ε_β over the group's meshes, which is what the reading enforces. The panel shows that core figure instead of `reports[meshes[0]]` (`TrainPanel.svelte:1937`), which takes an engineering choice out of TypeScript (rule 1).
- In `helix_for_overlap` (`mod.rs:2150`), make the interval half-open (sin β < 1). Past the limit, refuse with `overlap_unreachable` as an error instead of falling back to the automatic 0°.

**Proof.** One law: for every group with every width given, pinning any reading (helix, pitch diameter, overlap) at the figure the core reports leaves `helix_angles()` unchanged to 1e-4 relative, sign included. Fixtures that fail today: pair 17/43 at −15° pinned by its diameter comes back at +15.00006°, and a planetary with a 6 mm ring moves from 18.31° to 31.574°. A sweep of ε_β across b/(π m_n) must be monotone or refused. Today it goes 87.47° at 3.18, then an unrelated base-circle error at 10/π, then 0° at 3.2.
**Notes.** Nothing bounds a near-90° helix from any reading. A bound would have to be a stated, user-visible maximum. T14.6's normalisation of a helix stated three ways with last-wins [shape-a#13] is what this law guards.

### T10.12 Minimum face width inverted mesh by mesh
**Change.** In `MemberRating::rated` (`mod.rs:966-1010`), accumulate need_F = max_k(σ_F,k·b_k) and need_H = max_k(σ_H,k²·b_k) over the meshes that size the face, and divide once by the allowables. Drop the shared `width` accumulator. Document that the figure holds while it is at or below every mate's width.
**Proof.** Law: `rated().min_face_width` equals the probe-pass `asks()` value at any widths. Fixtures on `planetary(12,30,72,3)`:
- Ring typed at 2 mm: bending must be 0.4513 mm (1.6347 today) and contact 4.4766 mm (5.1185 today).
- Planet width 8 → 24 mm: bending must stay 1.2398 mm and contact 6.757 mm. Today bending grows from 1.2398 to 3.7195.

**Notes.** The error is always an overstatement, but it is still a wrong figure under rule 6.

### T10.13 Per-case flank map from the flow
**Change.** In `rate`, compute per case and per member the sign of the torque each non-idle mesh applies about the member's own axis. The per-mesh member torques are already computed near `mod.rs:4561`. From that sign:
- Reversal: a member reverses in a case iff two of its non-idle meshes load opposite flanks, or the duty reverses. This replaces `always_reverses = meshes_of[i].len() > 1` (`shape.rs:3373`).
- Cycles: per flank, sum the paths of the meshes loading it, then take the max over the two flanks. This replaces `paths_seen`'s max over meshes (`wiring.rs:234`).
- Under a reversing duty, halve contact only where the member's meshes do not already load both flanks.

Pass the per-member flag in `CaseLoad`. Rewrite the Reversal doc (`mod.rs:2590`, "a planet always does"), the `paths_seen` comment and `docs/state.md:551` to state the flank rule.
**Proof.** Laws:
- Idlers and planets reverse in every case.
- A pinion driving two gears and a gear driven by two pinions never reverse. Today the dual-driven gear carries `reversed_bending_uncorrected`, and with the switch on its fatigue bending width goes from 0.754 to 1.077 mm.
- The dual-driven gear counts 2× its revolutions: about 139, against 70 today.
- Under a reversing duty a planet's contact count equals its bending count: 30, against 15 today. The sun's halving stays.
- An idle mesh (the layshaft's unselected ratio) carries no sign.

**Notes.** Whether the 0.7 reversed fraction should apply at all is T08.4 [strength#0, ablate-constants-rating#0, lens-feature-gaps#2]. This task decides only which member it applies to.

### T10.14 Exact tooth-cycle counts; intermittent planet law
**Change.** In `loaded_cycles`, form the count from the exact `Ratio` speeds (`motion_in` returns them) as ceil((p·range·actuations·paths)/(q·360)). The numerator and denominator are integer-valued f64s, exact below 2^53, so ceil needs no epsilon snap. Use it in both branches.
**Proof.** Property test against `Fraction` arithmetic over random ratios, ranges k·360/n and actuation counts. It fails today:
- `planetary 12/16/44` sun gives 111, exact 110; reversing gives 120, exact 110.
- An emulation over 33,600 combinations gives 606 mismatches, for example 3000 against 2000 at 12/32/3, 90° × 1000, reversing.

Also run the existing law `an_epicyclic_members_cycles_are_its_turns_against_the_carrier` under both `Continuous` and `Intermittent` duties, taking the expectation from the kinematics rather than from `speed_against_carrier`. With that law in place, the `.abs()` mutation at `mod.rs:4531` must fail it ("401 against 2400"). Today the suite and the corpus both pass that mutation.
**Notes.** No rating reads the cycles yet (T21.6 [lens-standards#4]).

### T10.15 Worm wheel torque is the delivered torque
**Change.** In `member_torque` (`shape.rs:3721-3736`), give the driven side of a point mesh the read-across torque × the mesh's η in the case's direction. That is the flow's delivered torque. When the mesh locks (η = 0), keep the driver's pressing torque. Share the z_b/z_a conversion with `point_contact` in one helper, and correct the "reference cylinder" comment for point meshes.
**Proof.** Law on every crossed or worm mesh: the driven `GearCase.torque` equals driver × ratio × η, and equals `on_body` when the body has no other mesh. Fixture `worm(1,40)` at 20 N·m: 494.44 N·m, against 800 today (η = 0.618).

### T10.16 Report bottom clearance on every mesh
**Change.** Add `bottom_clearance: [f64; 2]` to `MeshReport`, computed at the running distance. A line contact uses `operating.bottom_clearance`. A point contact uses the centre − r_a − r_f expression already in `CrossedTrial` (`auto.rs:1194-1199`), shared as one function per kind. Include it in `teeth_clear`, add the note `mesh.tips_bottom_out` (member and depth) to all five catalogues, and run `check_bindings --write`. The owner may instead choose tip shortening as a recorded clamp with its own note (rule 5). Adding the field without changing `teeth_clear` would not fix the finding.
**Proof.** Fixture: 20/20 at x 1/1 reports −0.1214 mm, `teeth_clear` false and the note. Today it reports clear with no note. Law: every preset reports a non-negative gap, unchanged. The strings test fires the new note.

### T10.17 Stated floor on an automatic face width
**Change.** Add a member input b_min/m_n with its default computed in Rust (rule 1). Set the width to max(floor, ask, overlap floor) (`shape.rs:3461-3473`) and raise a note when the floor governs. Unsolved cases ask None upstream. `FaceSources::width_for` (`mod.rs:2553`) returns the box value only when no rating asked, which removes the `want == 0.0` jump. Correct `docs/state.md:746`, which calls a faceless gear unreachable.
**Proof.** Law: for every preset with automatic widths, the width is non-decreasing and continuous in torque through T = 0, never below the floor, and the note fires when the floor governs. Fixture pair 17/43: today 1e-12 N·m gives 4.48e-13 mm, and T = 0 jumps to 10 mm.

### T10.18 Assembly as one congruence rule per carrier
**Change.** Replace `Shape::assembly` (`shape.rs:640-714`) with a per-carrier rule. There is one unknown per planet body, A[e,P] is the signed tooth count of body P's gear on mesh e, and b_e = z_c/N for a central mesh or 0 for a planet–planet mesh. Equal spacing holds iff y·b ∈ ℤ for every integer y with yᵀA = 0, decided exactly by i64 Hermite or Smith reduction. Simultaneous meshing is N | z_c over the central members. This removes the per-axis pooling across bodies, the `pairs.len() < 2` None and the carried-central None. Replace the test `a_planet_meshing_a_planet_has_no_answer`, and delete the residual at `docs/state.md:754`.
**Proof.** Check against a phase search generalised to several bodies. It must share no algebra with the rule; `assembly.py`'s brute force shares A and b, so it does not qualify. Closed forms to reproduce:
- simple set: (z_s + z_r)/N
- stepped planet: the gcd rule
- meshed planets: (z_r − z_s)/N
- Ravigneaux: (z_s1 + z_r)/N and (z_r − z_s2)/N

Fixtures:
- Compound with its second planet moved to its own body at 18 teeth: (true, false). Today it gives (false, false).
- Sun-only planet at N = 3 with z_s 12 or 13: (true, true) and (true, false). Today both give None.
- Meshed planets with sun 25 and ring 96: not evenly spaced. Today it gives None.

### T10.19 A Wolfrom preset that assembles; generic spacing note
**Change.** Ship `wolfrom(18,[60,62],2)`, which solves, assembles and gives ratio 31, or a stepped-planet Wolfrom. Do not ship [60,63] at N = 3: the solve refuses it with NoCommonDistance. Reword `part.planets_not_evenly_spaced` in all five catalogues without "sun" or "sum". Add a "only one planet station exists" note where T10.18's congruence admits only φ = 0, as for today's 18/[60,61]/3. Update the golden corpus, `gear-cli` `kinematics.rs:202`, the tests at `arrangements.rs:1134` and `shape.rs:4375, 5832`, and `tools/train_kinematics.py:658` together.
**Proof.** Law: no `Preset::ALL` default raises `planets_not_evenly_spaced`. It fails today on Wolfrom.

### T10.20 `LoadRole` doc: a declared Free is the lock question
**Change.** Rewrite `LoadRole`'s doc (`mod.rs:2839-2842`). A declared Free counts as the reaction it declines to be; it is not a restatement of the default. This matches `relieve_case_toggles` (`mod.rs:3690-3697`) and `reference.md:2764`, which are deliberate and sound. Optionally, stop relief from turning the last given torque derived when no reaction is declared, so the solve asks the lock question and does not report `case_nothing_drives`.
**Proof.** Doc only. For the optional part: a pair given 0.1 N·m at body 1 with nothing at body 2 reports `load_not_reacted` after relief.
