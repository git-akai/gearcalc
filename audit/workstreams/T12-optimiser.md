## T12 — The shift optimiser and automatic values

**Why.** The optimiser (`Search::maximise` driven by `Shape::chosen_at`) takes a grid resolution as a feasibility test, takes a bare argmax over a surface whose ridges differ by less than the loss model can rank, and pays for both with a full `13^dof` grid. So it misreports or refuses in the cases a designer asks it about. On 9/37 at a given 24.28 mm the admissible window is 0.022 module wide, narrower than the 0.0314 grid step, and the search reports "no solution" and falls back to an inadmissible, interfering split whose η is 1.4e-4 *higher* [auto-search#0, lens-continuity#3]. The shipped Planocentric preset solves with the search off but refuses with it on, because `chosen_at` closes one round's shifts at the next round's plan [added2#94]. With the search switched on, a Layshaft is a 13^4 grid plus 80-direction walks: 46,251 objective calls and about 1.7 s native per solve, rerun on every input, and no gate times it [lens-performance#3]. Around these sit several smaller problems: an answer shifts 0.2 module for Δη = 2e-5 [auto-search#3]; the answer sits within 1.1e-5 of a wall, so the 4-decimal figure the panel shows is inadmissible [auto-search#5]; the floor rules disagree [auto-search#6, #8]; and a second, unshipped search is what the tests actually check [auto-search#7]. Several one-dimensional sub-problems (the division at a given distance, the root-round ceiling, the pointed roll) have closed or bracketed forms and are solved by search or by repeated generic solves instead.

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T12.1 Return shifts with the plan they were scored in | added2#94, added2#98, added3#20 (unverified) | high | M | — |
| T12.2 Close each component by its own outcome | shape-a#2, auto-search#1 | medium | M | — |
| T12.3 Search a held component's root member | shape-a#3 | medium | S | T12.2 |
| T12.4 A given ring is not judged as a candidate | added#8 | medium | S | — |
| T12.5 Port the pinned-sum box, then delete the unshipped search | auto-search#7 | medium | M | — |
| T12.6 One undercut floor, read once | auto-search#6, auto-search#8 | medium | M | T12.1 |
| T12.7 A violation residual: feasibility checked, the binding constraint named | auto-search#0, lens-continuity#3, added#44 | medium | L | T12.5 |
| T12.8 Answer one resolution step inside every wall | auto-search#5 | medium | S | T12.7 |
| T12.9 Convergence gate as a sweep; hula studies pinned by law | auto-search#4, ablate-constants-geometry#4 | medium | M | — |
| T12.10 Cost linear in dof, gated by call count | lens-performance#3, auto-search#2, added#42 | medium | L | T12.9 |
| T12.11 Exact solve for a one-dimensional division | lens-continuity#2 | low | M | T12.7 |
| T12.12 Near-equal ridges: a stated tolerance and a continuous tie-break | auto-search#3 | low | M | T12.7, T12.8 |
| T12.13 Score line meshes by loss, not by η (μ = 0) | added#39, auto-search#16 | low | S | — |
| T12.14 Root-round ceiling in closed form; fillet cap stated once | auto-search#14, lens-unification#9 | low | M | T03.2 |
| T12.15 One pointed-roll function | auto-search#13 | low | S | — |
| T12.16 Search and sizing literals given units and origins | auto-search#12 | low | S | T12.1 |
| T12.17 `admissible_profile_shift` doc states the thickness bound | auto-search#10 | low | S | — |
| T12.18 Interference is not "no contact" | added2#39 | low | M | — |

### T12.1 Return shifts with the plan they were scored in
**Change.** In `Shape::chosen_at` (shape.rs ~1865–1935) each round can re-size the tip-sized distance and replace `plan`/`bound_by`. If the next round finds nothing, or the three rounds run out, the post-loop `self.closed(&plan, &place(&v), helix)` pairs the old `v` with a plan no trial built. On Planocentric, round 1 chose [0.5235, 0.8874] at held Some(1.734), η 0.998; the re-size gives held None; the closure becomes [0.5235, 0.0], which is `OutsideInvoluteDomain`. Carry the best as a tuple (v, plan, bound_by, closed shifts, objective). After the loop, return the latest plan's closure only if `trial_efficiency` is Some there, otherwise return the round's own. Replace the silent `unwrap_or_else(|| settled.clone())` with that fallback. Do not always close at the found-in plan: that moves the hula table (d=2 α_w 33.0→33.4°). Keeping the re-sized closure where it is admissible and falling back where it is not keeps all 605 tests green.
**Proof.** New law beside `every_arrangement_of_a_set_solves`: for every `Preset::ALL` and every arrangement, with search on and with search on for each mesh alone, the train solves wherever search off solves, and the searched product of mesh η is ≥ the unsearched one. It fails today on Planocentric only.
**Notes.** This is also T10.8 [added3#19 (unverified)]. The rounds loop is where T12.6 needs "keep the best round, not the last" so the hula does not regress.

### T12.2 Close each component by its own outcome
**Change.** `closed()` divides a both-free sum equally only when `free.is_empty()`, which is a global switch. Once any component is searched, every non-asking mesh takes its first member at the floor and gives the rest of the sum to the other: on a layshaft (20,40)+(30,30)+(35,25) at 30.5 mm, searching mesh 1 lowers meshes 0 and 2 from 0.98569/0.98678 to 0.98501/0.98618 [shape-a#2]. In the multi-component branch (shape.rs:1882–1900), a component whose `maximise` returns None keeps its box midpoints and is reported `Chose` with no note. On the Layshaft preset at a given 31.156 mm with mesh 2 at ε_min 2.5 the result is 0.4250, the box midpoint [auto-search#1]. Make `closed` (and `sized`/`room_at`, which receive the same slice) take `free: &[Option<f64>]`, with Some only for members of components whose search found a point. Apply the divide rule per mesh wherever a member is Free and its entry is None. A failed component then gets exactly the whole-shape fallback (the even split [0.6403, 0.6403] above), and the global mode goes. Keep the part `Chose` (corrections.md:583 rejects part-level FoundNothing). Name the failed mesh with a mesh-level note: a new `Note` key, five catalogues.
**Proof.** Law over every preset × each mesh toggled, including a tip-sized distance: toggling search on mesh k leaves every shift outside k's component bit-identical. It fails today on the layshaft fixture. A second law on a single part with a held distance (the Layshaft preset at a given distance, one mesh made infeasible) checks two things: the other component's shifts equal what it chooses alone, and the failed one's equal the `closed(plan, &[])` split and carry the note.
**Notes.** Compute one mesh partition and derive asking/free/components from it. That also removes the second union-find (auto-search#15, shape-a#17 own the refactor).

### T12.3 Search a held component's root member
**Change.** On a given distance a member in more meshes than its mate (a planet) becomes `Role::Settled`, and `chosen_at` searches only `Role::Free`. So the search finds nothing to move and reports `NotAsked` even with every mesh asking. Make the root `Free` when its component asks, with an `Own` coordinate whose interval keeps both reaching mates admissible. Pass the matching `Decided` role to `undercut_bound`.
**Proof.** Planetary 12/30/72, distance = automatic + 0.1, ring automatic, search on: `how == Chose` and η ≥ 0.977972. That is the best of a hand sweep of the planet shift; today the result is 0.977792 and NotAsked.

### T12.4 A given ring is not judged as a candidate
**Change.** `trial_efficiency` (shape.rs:2217) maps every ring to `Cut::ByShaper`, which is judged by `clamps.is_empty()` before `plan.role` is read. The ring is `Given` in every set, so a candidate-independent clamp (`fillet_capped`, `cutter_teeth_reduced`) vetoes every candidate and raises a false "no solution". Test `plan.role[i] == Given` first. Carry a given ring unjudged (`judged: bool` on `ByShaper`, or a `Pinned` form over either tool). `tips_are_clear` must still see it as a ring. Rewrite `ring_is_cut_as_asked`'s "nothing to optimise" paragraph to match `Cut::Pinned`'s rationale.
**Proof.** For every preset and arrangement with a given ring, adding a candidate-independent ring clamp leaves the chosen shifts unchanged. For 20/20/60 with ring h_a 0.8 under cutter tip round 0.6 and under an 80-tooth cutter, the result must be Chose +0.0188/−0.0295; today it is FoundNothing.
**Notes.** Which ring notes are clamps at all is T05.11 [ring#6].

### T12.5 Port the pinned-sum box, then delete the unshipped search
**Change.** Production searches through `chosen_at`'s `Coordinate::{Own,Sum,Division}`, which re-implement `Pinned::place/box_of`. `shifts_for_efficiency`, `Bounds`, `Pinned`, `sides`, `member`, the free `maximise`, `Freedoms` and `Search::effort` are reached only by tests or by nothing. The one idea production lacks is `Pinned::box_of`'s intersection of a free member's interval with (sum − partner interval). Without it, 9/37 at 24.28–24.29 mm finds nothing where the dead copy finds [0.6535, 0.8284]. Steps, each green:
1. Port that intersection into `chosen_at`'s box for a Free member whose partner Reaches (shape.rs:1765–1781).
2. Retarget `the_search_beats_a_scan_of_the_same_interval` and the two auto.rs tests to `arr::pair(..).set_search(true)` with the distance fixed to imply each sum.
3. Add a unit test on `Search::maximise` with a 1-D box narrower than 0.096 whose optimum lies between grid points (added#47).
4. Delete the dead code and its prose (auto.rs:1336–1620). Fix the references at shape.rs:1790, gear-cli main.rs:1542, contact.rs:495, auto.rs:817/855, and reference.md:766/781. Move the true description onto `Search::maximise` (added#40).
**Proof.** The retargeted scan test fails with `place()` mutated to swap members, and fails with `.max(resolution)` removed from the walk's first step; both mutants pass the whole suite today. 9/37 at 24.28, 24.285 and 24.29 mm chooses.
**Notes.** This is also T17.3, which lists everything the deletion takes [ablate-features#0, added2#87].

### T12.6 One undercut floor, read once
**Change.** Three sites disagree:
- `shift_asked` floors at `x_min(h_w)`.
- `member_is_buildable` also demands `!tooth.undercut`, which is decided at h_f.
- `trial_efficiency` passes `gear.dedendum` to `undercut_bound` (shape.rs:2233).

The effects: with h_w < h_f, working depth only moves the grid anchor. With h_w > h_f the search goes below the asked floor unnoted (9/40 at h_w 1.5: floor 0.7598, chose 0.6629). With the search off, the settled shift builds an undercut tooth. Separately, choosers are floored at `max(x_min, 0)`, a non-negativity rule bundled into "no undercut". Its effects: the X-zero design x1 = −x2 is refused its own standard distance (17/43); a worm loses 0.49 points (objective 0.61805 against 0.62297 at x_min); and the answer depends on member order (added#45, which T10.5 closes once this floor is in place).

The fix:
- In `member_is_buildable`, keep only `x ≥ floor − compat::SAME_SHIFT`.
- Pass `working_depth.resolve(dedendum)` at shape.rs:2233 and in the `Absorbed` arm.
- Floor every member at `x_min`.
- If non-negativity is wanted, make it an explicit per-member option with its own note.

The recorded justification (the hula walks to −1.79 and loses efficiency) comes from the round alternation. Fix it with T12.1's "keep the best round". Rewrite the eight tests that encode the floor's refusals as geometry (added#46 lists two).
**Proof.** Chosen shift ≥ floor(h_w) for every member, paired or not, at h_w ∈ {0.8, 1.0, h_f, 1.5}; this fails today at 1.5. At z = 7..20 a pinion exactly at its floor with a non-interfering mate is admissible. Member-order symmetry at every given distance. Searched η at floor x_min ≥ at max(x_min, 0), checked against a brute force; the hula drive η must not fall (today it falls [19,18,17,18] 0.3089→0.2832 without T12.1).

### T12.7 A violation residual: feasibility checked, the binding constraint named
**Change.** `maximise` walks only from grid points the sweep admitted. When the admissible region is thinner than one step it returns None, and `chosen_at` reports FoundNothing with fallback shifts that are themselves inadmissible. Examples: 7/40 at root radius 0.10–0.18 (a brute force finds 20–1765 admissible points) and at clearance 0.08–0.09 (1560 points). The fallback `closed(plan, &[])` checks nothing: at 24.280 mm it interferes and reads 1.4e-4 better than the admissible answer 2 µm away. The shipped Planetary preset is always FoundNothing (ring pinned at 0, full depth, planet–ring flank interference at every sun shift), and the note never says why.

Target: `MeshTrial`/`CrossedTrial::efficiency` return `Result<f64, Violation>`, where `Violation` is the largest normalised shortfall among the constraints they already test (floor, ε_min, the flank relation σ_i(r_i − r_j,i), the bottom gap, the root round, tip foul), each with its key and mesh. The normalisation is stated: mm over module for gaps, raw for contact ratio. `maximise` takes `Result<f64, f64>`. If the sweep admits nothing, a feasibility walk minimises the violation from the least-violating points. FoundNothing only when the minimum violation stays > 0. The fallback is that least-violating point. The note carries the mesh, the constraint key and the shortfall.

Migration:
1. The trials return the residual beside None, and callers ignore it.
2. `maximise` gets the Result type.
3. Add the feasibility phase and the fallback.
4. The note gains its values (five catalogues).

Consider shipping Planetary with the short ring addendum that `a_shipped_sets_full_depth_ring_interferes_and_a_shorter_tooth_clears_it` already shows clears the interference.
**Proof.** A sweep law on 7/40, 9/37 and 17/43 (root radius 0.08–0.4, clearance 0–0.2, a given distance stepped to the edge of reach, ε_min): FoundNothing only where a brute grid at `resolution` finds nothing, and otherwise within 1e-5 of the brute best. It must fail today at 7/40 ρ 0.15 and clearance 0.08; genuinely empty cases (hula d=1, 3; 9/37 ε_min ≥ 1.36) stay FoundNothing. Across 24.270–24.290 mm, η never rises on entering the infeasible side and the division is continuous. Planetary as shipped: the note names mesh 1 and flank interference on member 0; with ring addendum 0.75, Chose.

### T12.8 Answer one resolution step inside every wall
**Change.** The walk accepts any better admissible point, so the answer lands on the wall to within the step. On 9/37 the flank-interference wall is 1.1e-5 away. The panel shows `toFixed(4)` and seeds a manual field from it (TrainPanel.svelte:1814/1849), so switching both shifts to manual gives (0.6746, 0.7332), which the tool then flags as interfering. Using T12.7's residual, admit a point only if the violation stays zero at ±`resolution` (1e-3) on each coordinate. Keep this in Rust. At the active constraint, report which one binds and its margin. The cost is ≤ about 1e-5 of η.
**Proof.** For every preset and the brute-force pairs, chosen shifts rounded to 3 or 4 decimals are admissible and solve with `flank_interference [false,false]`. This fails today on 9/37 (4 dp) and 7/40 (3 dp).

### T12.9 Convergence gate as a sweep; hula studies pinned by law
**Change.** `the_search_is_converged_not_budgeted` asserts 1e-5 against `refined(3)` on 14 default pairs only. Default 7/40 (1.07e-5) and 9/37 at clearance 0 (3.67e-5) already exceed it. The hula table in reference.md is labelled "Optimised"/"best" but records the shipped search's local optimum: at d=2 the stage is 58.392 % shipped and 59.850 % at refined(3), and efforts are not monotone (r4 58.901 %).

Make the gate a sweep: 7/40 plus clearance 0–0.2, helix 0–25° and tip width on three pairs, against a brute force whose resolution is stated (the verdict depends on it: 1.08e-5 against 1.34e-5 at 7/40). Then either fix the multistart (start from every local grid maximum, not the 2 best plus extremes) or write the measured bound, with size and sign, in `docs/state.md`. Pin the hula studies against the best of several efforts, or relabel them "as the shipped search finds". First find out why refined answers sit at exactly 0.3000 (a box edge?). Correct auto.rs:1643–1645 (T18.21 owns the stale figures [auto-search#9]).
**Proof.** The gate fails today on 7/40 and 9/37 at clearance 0. The hula law fails today at d=2 by 1.46 points of stage efficiency.

### T12.10 Cost linear in dof, gated by call count
**Change.** Measured calls per search: Spur 1,967; Idler 12,344; Compound 8,728; Layshaft 46,251 (28,561 of them the sweep); MeshedPlanets 45,850. Native time: Layshaft 1.7–2.3 s; the dof-5 line 3.6–5.9 s. At dof ≥ 3 most walks stop on `budget = 2000` unconverged (Layshaft 7 of 10, MeshedPlanets 8 of 9), contrary to its doc "a ceiling nothing reaches" (auto.rs:1687, corrections.md:561). Steps, each held to T12.9's gate and the corpus:
1. Replace the wall-clock `every_search_is_quick_enough_to_type_over` with a deterministic budget over every `Preset::ALL` with search on. Count objective calls and tooth builds (lens-tests-train#6). Add a converged-not-budgeted check at `budget × 100` for Idler, Layshaft and MeshedPlanets. `refined(3)` is not usable here: 69 M solves at dof 5, and it still hits its own budget.
2. Scale the budget with `directions.len()`, then replace the `3^dof − 1` stencil with the 2·dof axis directions plus the Sum/Division diagonals.
3. Replace the full `(scan+1)^dof` sweep with a fixed-size low-discrepancy sample, or, for chains of held distances, dynamic programming over the shared coordinate. Block ascent per mesh is not valid across a shared automatic distance.
**Proof.** Step 1 fails today: MeshedPlanets moves 1.52e-6 at budget × 100, and Layshaft exceeds any keystroke budget. A law: calls ≤ C·dof·budget.
**Notes.** Keeping a part's cut across load edits, so a torque edit never re-searches, is a separate change. lens-numerical-robustness#8 and lens-tests-train#12 cover the same gate.

### T12.11 Exact solve for a one-dimensional division
**Change.** At a given distance the pair has one free number, yet the pattern search finds it on a ~2^-9 lattice. x1(a) is a non-monotone staircase (24.272→24.278 mm: x1 falls 0.655902→0.653941 while a rises, and η falls 4.7e-6). When the search has a single `Division` coordinate, take the best of these candidates:
- the admissible interval's ends, each a bracketed root of its own residual term from T12.7, via `solve.rs`;
- the stationary points of the loss on each smooth piece.

Keep `maximise` for dof ≥ 2.
**Proof.** Against a 1e-6 dense scan: η ≥ scan max − 1e-9 at every a. x1(a) is continuous except where two optima are within T12.12's tolerance, and each such switch is reported.

### T12.12 Near-equal ridges: a stated tolerance and a continuous tie-break
**Change.** The argmax jumps 0.2–0.9 module between ridges with Δη 1e-6 to 4e-5. The dense brute force jumps at the same inputs (17/43 tip width 0.3125→0.325; 9/37 clearance 0.175→0.18), so the jumps are inherent. μ cannot reorder the ridges; the argument is the loss model's relative fidelity (2e-5 on a 1.5 % loss is 0.13 %). Add a user-visible "efficiency tolerance" option. Among admissible points within it, pick by a continuous secondary criterion with one optimum on each ridge: the largest minimum margin from T12.8's residual. Report the near-equal alternatives. No selection rule makes an argmax over a disconnected set continuous, and hysteresis would store an output (rule 3), so jumps stay and are reported.
**Proof.** On 9/37, 17/43 and 7/40 sweeps of clearance, helix, tip width and root radius: between steps, the shifts move ≤ K·Δinput with K stated, and every exception coincides with a reported change in the tolerance set's components. η stays within the tolerance of a brute force.

### T12.13 Score line meshes by loss, not by η (μ = 0)
**Change.** At μ = 0 every candidate scores exactly 1. The answer is then the stable sort's first start, reported `Chose`: 9/37 gives [0.5085, 0.0673] against [0.6746, 0.7332] at any μ > 0, and MeshedPlanets moves 3.06 module. Line-contact η is affine in μ (contact.rs:436). Where every scored mesh has μ = 0, score −Σ L_k with L_k = 1 − η at unit friction. This is exactly the μ→0⁺ answer, and it also removes the rounding near 1.0 that already moves 17/43 by 5.5e-4 at μ = 1e-9. Document in `MeshTrial` that the argmax is μ-free only for a single line-contact mesh. Do not cache shifts across μ edits: multi-mesh and crossed components move by up to 0.047 module.
**Proof.** For pairs and presets with search on, shifts at μ = 0 equal shifts at μ = 1e-6 within `resolution`; this fails today. For single line meshes, shifts at μ = 0.01 and 0.3 agree within `resolution` (not bit-identical).

### T12.14 Root-round ceiling in closed form; fillet cap stated once
**Change.** `searchable_shift` brents about 11 full `admissible_ranges` calls to invert a two-piece closed form. Below the shallow cut, w_tip and ρ_fit do not depend on x, so x_c = h_f − ρ/(0.95 cos β); above it, b_d = 0.05 m and ρ_fit falls linearly. The fillet-fit cap and the reference thickness are hand-copied between tooth.rs, auto.rs and ring.rs, and one copy already drifts: `ranges_at_shift` uses the uncapped b_d, so for z 2–3 its root-radius bound is too tight (z3 x0 d2.0: 0.0780 against the tooth's 0.3989). Take the cap from T03.2's `Rack::settle` (its ρ_fit), which `Rack::wanted_by` and `ranges_at_shift` both call, and write `root_round_shift_ceiling(p)` from the same terms. Correct `searchable_shift`'s doc ("both terms linear").
**Proof.** `admissible_ranges().root_radius.max` equals the bisected `clamp.fillet_capped` threshold over the `tests/common` grid, including z 1–3 and out-of-range dedenda; this fails today at z 2–3. `root_radius.max` evaluated at the closed-form ceiling equals ρ, to 1e-12.
**Notes.** Needs T03.2, which owns the other w_tip copies [lens-numerical-robustness#7, added#13].

### T12.15 One pointed-roll function
**Change.** u − atan u = ψ_b is solved by `newton_bracketed` in `Tooth::new` and by `brent` in `addendum_for_tip_width`. `inv_inverse` gives the same to 4e-16, and ring.rs:297 already uses it (its comment wrongly says Tooth does). Add `pointed_roll(ψ_b) = inv_inverse(ψ_b).map(tan)` and use it at all three sites. `addendum_asked` should read r, r_b and ψ_b through `transverse_thickness` (keeping its clamps and the pressure-angle guard) instead of building a Tooth: 793 of the 1,563 builds in a 9/37 search, a 2–4 % saving.
**Proof.** Tooth's pointed radius equals r_b·hypot(1, pointed_roll(ψ_b)) to 1e-12 over z, x, α and β, including the largest ψ_b the guards admit (`POINTED_TOOTH_MAX_ROLL` against inv_inverse's π/2 bracket).
**Notes.** added#5 (tip width not monotone) and added2#102 (reference.md says Newton) touch the same function.

### T12.16 Search and sizing literals given units and origins
**Change.**
- The "moved" test at shape.rs:1914 compares mm against `resolution·1e-3` (modules): scale it by m_n.
- When the three-round cap is hit while the distance is still moving, raise a debug assertion or a note instead of stopping silently.
- shape.rs:1700's `e + 1e-9` "by the solver's own tolerance" is false, since brent's tolerance is ~1e-14 mm: step with `f64::next_up` until room ≥ 0 and fix the comment.
- The `scan`/`starts` field docs should cite the gate that sets them.

**Proof.** `cargo nextest run`; the corpus is unchanged or each diff is explained.
**Notes.** lens-magic-numbers#14/#15 overlap. The `SAME_SHIFT` part is in T12.6.

### T12.17 `admissible_profile_shift` doc states the thickness bound
**Change.** The doc (auto.rs:258–279) and the `ShiftRange::bound` doc say the upper bound is the cutter depth (1.20). The code's bound is thickness, 0.95π m_t (1.942 at 20°); depth is the shallow-cut threshold. Rewrite both docs and re-record the bracket: between 1.20 and 1.942 the gear builds with a dedendum note; above 1.942 the thickness is capped.
**Proof.** A unit assertion: `admissible_profile_shift(z=17, α 20°).bound.max = 1.9421` for h_f 1.0 and 1.25, with `shallow_cut` = h_f − 0.05.

### T12.18 Interference is not "no contact"
**Change.** For small pinions, a contiguous band of mates fails with `NoContact`, "the teeth never contact" (z1=2: z2 5..36; z1=3: 7..98; z1=4: 9). Each is tip-into-root interference: `contact_stress` gets a negative curvature radius (mesh.rs:634) and returns None, and shape.rs:3322 maps that to NoContact. Per rule 5, rate the contact over the part of the path outside the interference points and raise the mesh's interference note. Refuse only when `ContactPath::new` finds no path.
**Proof.** A law: NoContact arises only from `ContactPath::new`. The z1 2–4 × z2 3–400 sweep solves, with interference noted, across today's failing band.

### Declined
None.
