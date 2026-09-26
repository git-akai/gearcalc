# Surviving mutants: analysis

Scope: the 181 `MissedMutant` entries of `mutants_all.json`, from the gear-core cargo-mutants runs (two library runs and one train run) at HEAD 857d578; every diff's original line matches HEAD. The json keys a mutant by (file, line, replacement), which merges three pairs of distinct mutants: gear.rs:502 (three `+ → *` columns) and auto.rs:357 (`+ → *` and `/ → *`). Counted by column there are **184 survivors**, and all 184 are tabled below. Two more lines in the crash-backup `missed.txt` (strength.rs:1065, gear.rs:420) were caught in the later run and are left out. The classification comes from reading each site and its callers. Four doubtful solve.rs cases were settled with a Python replica of `brent`/`newton_bracketed`. No mutant was rebuilt.

## 1. Summary

| Class | Count |
|---|---|
| equivalent (no output can change, or dead code) | 69 |
| instrument-only (verify.rs, test oracles, CLI-only readers) | 26 |
| gap (real behaviour no test pins) | 88 |
| gap only in run scope (killed by gear-io's tests) | 1 (material.rs:390) |

- **Dead code among the equivalents (12)**, each removed by an existing task:
  - auto.rs:963, 1383, 1389, 1398, 1399, 1618: the stage-era search (`Freedoms`, `Pinned::place`, free `maximise`) [ablate-features#0] T12.5 = T17.3.
  - ratio.rs:115, 186 ×2: `Ratio::signum`, and `Ratio::scale`, whose only caller is the dead `TrainMotion::read` (T17.2).
  - contact.rs:506: `split_residual` (T17.5).
  - strength.rs:618: `ToothOutline::flank_curvature` [added#34] (T17.7).
  - ring.rs:1025: `reference_geometry`'s rack-mismatch guard [ring#7, lens-unification#3] (T05.12).
- **Other equivalents:**
  - `<` against `<=` on continuous values;
  - ties already returned earlier;
  - division or multiplication by an exact ±1 sign;
  - safeguarded solves whose seed, slope or bracket pace changes only the evaluation count;
  - the unreachable fully-filleted ring branch [ring#5] (T05.2).
- **Instrument-only:**
  - the 14 verify.rs survivors (the rack and ring gates);
  - the Tooth copies of span and pins in metrology.rs (test oracles, T17.6);
  - `Search::refined`;
  - `equivalent_to_rack`;
  - `planetary::power` (the flow's oracle);
  - `Clamps::any`, `motion_report`'s inputs, `Train::split`, `GearResult::as_asked` and `place()`, which only the CLI or tests read.
- **Two causes cover most gaps:**
  - Every fixture is at module 1, so `/m` and `*m` agree. Nine survivors in six files.
  - A threshold, note or refusal is never approached from both sides, or its branch never runs.
- **Gaps outside gear-core's suite:** jgma.rs:60 and metrology.rs:164 are recorded by `check_wasm`, and material.rs:390 is caught by gear-io. They still have no gear-core law.

**Real gaps per file** (file:line, ×n = several mutants on the line):

| File | Gaps | Lines |
|---|---|---|
| train/shape.rs | 14 | 98, 597, 849, 1676, 1807, 1849, 2041, 2576, 2700, 2756, 3529, 3648, 3711, 3873 |
| auto.rs | 12 | 338, 357 ×2, 358, 359, 376, 643, 671, 761, 908, 1196, 1209 |
| train/mod.rs | 9 | 107, 424, 475, 661, 1513, 1950, 2151, 2334, 4274 |
| gear.rs | 8 | 484, 502 ×4, 504, 733, 971 |
| ring.rs | 7 | 252, 258, 364, 587, 702, 797 ×2 |
| metrology.rs | 6 | 164 ×2, 368 ×2, 728, 731 |
| solve.rs | 6 | 54, 104 ×2, 108, 152, 177 |
| outline.rs | 3 | 174, 264, 276 |
| screw.rs | 3 | 805, 997, 1223 |
| train/conditions.rs | 3 | 591, 649, 1293 |
| train/edits.rs | 3 | 306, 510, 671 |
| hertz.rs | 2 | 322, 367 |
| train/arrangements.rs, flow.rs, graph.rs | 2 each | 376, 725 · 167, 323 · 97, 325 |
| mesh.rs, shaper.rs, strength.rs, jgma.rs, train/groupings.rs, train/wiring.rs | 1 each | 172 · 471 · 813 · 60 · 236 · 240 |

## 2. Laws that would kill the gaps

Ordered by how many survivors each kills. Effort uses the plan's S/M/L.

**L1. Module similarity.** Scale every length input by λ ∈ {0.3, 2.5, 7}. This covers module, face width, distances, clearances and axial float, and applies to a lone gear, a ring and its shaper, a helical pair (β 15°), a crossed pair and the worm, and every preset of the T16.3 grid. Every dimensionless figure must be unchanged to 1e-9: ε_α, ε_β, η, shifts, Y_F, K_f, clamp and note keys. Every length must scale by λ, and every stress by 1/λ² at torque·λ. Also run T16.13's independent bending gate at m ≠ 1. *Kills* auto.rs:761, ring.rs:587, shaper.rs:471, strength.rs:813, train/edits.rs:306, train/mod.rs:107, 2151, train/shape.rs:3529 (with a nonzero axial float), 3873, and the instrument mutant verify.rs:293. *Belongs to* a module axis in T16.3's grid and in `tests/common`, T16.12 (ring grid), T16.13 [strength#5, lens-tests-geometry#3], T16.23, and T07.16 for the worm [crossed-worm#20]. Effort M.

**L2. Range edges are clamp edges.** Every bound and threshold that `admissible_ranges` and `admissible_profile_shift` publish must be the edge where the tooth's own clamp or note fires: fillet capped, tip pointed, tip above base, root off axis, shallow cut. Bisect each clamp on the `tests/common` grid (spur and helical, concentric and eccentric, including a small gear at a large shift). Require the published value to 1e-9, with the clamp silent just inside and firing just outside. A counter asserts that every threshold was present at least once (T16.6). *Kills* auto.rs:338, 357 ×2, 358, 359, 376, 671. *Belongs to* T12.14 [lens-unification#9], T12.17 [auto-search#10], T12.15 = T15.13 [auto-search#13, primitives#8]. Effort M.

**L3. Eccentric gear read back from its teeth.** Two checks:
- Each pin of `over_pins_at` touches both flanks of its space on `Gear::profile` to 1e-6 mm, for z ∈ {17, 23, 31}, Δx ∈ {0.2, 0.4} and λ ∈ {0, 0.5, 1}; on a concentric gear λ changes nothing, bit for bit.
- `Variation.tooth_thickness` equals the max − min of the closed-form thickness at each tooth's `shift_at`, the sinusoid's mean equals the sample mean, and |amplitude − half the range| ≤ `sinusoid_error`.

*Kills* gear.rs:484, 502 ×4, 504, 733, 971. *Belongs to* T09.3, T16.24, T04.6 and T16.23 [metrology#7, ablate-features#15, lens-unification#0]. Moot if T04.8 removes the eccentric gear. Effort M.

**L4. Every note fires on one side of its threshold, with its values.** A table-driven test: for each note, one fixture on either side of its threshold. The note fires on one side only, and the numbers it quotes are the report's figures. It covers:
- `mesh.overlap_below_one`: never on spur, always on partial helical;
- the undercut note against a severed member;
- `part.clearance_negative`: overlap = −clearance > 0;
- `gear.bending_unrated_in_mesh`: meshes numbered from 1;
- `near_self_locking`: μ between 0.8 and 1.0 of the threshold.

*Kills* train/mod.rs:475, 661, 1950, train/shape.rs:2700, 3711. *Belongs to* T16.20's per-firing check [gear-io#15, added2#29], T06.4 [added#2] and T07.13 / T15.9 [added2#73, lens-magic-numbers#0]. Effort S–M.

**L5. Solver contract: work count and non-finite ends.** Count evaluations through a wrapping closure on a fixed benchmark: x² − 2, cos x − x, eˣ − 5, a flat cubic, atan from a far guess, and a seeded smooth set. Each solver stays at or below its recorded count, `newton_bracketed` stays within one bisection of its log₂ bound, and no evaluation falls outside [lo, hi]. Both solvers return None when either end value is NaN or ±∞ (four cases each). In a replica the surviving mutants cost +28 %, +41 % and +139 % (Brent), and 56 evaluations against 10 (Newton). *Kills* solve.rs:54, 104 ×2, 108, 152, 177:65. *Belongs to* T16.8 and T15.4 / T02.9 [primitives#4, primitives#5]. Effort S.

**L6. Tooth and Gear metrology agree, and every branch counts.** On a concentric gear:
- `span_over_teeth_around` equals the Tooth form, including `contact_radius` and the Some/None verdict;
- `best_span_around`'s k equals `best_span`'s;
- `pin_diameter_range_around` equals `pin_diameter_range` bit for bit.

Replace the tests' silent `let … else { continue }` with a counter that must reach at least 1 per (z, k). *Kills* metrology.rs:164 ×2, 368 ×2, and the oracle survivors metrology.rs:260 and 293 ×2. *Belongs to* T17.6 (the Tooth forms become the oracle), T09.3, T16.24 and T16.6 [metrology#12, metrology#17, wasm-boundary#3, lens-tests-train#4]. Effort S.

**L7. A ring is drawn continuous, simple and to tolerance.** Three checks:
- In `Ring::half_profile`, each section ends where the next begins, and the last point is at `half_pitch`.
- The outline of every gear and ring, with bulges flattened, has positive signed area equal to `Gear::profile`'s within the chord error, and no self-crossing (T04.10's law on every gear).
- The ring gets T04.2's point-to-arc deviation law and the nonsense-tolerance fallback (0, −1, NaN, ∞).

*Kills* ring.rs:797 ×2, outline.rs:174, 264, 276. *Belongs to* T04.2, T04.5, T04.7, T04.10 and T05.17 [gear-outline#0, gear-outline#7, gear-outline#12, added#25, wasm-boundary#12]. Effort S.

**L8. Contact patch closes on its load.** For every line patch, the half width equals the independent b = √(4F′ρ/(πE*)) and p_max = 2F′/(πb). For an untruncated point patch, p_max = 3F/(2πab), and a truncated ellipse's length equals the line length. *Kills* hertz.rs:322, train/mod.rs:424, train/shape.rs:2576. *Belongs to* T15.17 and T08.11 / T08.12 [ablate-constants-rating#6, strength#8]. Effort S.

**L9. Train invariant and the held-body rule.** T13.1's `Train::check` runs over the seeded walk: every case entry names a listed, unheld, open port, and `held` is a subset of the listed bodies. A `Hold` of ground or of an unknown body is refused with `NoSuchIndex` and changes nothing. Add T13.3's Join that holds a loaded body: the headline must not start there, and `chain_ends` must not return a held end. *Kills* train/conditions.rs:591, 649, 1293. *Belongs to* T13.1 and T13.3 [graph-ops#6, graph-ops#8, edit-ops#5, edit-ops#6]. Effort M, landing with those tasks.

**L10. Relief honoured, and relief at a fixed point.** Run T16.7's law (pin everything, nudge one freedom, relieve) over the crossed and worm presets too, including a given distance with both shifts pinned. Add two assertions: relieving the relieved shape moves nothing, and the grid contains a state that needs a second pass (counted as in T16.6). *Kills* train/mod.rs:2334, train/shape.rs:849. *Belongs to* T16.7 and T10.9 [train-mod-a#8, train-mod-a#9, added2#11]. Effort M.

**L11. The shipped search against a scan.** T16.9's re-aimed scan law, plus a fixture of three free members in a chain (an idler with the search on): the chosen point is no worse than the best admissible grid point within SLACK. Add T12.2's law: turning the search on for mesh k leaves every shift outside k's component bit-identical. *Kills* train/shape.rs:1807, 1849, 2041. *Belongs to* T16.9 and T12.2 [auto-search#7, ablate-features#1, shape-a#2]. Effort M.

**L12. Ring clamps and domain edges.** Three checks:
- Port `the_geometric_bounds_are_exactly_where_the_generator_starts_clamping` to `Ring`: no clamp a hair inside the space bound, and `clamp.ring_space_*` a hair outside.
- One clamped ring goes through the ring cut gate.
- rf(x) is Lipschitz across the cutter's involute-domain edge on 60/50, 60/20 and 43/20.

*Kills* ring.rs:252, 258, 364. *Belongs to* T05.13, T05.4 and T16.12 [added2#79, lens-errors-policy#1, lens-errors-policy#7, ring#2]. Effort S.

**L13. Crossed-mesh contracts.** Three checks:
- A crossed pair whose tips do not cover the tangency gap has no path, and its mesh is refused as no contact.
- `face_widths_for(target)` fed back to `limited_by_face` gives ε = target to 1e-12, on a shifted pair off its reference distance, with the zone longer on either side, and at m = 0.5.
- The quoted locking friction μ* gives path efficiency 0 to 1e-9 whenever it is not the pitch-point fallback, with a counter showing the path branch ran.

*Kills* screw.rs:805, 997, 1223:49. *Belongs to* T07.3, T07.4 / T07.5 and T07.12 [crossed-worm#1, crossed-worm#2, crossed-worm#11, crossed-worm#12, added2#15]. Effort S.

**L14. Each refusal fires alone.**
- For `MeshTrial` and `CrossedTrial`: one fixture per refusal (tip reaching the mate root, ε below the minimum and the others) that violates only that refusal, and is refused under it; after T12.7, under its named key.
- For the contact primitives: `shift_sum_for` below the base-circle distance limit returns None, a pinned train distance there is refused with no NaN, and `relative_curvatures` with an infinite curvature returns None.

*Kills* auto.rs:1196:41, 1209, mesh.rs:172, hertz.rs:367. *Belongs to* T12.7 and T16.9 [auto-search#0], and T16.21 and T02.8 [mesh-contact#3, lens-errors-policy#14, lens-errors-policy#19]. Effort S.

**L15. Flow: part balance, load scale, coupling rows, break-away.** Four laws:
- In every solved case, each part's `CaseLoad.torques` sum to zero, and summed over parts at a body they give `CaseBody.torque`. This is the `on_body` claim, and today only the planetstage corpus sees it.
- Extend T16.4's mirror law: scaling every torque by λ ∈ {1e-3, 1e3} leaves the efficiencies and solved flags unchanged and scales the torques.
- T16.28, extended so every offset coupling appears once as a Coupling row, on a train whose coupling carries power.
- T16.5's Wolfrom break-away fixture.

*Kills* train/flow.rs:167, 323, train/groupings.rs:236, train/mod.rs:4274. *Belongs to* T16.3, T16.4, T16.5 and T16.28 [lens-magic-numbers#8 (T15.12), lens-tests-train#9, edit-ops#3, lens-tests-train#2]. Effort S each.

**L16. Edits keep what the graph says.** Four checks:
- T13.12: Remove(Axis) with a coupled bare body, and a layshaft whose output gear is moved off with the output held. The body stays a port of the first part on its axis, and the train stays well formed.
- T16.29's pins: a follower added at a mate starts with an automatic shift, and a carried axis added after a planet-gap edit takes that gap.
- `graph_of` on a stages file with two non-coaxial joins gives each coupling its own body and solves as the chain does.

*Kills* train/edits.rs:510, 671, train/graph.rs:97, 325, train/arrangements.rs:376. *Belongs to* T13.12, T16.29 and T14.4 [edit-ops#10, edit-ops#11, graph-ops#8, graph-ops#9]. Effort S.

**L17. Report identities.** Three checks:
- For every `GearResult`: lead angle + |β| = 90°, and d = z m / cos β.
- For every point mesh: sliding speed v₁ sin Σ / cos β₂ from the pitch-line speeds.
- For every replicated axis: the neighbour clearance equals the centre spacing 2r sin(π/n) − d_a, `clearance_ok` agrees with it, and a nearly touching fixture is included.

*Kills* train/mod.rs:1513, train/shape.rs:2756, 3648. *Belongs to* T14.11 [train-mod-a#11] and T10.2 [shape-b#0]. Effort S.

**L18. One-survivor laws, each S:**
- **Ring pair evenness.** Run `the_shift_sum_is_divided_as_evenly_as_the_members_allow` at sign −1. Kills auto.rs:908. Task T05.17.
- **Tight angular amplitude.** Check at amp·(1 ± 1e-3), with a fixture where the sink limit binds. Kills auto.rs:643 (`== `). Task T02.9.
- **Cutter tip width.** `cutter_tip_width` equals the settled rack's tip width, and the textbook value when unclamped. Kills metrology.rs:728, 731. Task T09.9 = T03.2 [added#13].
- **Outline tangent.** The analytic tangent equals a central difference for `flank_at` and `fillet_at` on both Tooth and Ring. Kills ring.rs:702. Tasks T16.23 and T05.17 [ring#9].
- **Order invariance of the plan.** Swapping a and b in a mesh, or permuting meshes, changes no shift. Kills train/shape.rs:597. Task T10.5 [shape-a#1].
- **Shared bracket walk.** `grow_bracket` finds a root beside a NaN floor. Kills train/shape.rs:1676. Task T15.5 [primitives#13, lens-magic-numbers#11].
- **Per-member cycles.** Kills train/wiring.rs:240. Task T10.13 [shape-b#5].
- **Worm seeds.** Turning each automatic worm helix given at its seed changes nothing. Kills train/arrangements.rs:725. Task T07.16.
- **Document defaults.** A member with no `pressure_angle` reads 20°. Kills train/shape.rs:98. Task T14.4 [gear-io#8].
- **Scale name round-trip.** Kills jgma.rs:60. Task T19.10 [metrology#6].

**Run scope.** Mutate with the workspace's tests (`--test-workspace`), so gear-io and `check_wasm` count: material.rs:390 is already killed that way. The verify.rs survivors call for two negative fixtures under T16.11: a cutter moved into the blank must fail the penetration half (verify.rs:170), and the SDF cross-check must assert a distance ≥ 0 (verify.rs:289).

## 3. All survivors

| # | file:line:col | function | mutation | class | reason / law |
|---|---|---|---|---|---|
| 1 | auto.rs:137:25 | `minimum_profile_shift`| `< → <=` | equivalent | f0 == 0 is returned before this branch, so < and <= agree |
| 2 | auto.rs:338:64 | `admissible_profile_shift`| `+ → *` | gap | **Range edges are clamp edges**. shared tool depth ignores the current shift: the min (root-off-axis) bound moves for small z typed above x = h_f - 0.05; no test puts a small gear at a large shift [lens-unification#9, auto-search#10, auto-search#13, primitives#8, T12.14, T12.17, T12.15] |
| 3 | auto.rs:357:39 | `admissible_profile_shift`| `+ → *` | gap | **Range edges are clamp edges**. pointed-shift threshold (crosses to the panel) miscomputed; the only test checks one side and is skipped when it is None [lens-unification#9, auto-search#10, auto-search#13, primitives#8, T12.14, T12.17, T12.15] |
| 4 | auto.rs:357:62 | `admissible_profile_shift`| `/ → *` | gap | **Range edges are clamp edges**. pointed threshold wrong for helical gears only (/cos b vs *cos b); every fixture is spur [lens-unification#9, auto-search#10, auto-search#13, primitives#8, T12.14, T12.17, T12.15] |
| 5 | auto.rs:358:24 | `admissible_profile_shift`| `/ → *` | gap | **Range edges are clamp edges**. pointed threshold wrong (psi_b garbage); the test is conditional on Some and never checks the edge from below [lens-unification#9, auto-search#10, auto-search#13, primitives#8, T12.14, T12.17, T12.15] |
| 6 | auto.rs:359:45 | `admissible_profile_shift`| `+ → *` | gap | **Range edges are clamp edges**. pointed threshold takes r + m*h_a*x as the tip radius; not pinned as an edge [lens-unification#9, auto-search#10, auto-search#13, primitives#8, T12.14, T12.17, T12.15] |
| 7 | auto.rs:376:34 | `admissible_profile_shift`| `- → +` | gap | **Range edges are clamp edges**. eccentric gear shallow-cut threshold (shown in GearPanel) takes +off_hi; no eccentric threshold is tested [lens-unification#9, auto-search#10, auto-search#13, primitives#8, T12.14, T12.17, T12.15] |
| 8 | auto.rs:643:27 | `admissible_angular_shift`| `> → ==` | gap | **Angular-amplitude edge is tight**. drops the by_sink limit for every z; the gear.rs edge test has only spread-bound fixtures and checks 1.5x past the bound [T02.9] |
| 9 | auto.rs:643:27 | `admissible_angular_shift`| `> → >=` | equivalent | sink = -cos(2pi floor(z/2)/z) is never exactly 0 for integer z, so > and >= agree [T02.9] |
| 10 | auto.rs:671:33 | `ranges_at_shift`| `+ → -` | gap | **Range edges are clamp edges**. addendum lower bound (tip above base) uses 1-frac; no test pins a range bound to the tooth clamp it restates [lens-unification#9, auto-search#10, auto-search#13, primitives#8, T12.14, T12.17, T12.15] |
| 11 | auto.rs:740:19 | `addendum_for_tip_width`| `< → ==` | equivalent | width(0) < min leaves the second brent unbracketed (width(u_point)=0 < min), None either way [added#5, T03.8] |
| 12 | auto.rs:761:21 | `addendum_for_tip_width`| `/ → *` | gap | **Module similarity**. (ra - r) * m instead of / m: every test of addendum_for_tip_width uses module 1 [T16.23] |
| 13 | auto.rs:848:9 | `searchable_shift`| `< → <=` | equivalent | round_fits(lo) == 0 gives brent the root lo and (lo < lo) is false: None either way [auto-search#14, T12.14] |
| 14 | auto.rs:908:17 | `divide_shift_sum`| `* → +` | gap | **Even division holds for a ring pair**. internal-pair (sign -1) contribution interval wrong; the evenness scan runs only at sign +1 [ring#9, T05.17] |
| 15 | auto.rs:908:17 | `divide_shift_sum`| `* → /` | equivalent | k = sign is exactly -1 in this arm, so hi / k = hi * k |
| 16 | auto.rs:921:37 | `divide_shift_sum`| `/ → *` | equivalent | sign is kind.sign(), exactly +-1, so /sign = *sign |
| 17 | auto.rs:963:9 | `Freedoms<N>::boxes` | `body → vec![(-1.0, 1.0)]` | equivalent | Freedoms has no caller at all (dead stage-era search) [ablate-features#0, T12.5, T17.3] |
| 18 | auto.rs:1196:41 | `CrossedTrial<'_>::efficiency`| `- → /` | gap | **Each trial refusal fires alone**. disables the crossed search refusal "tip reaching the mate root"; no crossed trial is ever refused on it [auto-search#0, T12.7, T16.9] |
| 19 | auto.rs:1196:59 | `CrossedTrial<'_>::efficiency`| `< → <=` | equivalent | < vs <= on a continuous clearance; equality is measure zero |
| 20 | auto.rs:1209:31 | `CrossedTrial<'_>::efficiency`| `< → <=` | equivalent | < vs <= on a continuous contact ratio; equality is measure zero |
| 21 | auto.rs:1209:31 | `CrossedTrial<'_>::efficiency`| `< → ==` | gap | **Each trial refusal fires alone**. disables the crossed search contact-ratio refusal; nothing tests a crossed candidate below min eps [auto-search#0, T12.7, T16.9] |
| 22 | auto.rs:1383:53 | `Pinned::place`| `/ → %` | equivalent | Pinned::place serves only the dead shifts_for_efficiency (reached by tests only) [ablate-features#0, T12.5, T17.3] |
| 23 | auto.rs:1389:62 | `Pinned::place`| `/ → %` | equivalent | Pinned::place serves only the dead shifts_for_efficiency (reached by tests only) [ablate-features#0, T12.5, T17.3] |
| 24 | auto.rs:1398:37 | `Pinned::place`| `/ → %` | equivalent | Pinned::place serves only the dead shifts_for_efficiency (reached by tests only) [ablate-features#0, T12.5, T17.3] |
| 25 | auto.rs:1399:44 | `Pinned::place`| `* → +` | equivalent | Pinned::place serves only the dead shifts_for_efficiency (reached by tests only) [ablate-features#0, T12.5, T17.3] |
| 26 | auto.rs:1618:5 | `maximise` | `body → Some(vec![0.0])` | equivalent | free fn maximise has no caller, not even a test [ablate-features#0, T12.5, T17.3] |
| 27 | auto.rs:1733:46 | `Search::refined`| `* → /` | instrument-only | Search::refined is a test oracle (called only from tests in crossed.rs and train/mod.rs); budget*k/k only weakens the reference |
| 28 | contact.rs:506:32 | `split_residual`| `\|\| → &&` | equivalent | Dead: split_residual has no production caller; the guard differs only at a zero tip pressure angle (inf vs NaN) [mesh-contact#10, T17.5] |
| 29 | elliptic.rs:202:51 | `r_d`| `- → /` | equivalent | Alters an O(δ⁴) series term below TOL = ε^(1/6); measured change ≤ 7e-15 relative over the test and Hertz arguments, the rounding floor |
| 30 | gear.rs:371:26 | `Gear::root_reach`| `% → +` | equivalent | Gear::tooth already wraps its index mod n, so j+n and j%n read the same tooth |
| 31 | gear.rs:484:27 | `Gear::space_half_angle`| `- → /` | gap | **Eccentric pins touch the drawn teeth**. λ·(a−b) vs λ·(a/b) differ only at λ≠0; no test measures pins at λ≠0 (concentric λ=0 has a=b; the eccentric pin test only checks hi≥lo) [metrology#7, ablate-features#15, T09.3, T16.24, T04.8] |
| 32 | gear.rs:502:51 | `Gear::space_centre_delta`| `- → /` | gap | **Eccentric pins touch the drawn teeth**. λ term of the pin-seat delta; zero at λ=0 and no pin test runs λ≠0 [metrology#7, ablate-features#15, T09.3, T16.24, T04.8] |
| 33 | gear.rs:502:69 | `Gear::space_centre_delta`| `+ → *` | gap | **Eccentric pins touch the drawn teeth**. psi(b+1)→psi(b) inside the λ term; invisible at λ=0, the only value pin tests use [metrology#7, ablate-features#15, T09.3, T16.24, T04.8] |
| 34 | gear.rs:502:76 | `Gear::space_centre_delta`| `+ → *` | gap | **Eccentric pins touch the drawn teeth**. drops the non-λ spread on eccentric gears; the eccentric pin test asserts only hi≥lo [metrology#7, ablate-features#15, T09.3, T16.24, T04.8] |
| 35 | gear.rs:502:94 | `Gear::space_centre_delta`| `+ → *` | gap | **Eccentric pins touch the drawn teeth**. psi(a+1)→psi(a) in the spread; only eccentric pins see it and nothing measures them [metrology#7, ablate-features#15, T09.3, T16.24, T04.8] |
| 36 | gear.rs:504:24 | `Gear::space_centre_delta`| `/ → *` | gap | **Eccentric pins touch the drawn teeth**. spread/2→spread*2; concentric spread is exactly 0 and eccentric pins are unmeasured [metrology#7, ablate-features#15, T09.3, T16.24, T04.8] |
| 37 | gear.rs:733:53 | `Gear::variation`| `* → +` | gap | **Eccentric summaries read back from their samples**. Variation.tooth_thickness (reaches wasm) is pinned only as zero-width on a concentric gear; the tabulated test checks base thickness only [ablate-features#15, T16.23, T04.8] |
| 38 | gear.rs:971:29 | `centre_profile_of`| `* → /` | gap | **Eccentric summaries read back from their samples**. the fitted sinusoid's amplitude is checked only against itself (throw inversion, λ-invariance, flat gear), never against the commanded samples [lens-unification#0, ablate-features#15, T04.6, T04.8] |
| 39 | gear.rs:974:48 | `centre_profile_of`| `/ → %` | equivalent | the sine coefficient is structurally zero (shift_at is even in θ), so s/n vs s%n changes only a 1e-16 residual and the phase keeps its sign |
| 40 | hertz.rs:322:5 | `line_half_width` | `body → 1.0` | gap | **Contact patch closes on its load**. Line-contact patch width reaches no output or law; the one test compares line with point patch at 10 % and both route through this function [ablate-constants-rating#6, T15.17] |
| 41 | hertz.rs:367:68 | `relative_curvatures`| `\|\| → &&` | gap | **Contact-primitive refusals fire**. Non-finite curvature guard untested: with && an infinite curvature (flank at its base cylinder) returns Some(NaN, inf) instead of None [lens-errors-policy#19, T16.21] |
| 42 | jgma.rs:60:9 | `Scale::as_str` | `body → "xyzzy"` | gap | **Scale name round-trips**. Scale::as_str is read only by gear-wasm (parse and serialise); no gear-core test, and check_wasm's record is outside this run [metrology#6, T19.10] |
| 43 | material.rs:390:9 | `MaterialLibrary::is_empty` | `body → true` | gap (scope) | **Mutate the workspace**. Not a gear-core gap: only gear-io::from_toml calls it, and gear-io's tests (default library loads) fail under it; the run tested gear-core only [T16.30] |
| 44 | mesh.rs:172:20 | `shift_sum_for`| `&& → \|\|` | gap | **Contact-primitive refusals fire**. A distance below the base-circle limit returns Some(NaN) instead of None; no test asks shift_sum_for (or a pinned distance) below that limit [mesh-contact#3, lens-errors-policy#14, T02.8, T16.21] |
| 45 | metrology.rs:132:33 | `Space::seat`| `* → /` | equivalent | σ=±1, so σ/(c−form) has the sign of σ·(c−form); differs only at exact equality |
| 46 | metrology.rs:135:45 | `Space::seat`| `> → >=` | equivalent | > vs >= at contact exactly on the tip circle, a continuous value |
| 47 | metrology.rs:164:5 | `pin_diameter_range_around` | `body → Some((0.0, 0.0))` | gap | **Tooth and Gear metrology agree**. pin_diameter_range_around reaches only gear-wasm; no gear-core test calls it (check_wasm records it outside this run) [wasm-boundary#3, metrology#7, T09.3, T14.3] |
| 48 | metrology.rs:164:5 | `pin_diameter_range_around` | `body → Some((1.0, -1.0))` | gap | **Tooth and Gear metrology agree**. pin_diameter_range_around reaches only gear-wasm; no gear-core test calls it (check_wasm records it outside this run) [wasm-boundary#3, metrology#7, T09.3, T14.3] |
| 49 | metrology.rs:216:15 | `pin_diameter_range`| `< → <=` | equivalent | < vs <= on two bisected ends; equal ends also fail the midpoint seat check |
| 50 | metrology.rs:260:31 | `span_over_teeth`| `* → +` | instrument-only | Tooth copy of the span (contact radius); ships only to tests and strings.rs, production uses span_over_teeth_at (metrology#12, T17.6). The Tooth=Gear law kills it [metrology#12, T17.6] |
| 51 | metrology.rs:293:47 | `best_span`| `- → +` | instrument-only | best_span's k choice on the Tooth copy; test oracle only (metrology#12, T17.6). The Tooth=Gear law kills it [metrology#12, metrology#2, T17.6, T09.4] |
| 52 | metrology.rs:293:47 | `best_span`| `- → /` | instrument-only | best_span's k choice on the Tooth copy; test oracle only (metrology#12, T17.6). The Tooth=Gear law kills it [metrology#12, metrology#2, T17.6, T09.4] |
| 53 | metrology.rs:368:42 | `span_over_teeth_at`| `/ → %` | gap | **Tooth and Gear metrology agree**. breaks the usable-flank interval so spans come back None; both span tests `continue` on None and never count what they measured [metrology#17, lens-tests-train#4, T16.24, T16.6] |
| 54 | metrology.rs:368:61 | `span_over_teeth_at`| `- → /` | gap | **Tooth and Gear metrology agree**. same: the span's validity window is wrong and the tests skip every None silently [metrology#17, lens-tests-train#4, T16.24, T16.6] |
| 55 | metrology.rs:489:50 | `pin_seat`| `> → >=` | equivalent | σ is always ±1, never 0 |
| 56 | metrology.rs:575:44 | `over_pins` | `delete - in over_pins` | equivalent | dead branch: the datum normal always points outward (dot(n,p1)=r·cos(π/z)>0 for z≥3); the Tooth copy is test-only too [metrology#12, T17.6] |
| 57 | metrology.rs:728:5 | `cutter_tip_width` | `body → 0.0` | gap | **Cutter tip width is the settled rack's**. cutter_tip_width (shown in the gear tab) is tested only for helix independence, which 0.0 satisfies [added#13, T09.9, T03.2] |
| 58 | metrology.rs:731:59 | `cutter_tip_width`| `* → /` | gap | **Cutter tip width is the settled rack's**. same figure with a wrong depth term; helix independence still holds [added#13, T09.9, T03.2] |
| 59 | outline.rs:174:30 | `Tooth::tooth_outline`| `+ → -` | gap | **Every outline is simple and counter-clockwise**. mirrors each tooth's traversal, so the outline zig-zags and self-crosses; the tolerance test measures radial deviation only and nothing checks orientation or crossing [added#25, gear-outline#0, T04.10, T04.5] |
| 60 | outline.rs:264:69 | `crate::ring::Ring::outline`| `> → >=` | gap | **A ring is drawn continuous and to tolerance**. a zero tolerance on a ring drops to the 1e-12·rf floor instead of the default; the fallback law is written for Gear only [wasm-boundary#12, gear-outline#12, T04.7] |
| 61 | outline.rs:276:40 | `crate::ring::Ring::outline`| `- → /` | gap | **A ring is drawn continuous and to tolerance**. ring root-arc bulge wrong; the ring outline test checks vertices, not the arcs they carry [gear-outline#7, T04.2, T05.17] |
| 62 | params.rs:214:9 | `Clamps::any` | `body → true` | instrument-only | Clamps::any is read only by gear-cli (main.rs:2460, 2509) and by tests that assert it true on clamped gears |
| 63 | planetary.rs:290:35 | `power`| `> → >=` | instrument-only | planetary::power is the test oracle for the shape's flow; >= differs only at exactly zero output power (k = 1), which no arrangement reaches |
| 64 | ratio.rs:115:9 | `Ratio::signum` | `body → 0` | equivalent | Dead: Ratio::signum has no caller anywhere [lens-architecture#15, T17.2] |
| 65 | ratio.rs:186:9 | `Ratio::scale` | `body → 1.0` | equivalent | Dead: Ratio::scale's only caller is TrainMotion::read (conditions.rs:178), which nothing calls [lens-architecture#15, T17.2] |
| 66 | ratio.rs:186:31 | `Ratio::scale`| `/ → %` | equivalent | Dead: as above, TrainMotion::read is never called [lens-architecture#15, T17.2] |
| 67 | ring.rs:252:60 | `Ring::cut_by_at_virtual_z`| `* → +` | gap | **Ring space clamps fire at their thresholds**. the ring space floor (0.02 m) is never approached by a test; 0.02+m moves it to 1.02 mm unseen [added2#79, lens-errors-policy#7, T05.13, T16.12] |
| 68 | ring.rs:258:79 | `Ring::cut_by_at_virtual_z`| `* → /` | gap | **Ring space clamps fire at their thresholds**. the shift read back from a clamped space is used only when a space clamp fires, and no test cuts a clamped ring [added2#79, T05.13, T16.12] |
| 69 | ring.rs:296:18 | `Ring::cut_by_at_virtual_z`| `< → <=` | equivalent | at ψ_b = 0 the guard gives ra_min = r_b, already below the r_b(1+1e-9) floor |
| 70 | ring.rs:364:23 | `Ring::cut_by_at_virtual_z`| `- → +` | gap | **Ring root continuous across the cutter domain**. the reference-centre fallback outside the cutter's involute domain is never reached by a test; T05.4 deletes it [ring#2, lens-errors-policy#1, T05.4] |
| 71 | ring.rs:535:37 | `Ring::solve_junction`| `> → >=` | equivalent | strict vs non-strict bound on a continuous radius |
| 72 | ring.rs:535:54 | `Ring::solve_junction`| `< → <=` | equivalent | strict vs non-strict bound on a continuous radius |
| 73 | ring.rs:564:9 | `Ring::solve_root_end` | `body → 0.0` | equivalent | the fully-filleted branch is unreachable (clamp.ring_fully_filleted is in strings.rs UNFIRED after a 71,750-case search), so s_root is always 0; T05.2 rewrites this solve [ring#5, T05.2] |
| 74 | ring.rs:564:36 | `Ring::solve_root_end`| `<= → >` | equivalent | with that branch unreachable, the inverted test sends every ring to a brent with no sign change, which returns the same 0.0 [ring#5, T05.2] |
| 75 | ring.rs:587:22 | `Ring::rim_radius`| `* → /` | gap | **Module similarity**. rim radius r+2/m_t equals r+2m_t at m_t=1, the only module the ring tests use [ablate-constants-geometry#2, T05.7] |
| 76 | ring.rs:702:26 | `Ring::flank_point_and_tangent`| `* → /` | gap | **Outline tangent is the derivative**. the ring's flank tangent feeds the Lewis flank candidate; only the external fillet tangent is checked against a finite difference [ring#9, T16.23, T05.17] |
| 77 | ring.rs:720:16 | `Ring::flank_point_and_load_direction`| `< → ==` | equivalent | len < MIN_POSITIVE happens only at roll 0, which no load point reaches; T15.11 replaces the fallback [ablate-constants-geometry#6, T15.11] |
| 78 | ring.rs:797:76 | `Ring::sample_section`| `- → +` | gap | **A ring is drawn continuous and to tolerance**. ring section sampling stops short of each section's end (gaps in the drawn ring); no test checks that sections meet [gear-outline#7, T05.17] |
| 79 | ring.rs:797:76 | `Ring::sample_section`| `- → /` | gap | **A ring is drawn continuous and to tolerance**. ring section sampling stops short of each section's end (gaps in the drawn ring); no test checks that sections meet [gear-outline#7, T05.17] |
| 80 | ring.rs:1025:36 | `reference_geometry`| `> → >=` | equivalent | > vs >= at a tolerance edge on a float difference |
| 81 | ring.rs:1025:83 | `reference_geometry`| `> → ==` | equivalent | the pressure-angle mismatch guard is unreachable in production (TipRoom::at runs only on meshes already built from one rack); T05.12 deletes reference_geometry [lens-unification#3, ring#7, T05.12] |
| 82 | ring.rs:1035:32 | `reference_geometry`| `&& → \|\|` | equivalent | operating_geometry's a' = a_ref·cos α_t/cos α_w is positive and finite whenever it returns [ring#7, T05.12] |
| 83 | ring.rs:1086:60 | `described_at`| `> → >=` | equivalent | > vs >= at exact equality of two radii |
| 84 | screw.rs:446:32 | `Screw::least_distance_lead_angle`| `\|\| → &&` | equivalent | The guard sits outside the bracket: γ ∈ [1e-9, hi−1e-9] keeps \|sin γ\| and \|cos β₂\| above 1e-12, so it never fires |
| 85 | screw.rs:692:28 | `Screw::contact_normal`| `< → ==` | equivalent | Σ exactly 0 is still refused; 0 < \|sin Σ\| < ε is no crossed pair any builder makes |
| 86 | screw.rs:805:28 | `Screw::path_of_contact_at`| `\|\| → &&` | gap | **One no-contact rule**. A crossed pair whose tip reaches do not overlap gets Some(path) with negative length instead of None; no test builds a pair whose teeth do not meet [crossed-worm#11, added2#15, T07.3, T07.2] |
| 87 | screw.rs:856:44 | `Screw::mesh_branch`| `* → +` | equivalent | The pitch point lies on the chosen tangent line by construction (offset ~1e-12), so widening the acceptance never changes the branch |
| 88 | screw.rs:936:30 | `CrossedPath::limited_by_face`| `< → <=` | equivalent | Tie on a continuous value: at exactly centre+half = hi only the Face/Tips label flips |
| 89 | screw.rs:997:26 | `CrossedPath::face_widths_for`| `- → /` | gap | **Face-width round trip off reference**. Only reached when the zone is longer on its positive side and read off-reference; the round-trip test runs at the reference distance where max(want−half, half) = half masks it [added2#15, crossed-worm#1, T07.4, T07.5] |
| 90 | screw.rs:1033:47 | `CrossedPath::half_span`| `&& → \|\|` | equivalent | Differs only for a zero or non-finite face (degenerate input T01.8 refuses) or zero axial rate, where half = inf limits nothing either way [lens-errors-policy#13, T01.8] |
| 91 | screw.rs:1223:38 | `CrossedPath::locking_friction`| `&& → \|\|` | equivalent | A non-locking direction carries the −1 sentinel; brent over [0, −1] finds no sign change and returns None anyway [crossed-worm#12, T07.12] |
| 92 | screw.rs:1223:49 | `CrossedPath::locking_friction`| `> → ==` | gap | **Locking friction zeroes efficiency**. With == every direction falls back to the pitch-point threshold; no test checks that the quoted locking friction is where the quoted path efficiency reaches zero [crossed-worm#2, crossed-worm#12, T07.12] |
| 93 | screw.rs:1289:11 | `tangent_line`| `< → <=` | equivalent | Tolerance comparison on a continuous norm (< vs <= at ε) |
| 94 | screw.rs:1407:59 | `pitch_point_curvatures`| `\|\| → &&` | equivalent | flank_curvature refuses the same bad radius downstream (ρ_n must be finite and positive), so the answer is None either way |
| 95 | shaper.rs:444:28 | `ShaperCut::largest_tip_round`| `< → ==` | equivalent | when the floor's angle is negative, brent finds no bracket and the result clamps back to the same floor [primitives#12, T15.5] |
| 96 | shaper.rs:471:30 | `ShaperCut::phase_from`| `* → /` | gap | **Module similarity**. π·m_t/2 vs π/m_t/2 agree at m_t=1; no ring or shaper test runs at m≠1 or helical [lens-unification#2, T16.12] |
| 97 | shaper.rs:501:35 | `ShaperCut::equivalent_to_rack`| `\|\| → &&` | instrument-only | equivalent_to_rack is called only from shaper.rs's own tests (ring#12); a corner ≤ 0 is refused again by corner_angle [ring#12, T17.8] |
| 98 | solve.rs:54:24 | `brent`| `\|\| → &&` | gap | **Solvers refuse a non-finite end**. one non-finite end value no longer refuses; no test hands brent a NaN/inf end [primitives#5, T02.9, T15.4] |
| 99 | solve.rs:68:19 | `brent`| `- → +` | equivalent | initial d/e only steer step acceptance; Python replica: same roots, same evaluation count over 3000 functions [T15.4] |
| 100 | solve.rs:75:19 | `brent`| `- → +` | equivalent | reset of d/e on bracket swap; replica: same roots, same evaluation count [T15.4] |
| 101 | solve.rs:104:30 | `brent`| `* → +` | gap | **Solver work count**. inverse-quadratic step corrupted: same root, +28% evaluations (replica) [primitives#4, T15.4, T16.8] |
| 102 | solve.rs:104:35 | `brent`| `* → /` | gap | **Solver work count**. inverse-quadratic step corrupted: same root, +41% evaluations (replica) [primitives#4, T15.4, T16.8] |
| 103 | solve.rs:105:32 | `brent`| `* → /` | equivalent | q corrupted: same root, +4.5% evaluations, below what a count law should resolve [T15.4] |
| 104 | solve.rs:108:21 | `brent` | `delete - in brent` | gap | **Solver work count**. q sign flip lost: interpolation always rejected, Brent degrades to bisection (+139% evaluations; x^2-2: 14 -> 25) [primitives#4, T15.4, T16.8] |
| 105 | solve.rs:111:39 | `brent`| `- → /` | equivalent | acceptance bound falls back to \|e q\|; replica: same roots, +0.1% evaluations, nothing evaluated outside the bracket [T15.4] |
| 106 | solve.rs:152:25 | `newton_bracketed`| `\|\| → &&` | gap | **Solvers refuse a non-finite end**. same for newton_bracketed [primitives#5, T02.9, T15.4] |
| 107 | solve.rs:161:13 | `newton_bracketed`| `< → <=` | equivalent | flo == 0 already returned, so < and <= agree |
| 108 | solve.rs:177:47 | `newton_bracketed`| `* → /` | equivalent | out-of-range test is backed by the too-slow test; replica: same roots and counts, nothing evaluated outside the bracket [T15.4] |
| 109 | solve.rs:177:65 | `newton_bracketed`| `- → +` | gap | **Solver work count**. out-of-range test broken: same root, but atan from a far guess takes 56 evaluations instead of 10 (replica) [primitives#4, T15.4, T16.8] |
| 110 | solve.rs:200:15 | `newton_bracketed`| `< → <=` | equivalent | fx == 0 exactly makes the next Newton step 0 and returns x either way |
| 111 | strength.rs:618:17 | `<impl ToothOutline for crate::ring::Ring>::flank_curvature`| `* → +` | equivalent | Dead: ToothOutline::flank_curvature has no production caller [added#34, T17.7] |
| 112 | strength.rs:781:34 | `finish`| `< → <=` | equivalent | Differs only if the section tangent is exactly horizontal, which a root section never is |
| 113 | strength.rs:813:66 | `finish`| `/ → *` | gap | **Module similarity**. Y_F computed with (s·m)² instead of (s/m)² is identical at m = 1, the module of every gear-core bending test; only the corpus (m = 2) sees it [strength#5, lens-tests-geometry#3, T16.13] |
| 114 | tooth.rs:250:41 | `allocate_by_arc_length`| `&& → \|\|` | equivalent | sections() emits only non-empty sections; a zero-length one would only add coincident points [tooth-form#8] |
| 115 | tooth.rs:458:21 | `Tooth::build_with_z`| `* → /` | equivalent | wrong derivative in a safeguarded Newton: it falls back to bisection and reaches the same root [primitives#8, T15.13] |
| 116 | tooth.rs:461:18 | `Tooth::build_with_z`| `* → +` | equivalent | seed only; the bracketed solve reaches the same root [primitives#8, T15.13] |
| 117 | tooth.rs:592:34 | `Tooth::solve_junction` | `delete - in Tooth::solve_junction` | equivalent | s_tan = −b_c/tan α with b_c ≥ 0.05·b_d > 0 (round capped at 0.95 b_d), so s_tan is always negative [tooth-form#7, T15.5] |
| 118 | tooth.rs:599:18 | `Tooth::solve_junction`| `*= → +=` | equivalent | the first probe always brackets: the trochoid at s_tan sits at hypot(r_b, l) > r_b, so the growth step never runs [lens-magic-numbers#11, T15.5] |
| 119 | tooth.rs:620:17 | `Tooth::solve_junction`| `- → +` | equivalent | the nudge is 1e-6 m and the crossing is unique in the bracket, so brent returns the same root [lens-magic-numbers#11, T15.5] |
| 120 | tooth.rs:922:57 | `Rack::wanted_by`| `* → /` | equivalent | a 1e-9-module floor, re-applied as ·m on the next line; the change is sub-nanometre and prints as 0.0000 [lens-numerical-robustness#2, T03.3] |
| 121 | train/arrangements.rs:376:31 | `Shape::push_axis`| `> → >=` | gap | **Edit-sizing pins**. A new carried axis takes the first axis's planet gap, not the replicated axes'; untested once a designer changes it [edit-ops#11, T16.29] |
| 122 | train/arrangements.rs:725:46 | `worm`| `- → +` | gap | **Worm seeds are the worm's figures**. The worm wheel's automatic helix seed is never read unless relief pins it; nothing turns it given at its seed [crossed-worm#20, lens-magic-numbers#5, T07.16] |
| 123 | train/conditions.rs:522:25 | `Train::motion_of`| `&& → \|\|` | instrument-only | Chooses which body a family is written per; only motion_report (gear-cli kinematics) reads it; solve uses motion only to refuse [T16.2] |
| 124 | train/conditions.rs:591:9 | `Train::is_open` | `body → true` | gap | **Train invariant and held-body rule**. Openness guard on the headline load; Join/Insert can leave a load on a held body, and nothing asserts the headline skips it [edit-ops#5, graph-ops#6, graph-ops#8, T13.1, T13.3] |
| 125 | train/conditions.rs:649:54 | `Train::chain_ends`| `== → !=` | gap | **Train invariant and held-body rule**. Lets chain_ends return a held end where every port of an end part is held or shared; fresh_case would then write a case at a held body [graph-ops#8, T13.1] |
| 126 | train/conditions.rs:782:29 | `Train::motion_report`| `!= → ==` | instrument-only | The 'given' count in motion_report, read only by gear-cli kinematics (recorded in tools/golden/kinematics.txt) [T16.2] |
| 127 | train/conditions.rs:1044:28 | `Train::split`| `+ → -` | instrument-only | Train::split is called only by tests and the gear-cli kinematics fixture; the fresh body's list position is not asserted |
| 128 | train/conditions.rs:1293:35 | `Train::edit`| `\|\| → &&` | gap | **Train invariant and held-body rule**. Hold(ground) and Hold(unknown body) stop being refused; the second pushes an unlisted body into held [graph-ops#8, edit-ops#6, T13.1] |
| 129 | train/edits.rs:306:27 | `Shape::carrier_radius`| `* → /` | gap | **Module similarity**. Carrier radius (z_c±z_p)m/2 is exact only at m=1; every edit fixture is at module 1 [edit-ops#11, T16.29, T16.3] |
| 130 | train/edits.rs:510:17 | `Shape::push_follower` | `delete field profile_shift from struct MemberGear expression in Shape::push_follower` | gap | **Edit-sizing pins**. A follower copies its mate's shift (given or automatic) instead of starting automatic; no law reads the added member's shift [edit-ops#11, T16.29] |
| 131 | train/edits.rs:671:32 | `Shape::clear_axis`| `== → !=` | gap | **One bare-body rule**. clear_axis's own bare-body rule (the one that ignores couplings) is untested on a coupled bare body; drop_member covers the rest [edit-ops#10, T13.12] |
| 132 | train/flow.rs:167:22 | `Flow::on_shafts` | `delete - in Flow::on_shafts` | gap | **Part torque balance**. The frame's reaction goes only into a part's CaseLoad.torques (PartReport); no gear-core test sums a part's torques. The planetstage corpus prints it |
| 133 | train/flow.rs:226:42 | `solve`| `== → !=` | equivalent | Only reverses the order assignments are enumerated in; the winner is the strict maximum, and ties differ only in zero-power meshes [edit-ops#3, T16.28] |
| 134 | train/flow.rs:323:25 | `solve`| `* → /` | gap | **Load scale law**. Power zero tolerance becomes 1e-12/P instead of 1e-12·P; the same at P≈1, which is the magnitude every fixture uses [lens-magic-numbers#8, T16.4, T15.12] |
| 135 | train/graph.rs:97:26 | `graph_of`| `+= → -=` | gap | **graph_of over two offset joins**. Two non-coaxial joins in one stages file would get the same or clashing body numbers; graph_of's fixtures have at most one [graph-ops#9, T14.4] |
| 136 | train/graph.rs:325:64 | `Shape::parts`| `!= → ==` | gap | **One bare-body rule**. A bare body (e.g. a layshaft output with its gear moved off) stops being a port of the first part on its axis; no law holds a bare, named body [edit-ops#10, graph-ops#8, T13.12, T13.1] |
| 137 | train/groupings.rs:236:35 | `Train::flow`| `== → !=` | gap | **Flow order, idle and coupling rows**. Coupling rows of a flow are never checked; in the presets a coupled body is reached through the epicyclic junction anyway [lens-tests-train#9, edit-ops#3, T16.28] |
| 138 | train/mod.rs:107:82 | `ContactRatios::of`| `* → /` | gap | **Module similarity**. ε_β = b sinβ/(π m_n) is the same at m=1; the helical fixtures are at module 1 [T16.3] |
| 139 | train/mod.rs:424:37 | `ContactPatch::line`| `* → /` | gap | **Contact patch closes on its load**. The reported line patch width (curvature across) is never checked against the pressure and load it came from [strength#8, T08.11] |
| 140 | train/mod.rs:475:18 | `line_mesh_report`| `> → ==` | gap | **Note firing and values pinned**. mesh.overlap_below_one would fire on every spur and never on a partial helical; no test reads that note [added#2, T06.4] |
| 141 | train/mod.rs:661:21 | `undercut_note`| `&& → \|\|` | gap | **Note firing and values pinned**. The undercut note would also fire on severed teeth; no train fixture has a severed member [tooth-form#12, T03.9] |
| 142 | train/mod.rs:1513:30 | `GearResult::of`| `- → /` | gap | **Report-field identities**. GearResult.lead_angle (shown by TrainPanel) is never compared with 90° − \|β\| [train-mod-a#11, T14.11] |
| 143 | train/mod.rs:1550:9 | `GearResult::as_asked` | `body → true` | instrument-only | GearResult::as_asked has one caller, gear-cli main.rs:1157 (the harness's filter) [T18.21] |
| 144 | train/mod.rs:1703:5 | `place` | `body → String::new()` | instrument-only | place() only feeds TrainError's English Display, for the CLI and Debug; T02.1 removes that English [train-mod-a#14, T02.1] |
| 145 | train/mod.rs:1950:36 | `distance_notes` | `delete - in distance_notes` | gap | **Note firing and values pinned**. The clearance_negative note's 'overlap' value loses its sign; strings tests check the key fires, not the value [T16.20] |
| 146 | train/mod.rs:2151:46 | `helix_for_overlap`| `* → /` | gap | **Module similarity**. helix_for_overlap's π m_n/b is the same at m=1 [T16.3] |
| 147 | train/mod.rs:2334:16 | `shape::Shape::relieved` | `delete ! in shape::Shape::relieved` | gap | **Relief honoured and at a fixed point**. Relief stops after the first pass that moves; no fixture needs a second pass, so the fixed-point claim is unproven [train-mod-a#9, train-mod-a#8, added2#11, T10.9, T16.7] |
| 148 | train/mod.rs:2379:35 | `shape::Shape::settle`| `&& → \|\|` | equivalent | The spared pass then turns nothing and the unspared pass turns the same others in the same order; differs only if no other entry can turn |
| 149 | train/mod.rs:4274:17 | `solve_parts` | `delete field efficiency from struct flow::MeshFlow expression in solve_parts` | gap | **Path laws**. Break-away uses the running efficiency in place of the static one; the at-rest fixtures lock either way [lens-tests-train#2, T16.5] |
| 150 | train/mod.rs:4433:28 | `solve_parts`| `match guard sol.is_unique() → true` | equivalent | A driver that leaves a family implies the 'short' count below is nonzero, so the case is refused either way |
| 151 | train/shape.rs:98:5 | `default_pressure_angle` | `body → Auto::new(1.0)` | gap | **Documents read as documented**. Serde default for a member with no pressure_angle; gear-core's suite reads no such document (gear-io's change log promises 20°) [gear-io#8, T14.4] |
| 152 | train/shape.rs:597:45 | `Shape::absorbers`| `== → !=` | gap | **Plan order-invariance**. The leverage used to choose an absorber loses its a/b symmetry; the presets still pick the same absorber [shape-a#1, T10.5] |
| 153 | train/shape.rs:849:36 | `Shape::helix_angles`| `- → +` | gap | **Relief honoured and at a fixed point**. A crossed mesh with a given distance and both shifts pinned sizes its helix off the wrong wheel helix; no test gives a crossed distance that way [train-mod-a#8, T16.7] |
| 154 | train/shape.rs:957:65 | `Shape::size_reaching`| `* → /` | equivalent | Only moves where the bracket growth starts; the branch is monotone and brent finds the same root [primitives#13, T15.5] |
| 155 | train/shape.rs:1382:67 | `Shape::closed`| `/ → *` | equivalent | sign is ±1, so /sign and *sign agree |
| 156 | train/shape.rs:1442:46 | `Shape::absorb`| `* → +` | equivalent | The slope only steers newton_bracketed; the bracketed root is unchanged |
| 157 | train/shape.rs:1676:30 | `Shape::sized`| `*= → +=` | gap | **Shared bracket walk**. A tip-room walk that meets the involute domain's NaN floor grows its step instead of halving and reports the tips clear; no fixture reaches the NaN branch [primitives#13, lens-magic-numbers#11, T15.5] |
| 158 | train/shape.rs:1687:26 | `Shape::sized`| `*= → +=` | equivalent | Only the pace of the bracket walk changes; brent then finds the same single crossing [primitives#13, T15.5] |
| 159 | train/shape.rs:1807:25 | `Shape::chosen_at`| `\|\| → &&` | gap | **Shipped search against a scan**. A free member in two both-free meshes gets two search coordinates; no searched fixture chains three free members [auto-search#7, T16.9] |
| 160 | train/shape.rs:1849:49 | `Shape::chosen_at`| `/ → *` | gap | **Shipped search against a scan**. The division maps to x_b wrongly (factor 4); the search still returns an admissible point, just not the best [auto-search#7, ablate-features#1, T16.9] |
| 161 | train/shape.rs:2041:70 | `Shape::asking_members`| `&& → \|\|` | gap | **Search toggles stay local**. Every member counts as asking once any mesh asks: searching one mesh moves others [shape-a#2, T12.2] |
| 162 | train/shape.rs:2576:30 | `PointBuilt::rate`| `* → /` | gap | **Contact patch closes on its load**. The point patch's length (2a, capped at the line) is never checked [strength#8, T08.11] |
| 163 | train/shape.rs:2700:60 | `point_mesh_report`| `* → /` | gap | **Note firing and values pinned**. near_self_locking fires at µ > 0.8·threshold; no test sits between 0.8·threshold and threshold [added2#73, lens-magic-numbers#0, crossed-worm#16, T07.13, T15.9] |
| 164 | train/shape.rs:2756:21 | `point_mesh_report`| `* → /` | gap | **Report-field identities**. Point-contact sliding velocity is never checked against a closed form [T07.13] |
| 165 | train/shape.rs:2824:35 | `Shape::build_cached`| `> → <` | equivalent | Only the tooth cache is cleared early; every build is still correct (a work count would see it) [T16.8] |
| 166 | train/shape.rs:3529:49 | `rate`| `* → /` | gap | **Module similarity**. p_bn = π m cos α in the axial-float play term; masked at m=1 and zero axial float on parallel meshes [T16.3] |
| 167 | train/shape.rs:3648:43 | `rate`| `* → /` | gap | **Planet layout law**. Planet neighbour clearance 2r sin(π/n) − d_a is reported but no test checks it or clearance_ok [shape-b#0, T10.2] |
| 168 | train/shape.rs:3711:46 | `rate`| `+ → *` | gap | **Note firing and values pinned**. The bending_unrated_in_mesh note numbers meshes from 0 instead of 1; nothing reads the value [T16.20] |
| 169 | train/shape.rs:3873:29 | `rate`| `* → /` | gap | **Module similarity**. The worm's reference radius z·m/cos β/2 is right only at m=1 (the worm preset's module); it feeds the sliding velocity [crossed-worm#20, T16.3, T07.16] |
| 170 | train/wiring.rs:240:46 | `Wiring::paths_seen`| `== → !=` | gap | **Per-flank cycles**. Paths counted over every mesh of the part, not the member's own; the same unless a part mixes replicated and single meshes (a gear added at a set's sun) [shape-b#5, T10.13] |
| 171 | verify.rs:83:43 | `rack_travel_range`| `- → +` | instrument-only | verify.rs rack/ring gate. Travel-range bound; the padding and the other bounds cover the shifted end [T16.11] |
| 172 | verify.rs:137:38 | `check_cut`| `* → +` | instrument-only | verify.rs rack/ring gate. Copy spacing of the rack in the rack gate; the gate still passes its fixtures [T16.11] |
| 173 | verify.rs:170:21 | `check_cut`| `< → ==` | instrument-only | verify.rs rack/ring gate. Penetration is then never recorded: the rack gate's penetration half has no negative fixture [T16.11] |
| 174 | verify.rs:198:31 | `check_cut`| `* → /` | instrument-only | verify.rs rack/ring gate. Parabolic refinement is clipped by dmin.min(refined); the unrefined minimum already passes the limit [T16.11] |
| 175 | verify.rs:202:20 | `check_cut`| `- → +` | instrument-only | verify.rs rack/ring gate. As above: a worse refinement is discarded by the min with dmin [T16.11] |
| 176 | verify.rs:249:41 | `fillet_envelope_error`| `- → +` | instrument-only | verify.rs rack/ring gate. Path sampling density of the fillet-envelope cross-check [T16.11] |
| 177 | verify.rs:289:5 | `sdf_matches_polyline` | `body → -1.0` | instrument-only | verify.rs rack/ring gate. Cross-check returning −1 passes, since the test asserts only an upper bound [T16.11] |
| 178 | verify.rs:293:16 | `sdf_matches_polyline`| `* → /` | instrument-only | verify.rs rack/ring gate. Lattice top at module 1 is unchanged (m·x = m/x at m = 1) [T16.11] |
| 179 | verify.rs:326:27 | `sdf_matches_polyline`| `+ → *` | instrument-only | verify.rs rack/ring gate. Lattice coordinate; any near-field sample set still agrees [T16.11] |
| 180 | verify.rs:327:57 | `sdf_matches_polyline`| `/ → *` | instrument-only | verify.rs rack/ring gate. Lattice coordinate; any near-field sample set still agrees [T16.11] |
| 181 | verify.rs:415:35 | `ring_cut_envelope_spans`| `+ → *` | instrument-only | verify.rs rack/ring gate. Ring-gate cutter tip radius; the ring gate is replaced by T16.12's exact distance [T16.12] |
| 182 | verify.rs:452:26 | `ring_cut_envelope_spans`| `/ → *` | instrument-only | verify.rs rack/ring gate. Sampling of the cutter round in the ring gate [T16.12] |
| 183 | verify.rs:496:23 | `ring_cut_envelope_spans`| `+= → *=` | instrument-only | verify.rs rack/ring gate. Fold of the angle into one pitch; the following abs() and % cover the branch [T16.12] |
| 184 | verify.rs:631:39 | `inside`| `/ → %` | instrument-only | verify.rs rack/ring gate. inside() is the outline-placement oracle for loaded_flank_phase; tooth 0 alone still finds the contact |
