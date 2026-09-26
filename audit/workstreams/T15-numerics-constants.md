## T15 — Numerical hygiene and constants

**Why.** Most of the numbers the solve rests on are right. The literals around them are written by feel, written more than once, or given in the wrong unit, and the test suite cannot tell. Changing the tip-sizing lean from 1e-9 to 1e-8 mm leaves all 679 tests and all 32 golden outputs unchanged, and so do twelve perturbations of bracket-walk growth factors, caps and first steps [ablate-constants-geometry#8, ablate-constants-geometry#12]. The shipped clearance and tolerance band are absolute millimetres (0.02 mm ± 0.02 mm), so the default minimum backlash is exactly zero at every module [lens-standards#6]. Rounding a seed to 1e-4 in the core, when a box is pinned, turns the Compound preset's minimum backlash from 0.0 to −9.70e-5, a sign change no note reports [train-mod-a#15]. The fix is the same throughout. Each question gets one tolerance with a stated basis, and each default lives in one place in the core. A constant is derived from a closed bracket or a physical limit, or it is named with its reason and made visible to the user when it is a rule of thumb. Every surviving constant gets a law that fails when it moves.

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T15.1 Defaults defined once, in the core | lens-magic-numbers#3, wasm-boundary#7, graph-ops#14, added2#37, added2#24, lens-magic-numbers#4, lens-magic-numbers#6, graph-ops#11, gear-io#12 | low | S | — |
| T15.2 Give the default friction a source and a basis | lens-magic-numbers#1 | medium | M | T15.1 |
| T15.3 Length defaults in modules; backlash band as a named option | lens-standards#6 | medium | L | T15.1 |
| T15.4 One stopping rule in `solve.rs`, and a solve that returns a chosen side | primitives#4, ablate-constants-rating#14 | low | S | — |
| T15.5 `grow_bracket` and `bisect_predicate`; closed brackets where they exist | lens-magic-numbers#11, primitives#13, ablate-constants-geometry#8, ablate-constants-geometry#9, primitives#12 | low | M | T15.4 |
| T15.6 Tip sizing returns the clear side exactly (no 1e-9 lean) | ablate-constants-rating#12, added2#106, ablate-constants-geometry#12, lens-magic-numbers#14, lens-continuity#9 | low | S | T15.4 |
| T15.7 The rest of `shape.rs`'s sizing constants | shape-a#11, lens-magic-numbers#15, added2#104, ablate-constants-rating#10 | low | M | T15.5, T15.6 |
| T15.8 One seed rule for relief, shared with the panel | lens-magic-numbers#7, train-mod-b#10, train-mod-a#15 | low | M | — |
| T15.9 State the self-locking note thresholds; margin as an input | lens-magic-numbers#0, ablate-constants-rating#9 | low | S | T15.2 |
| T15.10 One tolerance per question: the shift floor and the pressure-angle guard | lens-magic-numbers#9, ablate-constants-geometry#10, primitives#10 | low | S | — |
| T15.11 Closed-form flank load normal; the ring's tip epsilon from the guard | ablate-constants-geometry#6 | low | S | — |
| T15.12 Flow tolerances: one named zero; solve in power units | lens-magic-numbers#8, ablate-constants-rating#11, lens-performance#10 | low | M | — |
| T15.13 Pointed-tooth roll through `inv_inverse` | primitives#8, ablate-constants-geometry#7, primitives#7 | low | S | — |
| T15.14 Identities said once: plane helpers, torque→force, contact-ratio note | primitives#9, lens-docs-accuracy-2#12, lens-magic-numbers#12, primitives#15 | low | S | — |
| T15.15 Edit tooth floors from the fit, not 4 / zp+2 / 6 | lens-magic-numbers#13, lens-errors-policy#17 | low | S | — |
| T15.16 Crossed path: Gauss–Legendre instead of 2048 trapezia | crossed-worm#18 | low | S | — |
| T15.17 Laws for constants nothing reads: line patch width, the hula gap | ablate-constants-rating#6, ablate-constants-rating#8 | low | S | — |
| T15.18 `centre_profile` has no exact-zero branch | wasm-boundary#13 | info | S | — |

### T15.1 Defaults defined once, in the core
**Change.**
- The load-case figures land as T13.6's `LoadCase::fresh(kind)`: ultimate (0.1 N·m, 30 000 rpm) and fatigue (0.02 N·m, 30 000 rpm, the torque written as ultimate/5, as the comment at lib.rs:1175 says). Rewrite lib.rs:1165's comment in the present tense.
- Add `Duty::continuous()` beside `Duty::intermittent`, with a named hours constant documented as a placeholder seed, not a design life. Use it in `set_duty` (conditions.rs:1161) and at the gear-cli and gear-io sites that write `runtime_hours: 1000.0`. Fix the docs of `set_duty` and of `edit_train` (lib.rs:1527): a fresh case is intermittent, so the continuous figure is not "a fresh case's".
- Build `MemberGear::default` (train/mod.rs:1300) from `GearParams::default()` for teeth, addendum, dedendum and root radius, as shape.rs:97 already does for the pressure angle.
- In `arrangements.rs`, delete the six no-op assignments in `pair()` (674–676) and `planetary()` (757–759). Name `push_distance`'s fallback as `RUNNING_CLEARANCE_MM` / `DISTANCE_TOLERANCE_MM` beside `FRICTION`, and point the doc comments at lines 33 and 452 and `pair()`'s doc to them.
- Use `REFERENCE_CIRCLES_BY_DEFAULT` at lib.rs:1161.
- Replace "1.2" in `ui.train_note_min_contact_ratio`, in all five catalogues, with a `{default}` placeholder filled from `DEFAULT_MIN_CONTACT_RATIO`. Say that the default follows the practice rather than restating the number.
- Leave `UI_SEED` (5 mm) alone: it is argued and held by a test.

**Proof.**
- A test that `set_duty(i, false)` yields `Duty::continuous()`.
- A test that `MemberGear::default`'s rack equals `GearParams::default`'s.
- `check_golden`, `check_wasm.sh` and `check_strings.py` show no diff. That no diff appears is the proof the arrangement lines were no-ops.

### T15.2 Give the default friction a source and a basis
**Change.** Rename `FRICTION: (0.08, 0.16)` (arrangements.rs:30) to `DEFAULT_FRICTION`. Cite a source for the lubricated sliding μ. Argue the static/sliding ratio of 2 in `rationale.md#two-friction-coefficients`. State that the preset worm's self-locking verdict depends on that ratio: at lead ≈ 8.21° the backward threshold is ≈ 0.136, which static 0.16 exceeds and sliding 0.08 does not. In the panel, mark the value as estimated, as material values are.

The target is a friction that is a property of the material pair, carrying a `Basis` and extended by the materials law. That changes the mesh's input model and belongs with T07.13 [lens-standards#0, lens-standards#1], so do the naming and documentation first.

**Proof.** The materials law ("every non-datasheet value carries a note") is extended to the friction default and fails today. `check_doc_links.py` passes.

### T15.3 Length defaults in modules; backlash band as a named option
**Change.**
- Express `min_tip_width` (mod.rs:1306, 0.1 mm) in modules.
- Express the running clearance and tolerance band (`push_distance`, 0.02 mm ± 0.02 mm) in modules, or give the clearance an automatic value from ISO/TR 10064-2's j_bn,min = (2/3)(0.06 + 0.0005·a + 0.03·m_n): 0.070 mm for the default m = 1, a = 30 pair, 0.190 mm at m = 5, a = 150. That table targets coarse pitch (m_n ≳ 1.5), so at m = 1 it is a rule of thumb. It must appear as a named, user-visible option with a note, not as a silent default.
- Keep `default_planet_clearance` (0.3 mm) absolute: it is a physical gap.
- The standard way to take backlash off a fixed housing is a thickness allowance on both gears. That needs lifting the k1 + k2 = 2 rule (state.md:764–770). Its backlash is closed form, j_t = (2 − k1 − k2)·πm/2, and the tips already read x + x_s. Coordinate with T21.3 and T10.1 [lens-feature-gaps#3, mesh-contact#0]. The absolute 1.75 mm pin default is T09.7 [metrology#14].

**Proof.** A scaling law over explicitly scaled inputs (every length ×s, including `face_width`) scales every length output by s. It fails today because of the mm defaults. A test that the default pair's minimum backlash is ≥ the stated option's value, where that option is on. Today it is −0.00000.

### T15.4 One stopping rule in `solve.rs`, and a solve that returns a chosen side
**Change.** State the rule once and make both solvers read it the same way. Today Brent stops at a bracket ≤ x_tol + 4ε|b| and Newton at a step < x_tol + 2ε|x|. The 1e-15 absolute floor governs for |x| < 2.25, so Brent's relative error on a root of 1e-9 is 1.8e-7. Either:
- (a) correct the `Tol` doc and reference.md#root-finding to the rule actually implemented, or
- (b) have each call site pass its variable's natural scale, x_tol = ε·scale (the module for a length, 1 for an angle).

Do not use 4ε·max(|b|, |lo|, |hi|): it loosens the pointed-roll solve about 40×. Keep `max_iter` settable, because the exhaustion test depends on it. Give `hertz.rs:433` `Tol::default()`: its 1e-14 has no stated reason and a tenfold change moves nothing.

Add `brent_side(f, lo, hi, tol, want_nonneg)`, which returns the end of Brent's final bracket on the requested sign. The bracket (b, c) is already held, so this costs no extra evaluations. T15.6 uses it.

**Proof.** Extend `a_large_root_converges` to roots from 1e-9 to 1e6: under (b), relative error < 8ε at every scale. It fails today at 1e-9 and 1e-6 for Brent. A unit test that `brent_side` returns an end with f ≥ 0, and that the end is within one stopping width of the root. `check_golden` may show last-digit moves; review them.

### T15.5 `grow_bracket` and `bisect_predicate`; closed brackets where they exist
**Change.** About ten sites hand-write bracket growth or predicate bisection, each with its own factor (1.4, 1.5, 1.6, 2) and cap (16, 40, 60, 64, 80, 200).

First, remove the walks that a closed bracket makes unnecessary:
- Tooth walk 1 (tooth.rs:591–603) brackets on [−rb, 0], because the trochoid radius is ≥ |s|. Tooth walk 2 brackets on [s(ra), s_b].
- The ring junction (ring.rs:540–549) brackets on [−(r_j + a), 0].

This deletes `BASE_CROSS_GROWTH`, `CROSSING_GROWTH`, `MAX_STEPS`, the ring's 1.4, its 64 and its −m_t start. The closed tooth bracket reproduces the walked junction to 1.6e-11 on 143,138 cases. T05.10 proposes a closed form for the ring junction [ring#11]; if it lands first, it supersedes this.

Then add two helpers to `solve.rs`:
- `grow_bracket(f, from, first_step, mode, limit, budget)`. `mode` is additive or geometric. It halves back on a non-finite value, which `shape.rs` needs. It stops at a caller-stated physical limit, and at an evaluation budget, because the predicates are expensive.
- `bisect_predicate(pred, lo, hi)`. It stops at `mid == lo || mid == hi || hi − lo ≤ ε·|hi|`, which is at most about 53 + log2(hi/lo) steps. It must not run to pure float adjacency, which takes about 1,100 subnormal steps when the boundary is at 0.

Migrate `metrology.rs:190/200`, `gear.rs:1119`, `verify.rs:726`, `hertz.rs:418` (whose `MIN_POSITIVE` floor becomes its `limit`) and `auto.rs:137–147`. In `auto.rs`, a failed bracket returns `None` or a note instead of `.unwrap_or(x0)`. Its bracket cannot be the admissible shift range, which misses x_min in 2,421 of 5,120 probed gears. Keep its walk and write down the termination argument.

Fix metrology's doc: it says "sixty halvings of a bracket a few modules wide", but the code does 64 halvings of [0, rb·2^k]. The same slip is T18.18 [added2#63].

In `shaper::largest_tip_round` (shaper.rs:432–450), solve the raw corner angle on [floor, r_ac − r_bc]. It is continuous and monotone there, with dθ/dρ = −(1 − sin α_g)/r_bc, so use `newton_bracketed`, and exit before any solve when the asked round fits. Do not return NaN outside the domain: every ring would lose its fillet.

**Proof.**
- Unit tests of both helpers on monotone, NaN-holed and unbracketable functions, and at 1e±300.
- A law over the `tests/common` grid: the closed-bracket root equals today's walked root within 1e-10.
- A counting-closure test: `largest_tip_round` takes ≤ 15 evaluations, against 44–76 today.
- `check_golden`, reviewing any last-digit moves.

### T15.6 Tip sizing returns the clear side exactly (no 1e-9 lean)
**Change.** Replace `hi.min(e + 1e-9)` and its comment (shape.rs:1693–1702). The comment calls the lean "the solver's own tolerance"; the solver's tolerance is about 1e-14 mm. Use `brent_side(room_at, lo, hi, …, want_nonneg = true)`. `hi` is always the clear end in both walks. Leaning by tol1 does not guarantee room ≥ 0, because `room_at` carries rounding noise. If `brent_side` is not taken, step with `next_up` until `room_at ≥ 0`, bounded by `hi`.

**Proof.** In shape.rs's test module (`tip_room` and `sized` are private), a law over every preset and hula fixture with an asked clearance: `room_at(held[d]) ≥ 0` exactly. The far-side gap equals the asked gap to 1e-12 where the gap binds. At 19/18/17/18 with a 0.3 mm gap, the built gap is 0.30000000328 today, which fails that law. Then the M58-style mutation (lean ×10) has nothing left to mutate.

### T15.7 The rest of `shape.rs`'s sizing constants
**Change.**
- Share one `AGREE_REL` between `closed` (1404) and `build` (2872), which both use 1e-9·max(|p|,1).
- In the re-size loop (1914), compare held distances in mm against `resolution × module` of the distance's mesh, not the dimensionless `resolution·1e-3`. Replace the fixed 3 rounds with a named `MAX_ROUNDS` and its reason, or loop until nothing moves. Raise a note if the cap binds: today the loop exits silently with divisions from the previous distance.
- Delete the dead ±5-module shift fallback (1479–1480): `admissible_profile_shift` always closes both sides, so make `ShiftRange::bound` a closed pair. The same dead code is T17.9 [ablate-constants-rating#2].
- Name the 89° helix-bracket top (863). When `at(89°) < target`, report the helix limit instead of letting `NoContact` surface later: today a target 58× the straight distance fails as `InPart{NoContact}`.
- Use one named relative inset for the diameter floor at 928/955/957 (today `1.000_001` and `1 + 1e-9` for the same edge), so the manual clamp and the bracket agree.
- Check whether `room_at` is finite at the exact base-circle floor (1620); if it is, drop the `1 + 1e-6`. If it is, `grow_bracket` can also replace the 40/80-step walk at 1645/1667.

**Proof.** A law over every preset and hula fixture: at exit, re-sizing at the chosen divisions moves the distance by less than resolution × module. A test with a target beyond 57.3× the straight distance expects the named helix-limit refusal. `check_golden` shows no diff.

### T15.8 One seed rule for relief, shared with the panel
**Change.** Three sites seed with `(v·1e4).round()/1e4` (mod.rs:2359, 3661, 3664). A fourth copy is `toFixed(4)` at TrainPanel.svelte:1814. This rounding is absolute for every unit, so 7.46e-5 N·m seeds as 0.0001 (+34 %), and pinning a box moves quantities the designer did not touch.

Recommended: seed the exact figure, and have the panel format every box, given or automatic, at one display precision the core exports as a named significant-figure count. That changes the recorded intent "the digits they saw" and is the owner's call. The alternative keeps rounding, but to significant figures in one Rust `seed()` helper, with the panel formatting to the same count. The display side is T19.7 [web#9]; change both together.

**Proof.** A law over every preset and toggle: `relieved_from` followed by a solve reproduces every figure to 1e-12 relative (exact seeding), or to 1e-6 (significant figures), across torques from 1e-6 to 1e6. It fails today on Spur, Compound, Worm and Crossed (the Compound minimum backlash changes sign). `a_box_relief_pins_is_seeded_from_its_figure`'s 0.1235 becomes the exact value.

### T15.9 State the self-locking note thresholds; margin as an input
**Change.** In `point_mesh_report` (shape.rs:2700, 2709):
- Name `LOW_EFFICIENCY = 0.5`. It is definitional: η_back = 2 − 1/η_fwd at the pitch point.
- Name the near-locking margin 0.8, whose value nothing argues. Frame it as the allowance for uncertainty in static μ and make it a user-visible input with default 0.8, per the owner's rule on rules of thumb.
- Pass the margin into the note as `{margin}` in all five catalogues.
- Document both next to "locked backwards" in reference.md, including that the efficiency note reads sliding μ while locking reads static μ. That is why the two notes can disagree near the line.

Do not derive the margin from the sliding/static spread: static ≥ sliding, so no band falls out. This task lands inside T07.13, which carries the worm side's note thresholds [crossed-worm#16, added2#73].

**Proof.** Boundary tests with static μ at 0.79 and 0.81 of the threshold, and with η_fwd just either side of 0.5. They fail under today's recorded perturbations (0.8 → 0.88, 0.5 → 0.55), which the suite does not notice. Today crossed.rs:1079 accepts either note. `gear_io::strings` passes with the new placeholder.

### T15.10 One tolerance per question: the shift floor and the pressure-angle guard
**Change.**
- Use `compat::SAME_SHIFT` (1e-9) in place of 1e-12 at auto.rs:1021 (`member_is_buildable`) and at 1507 (the search's pre-filter), matching tooth.rs:481's undercut flag. Reword `member_is_buildable`'s doc: the two tests coincide only when floor = x_min. Leave mod.rs:1255, which is a separate report flag.
- The pressure-angle guard lands as T01.5, which carries its readers, its policy and its proof. This task is the shift floor alone.

**Proof.**
- A test at x = x_min − 5e-10: `tooth.undercut` is false and `member_is_buildable` is true. It fails today.

### T15.11 Closed-form flank load normal; the ring's tip epsilon from the guard
**Change.** In `flank_point_and_load_direction` (strength.rs:376) and its ring twin (ring.rs:713), return the exact normal: [cos(ψ_b − u), −sin(ψ_b − u)] for the external tooth, [−cos(ψ_b + u), sin(ψ_b + u)] for the ring. This removes the hypot, the `MIN_POSITIVE` test and the wrong [1, 0] fallback at u = 0 (unreachable today, but a 14 % jump in the bending factor). It also removes cancellation noise of 2e-5 at u = 1e-12. Replace ring.rs:282's bare `1e-9` with `guard::TIP_ABOVE_BASE_FRACTION`.

Every ring with z ≤ 33 at default proportions sits on that clamp, and its rating moves by about 3e-5 per decade of the epsilon (u_tip = √(2ε)). An epsilon-free rating needs the ring tip clamped at exactly rb. Check the other consumers of u_tip (flank curvature, contact path, outline) before doing that.

**Proof.** A law for an external tooth and a ring: the bending factor at u = 0 equals its right limit to 1e-9 relative. It fails today (1.0088 against 1.1678). A law that the closed normal matches the old one to 1e-12 over u ∈ [1e-6, 1]. Do not assert that ε ×10 moves no ring rating; that cannot hold while u_tip = √(2ε).

### T15.12 Flow tolerances: one named zero; solve in power units
**Change.**
- Make `flow::ZERO` `pub(crate)` and use it in groupings.rs:242 in place of its own 1e-9. Both are normalised by input power.
- Remove the `fold(1.0, max)` floor from the consistency check (flow.rs:394–401) and use one named relative tolerance. 1e-9 is defensible; 64ε is too tight once the residual carries n·ε·cond.
- The absolute pivot floor 1e-12 (flow.rs:366) refuses forward flows through long reductions: 10/70 × 17 stages (smallest pivot 4.2e-13) is refused as `train.load_shared`, while the backward direction solves. The cause is partial-pivot fill-in in torque units: cond 1.9e14, against 19 when the same system is scaled to mesh power. A column-relative pivot does not cure it. Scale both sides to power units: unknown p_k = c_k·z_a·(ω_a − ω_frame), known rows multiplied by their speed where it is non-zero, and bodies at rest left in torque units. The entries become ±1 and ±η, and one relative tolerance then means something. A seeded, triangular per-path solve is T11.4 [lens-performance#0]; the kinematic-exactness angle is T11.15 [kinematics-flow#7].

**Proof.**
- A law: a chain of n spur pairs, n ≤ 20, solves forward at η^n. It fails today at n = 15 (10/100) and n = 17 (10/70).
- A law: scaling every known torque by 1e±12 leaves every efficiency and direction assignment unchanged. Include an over-determined case (input and output loads both given), since today's probe only reaches determined systems.
- A law over the presets: every mesh the groupings call idle has |power| ≤ ZERO·P_in.
- Pennestrì on every arrangement still passes.

### T15.13 Pointed-tooth roll through `inv_inverse`
**Change.** At tooth.rs:455 (a bracketed Newton) and auto.rs:743 (Brent), use `crate::involute::inv_inverse(psi_b).map(f64::tan)` and delete `POINTED_TOOTH_MAX_ROLL`. The same duplication is T03.7 [lens-magic-numbers#10]. Alternatively, let `Tooth` keep the pointed radius it already solves, so `addendum_for_tip_width` does not solve again. Correct reference.md:251, which calls this solve "a bracketed Newton" (T18.18 [added2#102]).

Optionally, seed `inv_inverse` with the 5-term Lagrange series (y = (3v)^{1/3}; coefficients −2/15, 3/175, −2/1575, −16/202125), and above 1.2 rad with the atan fixed point. That saves 1–3 Newton steps. Keep `inv` as tan α − α: the proposed 6-term Taylor switch at 0.25 rad is 1.6e6 ulp worse there, and no caller reads relative accuracy.

**Proof.** A law: at the pointed cap, tooth ra equals rb/cos(inv_inverse(ψ_b)) to 1e-14 relative (measured 5.2e-16 over 2,268 teeth). If the seed changes, add an absolute accuracy law: |inv_inverse(inv α) − α| ≤ 4ε·max(α, 1). Not "2 ulp everywhere", which is unreachable below 0.3 rad. `check_golden`, reviewing ulp-level moves.

### T15.14 Identities said once: plane helpers, torque→force, contact-ratio note
**Change.**
- screw.rs:1471: use `plane::transverse_pressure_angle`.
- screw.rs:869 and shape.rs:3529: use `plane::base_pitch`. shape.rs:3532: use `mesh::angular_play(slide, 1, p_bn)`. All three replacements are bit-identical.
- Add `plane::transverse_module` for the eight `m / beta.cos()` sites.
- Add a raw `plane::transverse_thickness` for tooth.rs:834 and auto.rs:357/678, and let tooth.rs's guarded version clamp it. Review ring.rs:246/358, which use a different product order, under `check_golden`.
- Add `plane::force_from_torque(torque_nm, arm_mm)` for the five N·m→N·mm sites (strength.rs:1267, 1279; contact.rs:763; screw.rs:527, 528).
- Raise `MESH_CONTACT_RATIO_BELOW_ONE` from one helper that takes the deciding ratio (train/mod.rs:471, shape.rs:2732). Its helical wording is not settled, so do not cement ε_α.
- Drop reference.md:65's dated "six sites" count.

**Proof.** A new plane.rs test for tan β_b = tan β·cos α_t. `check_golden` shows no diff, except under review at ring.rs. No grep lint: `/ beta.cos()` has legitimate uses.

### T15.15 Edit tooth floors from the fit, not 4 / zp+2 / 6
**Change.** In edits.rs (417, 419, 429, 491), size the central gear from the fit and let the solve decide. Refuse `NoRoom` only where the fit gives no count ≥ 1 of the right kind. For a ring, use the fit itself, never max(fit, zp + 2). If a sun floor is kept, derive it: the smallest count with contact against the mate at its automatic shift. `preview.rs`'s dry run reports why an edited train does not solve. Write the guard NaN-safe (`!(z >= 4.0)`, or a finiteness check) and cap z at `u32::MAX` before the cast. Fix `carrier_radius`/`central_teeth` to use transverse modules. Unpinned edit-sizing constants are also T16.29 [edit-ops#11].

**Proof.** An edit law: every count refused as `NoRoom` is outside `admissible_ranges` or fails to solve, and every accepted ring solves. It fails today both ways: pairs 3/100 solve but are refused, and planocentric(18,19) accepts a 20-tooth ring that fails with `NoCommonDistance`. A test that `AddGear` at module 1e300, helix 90° is refused, never a 0-tooth member.

### T15.16 Crossed path: Gauss–Legendre instead of 2048 trapezia
**Change.** Replace `PATH_SAMPLES = 2048` (shape.rs:2645) and `SEARCH_SAMPLES = 256` (auto.rs:1591) with one fixed Gauss–Legendre rule of 8–16 nodes. On the probed fixtures, GL8 reaches 1e-14 against trapezium-2048's 3e-10, and each locking Brent costs 1.36 ms today. Point both docs to the convergence test's real home, screw.rs's tests. Drop the exact-90° branch in `least_distance_lead_angle`, or keep it: it is continuous, and the closed form is better as a test oracle (T07.18 [ablate-constants-geometry#14]). The `mesh_branch` 1e-6 residual and `solve3`'s |det| < 1e-12 go with T07.6's closed-form line [crossed-worm#3]; a wider tolerance only moves the noisy band.

**Proof.** Restate `the_path_average_has_converged` as GLn against GL2n, both below a stated tolerance. The typing-speed test passes. Corpus digits finer than convergence are T20.8 [ablate-constants-rating#17]; re-record once.

### T15.17 Laws for constants nothing reads: line patch width, the hula gap
**Change.**
- Line patch width: beside strength.rs:2812's `contact_stress_matches_the_half_width_route`, assert `ContactPatch::line(&cs, 1.0, b, e_star).patch_width/2` equals that test's independent half-width to 1e-12. Optionally print the line patch in `gear-cli train`.
- Hula gap: rewrite the `hula` doc (arrangements.rs:482/512). At the shipped 65/61/57/61 the tips' crossing sizes the crank (2.064116008 mm for any asked gap up to 2.5 mm; as-built far gap 2.76 mm). The 0.3 mm binds only at one tooth of difference, just above the tips' own 0.278 at z = 18.
- In `a_four_tooth_planocentric_is_sized_by_its_tips` (arrangements.rs:1191–1226), assert which room bound. Move the equality with the asked gap to a one-tooth fixture, and correct its false comment. The tip-foul inside the tolerance band is T05.3 and T10.10 [added#66, added2#117].

**Proof.** The patch law fails under the recorded 2.0 → 2.02 perturbation. At 19/18/17/18 the running distance strictly increases with the gap above 0.278 and the far gap equals the asked value; at 65/61/57/61 it does not depend on the gap below 2.5.

### T15.18 `centre_profile` has no exact-zero branch
**Change.** Delete lib.rs:939's `angular_shift == 0.0` refusal and the `ui.gear_concentric_has_no_profile` key in all five catalogues, and return the flat profile, as the core already does at 1e-300. Check `GearPanel.svelte:183` (`sizeByThrow`), which seeds from defaults when the profile is unavailable. `amplitude_for_throw(0)` must return 0.

**Proof.** Change the wasm test at lib.rs:2155 to assert amplitude 0 and range[0] == range[1] at zero shift. A continuity test: 0 and 1e-300 give identical answers. `check_wasm.sh --write`, then `check_strings.py`.
