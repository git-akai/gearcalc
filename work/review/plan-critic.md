# plan-critic — adversarial review of audit/plan.md and T01–T21

Branch audit-ablation @ 03f9823. Read: plan.md, all 21 workstreams, mutation.md §1–2. Code spot-checked read-only (no worktree needed).
Programmatic checks: ledger coverage, roadmap coverage, phase-vs-Needs ordering (script output quoted below).
IDs are PC-n. Severity is for the plan (how badly it misleads the implementer).

---------------------------------------------------------------------------------------------------
## 1. Proposed changes that violate the owner's goals
---------------------------------------------------------------------------------------------------

**PC-1  T05.2 adds a special case and a sign-valued (discontinuous) parameter.** ring.rs/shaper.rs:249/304/357.
Claim: T05.2 stores κ = sign(r_g − r′_c) ∈ {−1,0,+1}, uses k = 1 + κρ/d, and says "Handle κ = 0 … as its own case". That is a new branch and a parameter with a jump, at exactly the prolate/curtate transition the task is trying to make continuous (its own proof asks for continuity through 30/20, x≈0.683).
Evidence: T05.2 text; T03.14 already shows the singularity-free route (parameterise the corner by its normal angle φ, travel linear in φ, "correct branch whatever the sign"). T03.13's RollingCorner is written with continuous sinc/versine helpers.
Fix: write the shaper corner once in the φ (normal-angle) form of T03.14/T03.13 so d = 0 is an ordinary point; drop κ-as-sign and the κ = 0 case. Land T05.2's closed-form junction inside that form.
Severity medium. Confidence medium-high.

**PC-2  T05.4 introduces a new named cap with no basis.** ring.rs:363-373.
Claim: "clamp x … to a margin above x_lim. Derive that margin from a minimum α_w named in guard." A minimum operating angle is a new convention constant; the task gives no value or derivation (it only says α_w = 0 exactly "reads 0.02–0.04 mm off the cut", a symptom of the cut simulation, not a physical limit).
Fix: clamp at the involute-domain edge itself (inv α_w = 0 is closed form), and state the cut-gate residual there as the known size; or make the margin a derived quantity (e.g. the α_w at which the generated flank shrinks below one sampling step). No new guard constant.
Severity medium. Confidence high (text-level).

**PC-3  T05.7 replaces one display rule of thumb with another and ignores the rim input that exists.** ring.rs:586-588.
Claim: rim = rf + 2·m_t ("two modules of material") is a new unsourced constant. The rating already reads `MemberGear::rim_thickness` (train/shape.rs:3224) — which CLAUDE.md says the panel never reads — so the drawn rim and the rated rim can disagree.
Evidence: ring.rs:587 `self.r + 2.0 * self.mt`; shape.rs:3224 reads rim_thickness.
Fix: draw the rim at rf + rim_thickness when given; when not given draw none (or label it as a display extent, not material). One source for "the rim".
Severity low-medium. Confidence high.

**PC-4  T06.7 keeps a 33 % step at ε = 2 and calls it physical.** contact.rs:869-925.
Claim: "Do not force continuity: the single-pair zone really vanishes there." The step comes from the ramp's endpoint weights (1/3, 2/3) — RAMP_MAX = 2/3 is reached at the plateau edge, so as the plateau shrinks to a point the peak share drops 1 → 2/3. A compliance model (T21.13) gives a continuous peak share. This is exactly "a value becoming a different value across a boundary the physics does not have" (the code's own comment at contact.rs:910-916).
Evidence: read load_share; computed shares at ε = 2+δ after T06.5's normalisation: pair at d=1 gets (2/3)/(2/3+1/3) = 2/3.
Fix: make the ramp weights depend on the plateau width so the peak share → 1 as ε → 2⁺ (continuous), or make T21.13's compliance weight the default and demote the ramp to a named option; in either case state the remaining bias in state.md.
Severity medium. Confidence medium.

**PC-5  T11.4 builds two algorithms (seeded fixed point + subset enumeration fallback) where one exact one suffices.** train/flow.rs:223.
Claim: T11.4's evidence says the iteration alone fails in 7,113 compound and 2,115 Ravigneaux cases, so the fallback is load-bearing — the production solve would be iteration + a 2^|L| enumeration + a cap, and the oracle a third copy. Step 1 alone (split into components sharing no unknown torque) already turns 2^ΣM into Σ2^{M_i}; every shipped part has M_i ≤ ~4, and the 16-mesh planetary chain decomposes into 2-mesh components.
Fix: make step 1 (+ a block-cut/series ordering of components, lens-architecture#7) the design, keep exact per-component enumeration as production (it is then its own oracle), keep T11.1's cap only per component. Drop steps 2–4 unless a measured component exceeds the cap.
Severity medium. Confidence medium (design judgement; the decomposition claim is from T11.4's own text).

**PC-6  T12.7/T12.8/T12.10/T12.12 stack four heuristics on the search instead of using its structure.**
Claim: T12.7's violation residual mixes units by a chosen normalisation ("mm over module for gaps, raw for contact ratio") and its fallback point depends on that choice; T12.8 adds a resolution-sized margin (1e-3) tuned to the panel's 4-decimal display; T12.10 step 3 swaps the grid for a "fixed-size low-discrepancy sample"; T12.12 adds a tie tolerance. The objective is a product of per-mesh η, each depending on one mesh's two shifts, coupled only through shared members and held distances; constraints are closed-form in x (T12.11 already exploits this in 1-D).
Fix (redesign R3 below): per component, exact 1-D solves on held distances (T12.11 generalised), and DP over shared members for chains (T12.10's own DP remark), with constraint edges found as bracketed roots. Then T12.7's walk, T12.8's margin and T12.10's sampling are unnecessary; a user-visible tie tolerance (T12.12) may remain.
Severity medium. Confidence medium-low.

**PC-7  T04.3's stop bound degenerates at the involute cusp; T04.7's law then fails at x = x_min.** outline.rs subdivide.
Claim: split while L²/(8ρ_min) > tol with ρ = r_b·u. At the undercut threshold (u_j → 0) ρ_min → 0, the bound is infinite and the first span always runs to MAX_SUBDIVISION_DEPTH, while the true deviation there is tiny. The search floors shifts at x_min (T12.6), so this is a common design point, and T04.7's law "at the floor, no span ends on depth" fails there. Also: c²/(8ρ) is the small-angle sagitta, a slight under-estimate, not a bound; "ρ monotone on each section" is asserted, not shown, for the trochoid.
Fix: bound the chord error by the tangent-turning angle Δφ over the span (deviation ≤ (L/2)·tan(Δφ/2)); for the involute Δφ = Δu exactly, for the trochoid it is the difference of closed-form tangent angles. Finite at the cusp, and a true bound.
Severity medium. Confidence medium-high.

**PC-8  T08.11 ships an unsourced interpolation as the production contact model.** hertz.rs:290-301.
Claim: p = p_e/√(1.5c − 0.5c³) is not a published result; its only support is a new FFT tool showing it sits 0.1–3.8 % above. Owner rule: rules of thumb only as visible options, with bias recorded. Also T08.11 leaves hertz.rs:300's `.map_or(0.0, …)` (a failed ellipse reads 0 pressure) unaddressed.
Fix: either keep max(line, ellipse) and record its measured 5.9 % unconservative dip in state.md with sign, or adopt the formula only with a state.md bias entry generated by the FFT tool; make the ellipse failure an Option, not 0.
Severity low-medium. Confidence medium.

**PC-9  T08.6's "C(κ) comes in closed form from the Hertz field" is not true.**
Claim: onset of subsurface yield for an elliptical contact needs a 1-D maximisation over depth of von Mises built from elliptic-integral stress expressions; only the two limits (1.79 line, 1.60 circle, ν = 0.3) are closed-form numbers. The plan otherwise forbids unstated numerical procedures.
Fix: state it as a bracketed 1-D maximisation in solve.rs (listed in T18.18's inventory) or tabulate with a stated interpolation error; keep the two limits as the test.
Severity low. Confidence medium.

**PC-10  T13.4 silently moves a user's load to a guessed body.** conditions.rs:341-355.
Claim: "If ambiguous, it goes to chain_ends().1" relocates a load entry by a heuristic. That is a hidden rule of thumb acting on inputs (rule 3: inputs are the only state).
Fix: re-seat only when the join body is unique; otherwise park the entry with a note (the task's own third branch) — no chain_ends guess.
Severity low-medium. Confidence high.

**PC-11  Guard "conventions" stay hidden rules of thumb; one is never mentioned by the audit.** params.rs:225-270.
Claim: params.rs's own doc says the first five guard constants are **conventions** that "could be widened without admitting anything impossible" and that they silently steer the search optimum: MIN_CUTTER_DEPTH 0.05, MAX_CUTTER_DEPTH_FRACTION 0.9, MIN_TOOTH_THICKNESS 0.02, MAX_TOOTH_THICKNESS_FRACTION_OF_PITCH 0.95, FILLET_FRACTION_OF_MAX 0.95 (+ MIN_PRESSURE_ANGLE 0.5°). No task makes them user-visible options or derives them; T03.3 and T12.14 build on 0.95 and 0.05 as given. MAX_TOOTH_THICKNESS_FRACTION_OF_PITCH appears in no ledger finding and no task (grep: 0 hits).
Fix: one task: each convention becomes a named, user-visible search/cut policy input with its default and reason (or is derived — e.g. the fillet 0.95 margin from the root-arc sampling it protects), with a both-sides law each. NEW.
Severity medium. Confidence high.

**PC-12  Smaller constant/threshold issues retained by tasks.**
- T10.17: new member input b_min/m_n "default computed in Rust" — no basis given for the value. Fix: name the source or default to 0 (no floor) with the note.
- T10.2: "refuse a longer cycle … on its closure residual" — tolerance unspecified; state it relative to the distances (one AGREE_REL, T15.7).
- T02.6: exact-zero root radius "keeps its silent 1e-9·m floor" — MIN_FILLET_MODULES stays silent; fine physically, but say so in the guard doc and pin it with a law.
- T15.3 keeps the 0.02 ± 0.02 mm band absolute, so lens-standards#6 (min backlash exactly 0 at every module, listed in P9) stays uncured until the optional Phase-6 T21.3. See PC-33.
- T16.9 sets SLACK "from the measured wall error" — a threshold from where a sweep stopped (CLAUDE.md testing rule 3).
Severity low. Confidence high.

---------------------------------------------------------------------------------------------------
## 2. Redundant, conflicting and mis-ordered tasks (beyond plan.md's merge list)
---------------------------------------------------------------------------------------------------

### Direct conflicts (two tasks, incompatible edits to the same code)

**PC-13  Tip at/below the base circle: four incompatible policies.** tooth.rs:471, ring.rs:282, auto.rs:671.
- T02.6: refuse through the gate; retire `clamp.ring_tip_raised`; use TIP_ABOVE_BASE_FRACTION at ring.rs:283.
- T03.4 (Phase 1): delete the tooth's base floor, floor at the root only, raise `clamp.tip_below_form` — a fillet-only tooth "can still be cut".
- T05.15 and T15.11: keep the ring's ε-clamp, just rename the literal.
- T16.23: pin addendum.min = max(−h_f, (r_b(1+TIP_ABOVE_BASE_FRACTION) − r)/m − x) and assert ra = r_b(1+f) at lo−ε.
- T05.13: ring lower bound from "the ra_min clamp".
Phase 2 (T02.6) would refuse what Phase 1 (T03.4) makes legal, and T16.23 pins the formula T03.4 deletes.
Evidence: tooth.rs:471 `ra.max(rb*(1+guard::TIP_ABOVE_BASE_FRACTION))`, ring.rs:282 `rb*(1.0+1e-9)`, auto.rs:671.
Fix: decide once (T03.4's clamp-with-note is rule 5 and the continuous choice, for both kinds), then rewrite T02.6's first bullet, T05.13, T05.15, T15.11 and T16.23 against it; TIP_ABOVE_BASE_FRACTION then disappears. Severity high (plan-breaking). Confidence high.

**PC-14  T08.2 vs T17.7: bending_factor returns None vs returns f64.** strength.rs:1046, 1097.
T08.2 (Phase 1): `bending_factor(DolanBroghamer)` returns None when the factor or moment arm ≤ 0 ("stress_correction's contract already allows None"). T17.7 (Phase 4): make stress_correction/bending_factor/bending_stress return f64, delete the None docs and the CLI branch. Phase 4 would revert Phase 1's domain fix. Evidence: both currently `-> Option<f64>`, every arm `Some` (stress_correction), moment_arm ≤ 0 makes `by_height.powf(m)` NaN. Fix: drop T17.7's first bullet (the Option becomes meaningful after T08.2). Severity high. Confidence high.

**PC-15  T07.6 vs T17.4: the pitch-point sliding closed form.** screw.rs:246-249.
T07.6 replaces `sqrt(1 − 2k cos Σ + k²)` with |sin Σ / cos β₂| (cancellation-free); T17.4 says "Keep screw.rs's closed form √(1 − 2k cos Σ + k²)" as the independent check. Fix: keep |sin Σ/cos β₂| in production, the law-of-cosines form in the test. Severity medium. Confidence high.

**PC-16  T11.15 vs T15.12: flow pivot.** flow.rs:366.
T11.15: "make the pivot relative to the column"; T15.12: "A column-relative pivot does not cure it. Scale both sides to power units." Both also drop `fold(1.0, max)` (dup). Fix: merge into T15.12 (power units), delete T11.15's bullet. Severity medium. Confidence high.

**PC-17  T13.9 vs T15.15: ring floor.** edits.rs:414/429.
T13.9 lowers zp+2 → zp+1 (another constant); T15.15 removes the floor ("use the fit itself, never max(fit, zp+2)"); T16.29 pins zp+2 "once, with its reason". Fix: T15.15 only; drop T13.9's first-half bullet and T16.29's zp+2 pin. Severity medium. Confidence high.

**PC-18  T10.4 vs T15.3/T21.3: the thickness rule.**
T10.4 hardens k_a + k_b = 2 as one freedom per mesh group; T15.3's third bullet and T21.3 obtain backlash from a thickness allowance on both gears, which "needs lifting the k1 + k2 = 2 rule". Two incompatible models of thickness. Fix: redesign R6. Severity medium. Confidence high.

**PC-19  T05.1 vs T10.10: two rewrites of TipRoom's verdict in different units.**
T05.1 (Phase 1): signed margin in radians, `tip_interference = margin < 0 || far_gap < tip_clearance`. T10.10 (Phase 4): crossing gap in mm (or converted at r_a1), room = min(far_gap − c, crossing − c), verdict `crossing < 0 || far_gap < c`, plus T05.3 (tight end), T05.12 (shrink to `tip_room`), T14.10 (move TipRoom). Five tasks, three phases, one function. Fix: redesign R4. Severity medium. Confidence high.

**PC-20  T18.5 + T10.15 give `GearCase::torque` two meanings.**
T10.15 makes a point mesh's driven torque the delivered torque (η·i·T₁); T18.5 documents the line-contact figure as "before the mesh's loss" and "then says so for point contacts". One field, meaning keyed on contact kind — a new rule-4 split. Fix: one definition for both (tooth normal-load torque = driver × tooth ratio, both kinds) with `on_body` the delivered figure for both; T10.15 changes `on_body`/a separate field, not `torque`. Severity medium. Confidence medium-high.

**PC-21  T07.2 (Phase 1) gates are reversed by T07.4 (Phase 4).**
T07.2's first fixture requires ε = 0 for 17/43 + 20°, Σ 0.5°, 10 mm faces; T07.4 says faces centred at the reference distance make "an aligned pair lose contact at ordinary faces for any Σ ≲ 2°" — an artefact it removes, after which "the zone is never empty while b_i ≥ the tip-zone width". Phase 1 pins a figure Phase 4 must un-pin; T07.5's window also uses the centres T07.4 moves. Fix: land T07.4 with T07.2 (or state T07.2's fixtures at offset faces), then T07.5/T07.15. Severity medium. Confidence medium-high.

**PC-22  T15.16 (fixed Gauss–Legendre) vs T06.8 (piecewise 1/L(t) split in CrossedPath::efficiency).**
A fixed 8–16-node GL rule loses its spectral accuracy on a piecewise integrand. Fix: integrate per smooth piece (breakpoints are known in closed form) or make T15.16 follow T06.8 and gate GLn vs GL2n per piece. Severity low-medium. Confidence medium.

**PC-23  Smaller conflicts / stale cross-references.**
- T21.11 relies on "the max(elliptical, line) rule" that T08.11 replaces; T21.11 has no dependency on T08.11.
- T15.14: "helical wording is not settled, so do not cement ε_α" — T06.4 settles it (ε_γ). Stale.
- T15.5 says "T05.10 proposes a closed form for the ring junction"; it is T05.2 (ring.rs:528-548). Wrong pointer.
- mutation.md L18 "A member with no pressure_angle reads 20°" (kills shape.rs:98, task T14.4) contradicts T14.4, which makes every field required and removes that default.
- plan.md Phase-1 exit: "The 144 surviving gear-core mutants are re-run" — mutation.md has 181 entries / 184 survivors; 144 is an unrelated row number.
- T03.1 vs T16.11: two different refinements for the rack gate's phase error (bracketed 1-D minimisation over ±1 step vs corner-trochoid/V-fit); pick one.
- T03.13's κ = 1/r_c (curvature) and T05.2's κ = sign(r_g − r′_c): same symbol, different quantities, same shaper functions.

### Duplicates not in plan.md's merge list (each lands twice or three times)
| Change | Tasks |
|---|---|
| signed load angle, strength.rs:794 | T08.7 b1 = T17.7 b5 |
| hertz.rs:433 x_tol → Tol::default | T15.4 = T17.7 b7 |
| screw.rs:1416 discarded β_b | T07.18 b2 = T17.7 last |
| ±5 shift fallback / ShiftRange closed | T10.6 = T15.7 b3 = T17.9 b2 |
| re-size loop units (shape.rs:1914) + 3-round note | T12.16 b1-2 = T15.7 b2 |
| tip lean 1e-9 → exact clear side | T12.16 b3 = T15.6 |
| tooth junction closed brackets, delete BASE_CROSS_GROWTH/CROSSING_GROWTH/MAX_STEPS | T03.4 (Ph1) = T15.5 b1 (Ph4) |
| ring junction bracket | T05.2 = T15.5 b1 |
| Ratio::cmp_checked deletion | T11.5 = T17.8 b4 |
| equivalent_to_rack to cfg(test) | T03.10 = T17.8 b1 = T05.17 step 3 |
| gear_to_dxf(&Gear) | T04.4 = T14.14 b1 |
| kind.signed at shape.rs:689; 1+σ(1−x) at shape.rs:1056 | T05.15 b2-3 = T14.8 |
| ring.rs:282 TIP_ABOVE_BASE_FRACTION | T02.6 = T05.15 = T15.11 (and see PC-13) |
| one flow ZERO / idle threshold | T11.10 b4 = T15.12 b1 = T16.28 = T19.9 (idle bool) |
| fold(1.0,max) floor | T11.15 = T15.12 |
| CLAUDE.md rule 3 "microseconds" | T11.13 = T18.12 |
| planet count .max(1) | T01.8 = T02.6 |
| coefficient legacy arm, ground_if_null | T01.3 = T14.4 |
| placeholder grammar law | T14.20 = T19.7 |
| cutter grid shared with gear-cli | T16.10 = T20.10 |
| T12.6 scheduled in two Phase-4 tracks (Train graph and Optimiser) | plan.md:163-164 |

### Ordering / churn
- T08.2 (Ph1) → T17.7 (Ph4) reverts it (PC-14).
- T03.4 (Ph1) legalises tip-below-base; T02.6 (Ph2) refuses it (PC-13).
- T07.1 (Ph1) adds refusals + 5 catalogue strings that T07.7 retires; acceptable only as a short interim — say so, or land T07.7 instead for the β₁<0 half.
- T09.10 deletes `limits` (Ph4); T21.3 re-adds them (Ph6).
- T14.8 turns BuiltMember/`Cut` match sites into methods; T03.15/T05.17 then delete those enums.
- T05.17 step 3 (shaper corner in κ form, rack at κ = 0) is T03.13 — scheduled in two parallel Phase-4 tracks.
- Phase/Needs check (script): no task needs a later-phase task once the merge list is applied.

---------------------------------------------------------------------------------------------------
## 3. Clusters one deeper redesign would subsume
---------------------------------------------------------------------------------------------------

**R1 — One generator: signed workpiece, rolling corner in normal-angle form.** Replaces T03.13, T03.14, T03.15, T05.2, T05.9, T05.14, T05.15, T05.17, T14.8 (member half), T04.1's "lasting cure" walker, T15.5 b1, T17.8 b1, T03.10 (part), mutation L18 ring-evenness. Deletes: Ring's parallel machinery (ring.rs is 1,103 production lines; roughly half is the mirrored flank/fillet/sections/outline/profile), the 7 corner copies (tooth/strength/shaper), `BuiltMember::{Rack,Ring}` and `auto::Cut::{ByRack,ByShaper}` dispatch, the κ = 0 special case of PC-1. Plan today runs this as two XL tracks (T03.15, T05.17) plus four partial precursors; make it one track with one migration.

**R2 — Flow by component (block-cut order), exact per component.** Replaces T11.1's global cap (becomes per component), T11.4 steps 2–4, T11.15, T15.12's conditioning half (small per-component systems are well scaled), T11.8's memo, and makes T11.7's order problem local. Deletes: the fixed-point iteration and fallback-subset logic T11.4 would add (~150–250 lines never written), the global 2^M loop. See PC-5.

**R3 — Search on the problem's structure.** Per component: held distance → exact 1-D division (T12.11 generalised, constraint edges as bracketed roots), free distance → 2-D per mesh, chains → DP over shared members. Replaces T12.7's feasibility walk, T12.8's margin, T12.10 steps 2–3, T16.8/T11.12's budget gate (cost becomes structural), much of T12.9. Deletes `Search::maximise`'s grid + compass walk and budget constants (auto.rs ~1680–1830, ~150 lines) and the rounds loop's re-size heuristics. Confidence medium-low: needs a check that the objective is separable as claimed on every preset.

**R4 — One signed clearance record per mesh, evaluated at both band ends.** `Clearances::at(mesh, a) -> {tip_crossing_mm, far_gap_mm, bottom_mm[2], flank_interference_depth[2]}`, used by sizing, the search filter and the report alike. Replaces T03.5, T05.1 (verdict), T05.3, T05.12 (TipRoom), T06.2, T10.10, T10.16, T14.10's TipRoom bullet, and T15.17's hula-gap doc. Deletes RingMesh (~250 lines, per T05.12) and three separately-written verdicts.

**R5 — One input validator (already half-merged).** T01.2 + T13.1 + T14.6 share check() per plan; also fold T01.6, T01.8, T01.9, T05.8's normalisation, T13.5, T02.6's refusal bullets and T02.7's key mapping into the same per-kind validator table, so "every refusal key names its field" is one table, not seven lists.

**R6 — Thickness as a per-member signed allowance.** Nominal split k (k_a + k_b = 2) stays the zero-backlash design; backlash comes from per-member thickness deviation A_sn (closed form j = Σ A_sn/cos α …). Replaces T10.4's odd-cycle special case, T15.3 b3, T21.3, T09.10+T21.3 churn, and cures lens-standards#6 (PC-33). Deletes the per-mesh ThicknessMod groups and the mm clearance band's role as the only backlash source.

**R7 — Crossed face as an interval on the operating contact's axial coordinate.** T07.2 (zone), T07.4 (centring), T07.5 (window), T07.15 (automatic face), T08.12 (widths as Σ → 0) and screw.rs:935/1005's `axial_centre().unwrap_or(0.0)` are one model; landing them apart causes PC-21.

**R8 — One zero/tolerance module for the train.** T11.10 b4, T11.15, T15.7 (AGREE_REL), T15.12, T16.28, T19.9 idle: one `train::tol` with relative, power-normalised constants and their derivation.

---------------------------------------------------------------------------------------------------
## 4. Premises checked against the code (18 medium/high tasks)
---------------------------------------------------------------------------------------------------
Confirmed (premise holds as stated):
1. T01.1 — graph.rs:288-298 pushes the carrier's axis unconditionally; a cycle never terminates.
2. T11.1 — flow.rs:223/226 `0..(1u32 << m)`, `1 << k`.
3. T08.2 — strength.rs:1053-1058: DB K_f = h + (s/ρ)^l (s/h)^m; moment_arm ≤ 0 gives NaN/negative; wrapped in Some.
4. T08.7/T17.7 — strength.rs:794 `load_dir[0].abs().clamp(-1,1).acos()` loses the sign.
5. T07.6 — screw.rs:249 `(1 − 2k cos Σ + k²).sqrt()`, cancelling near Σ → 0.
6. T05.7 — ring.rs:587 `self.r + 2.0*self.mt`, i.e. 0.75 m beyond rf at x = 0.
7. T10.13 — shape.rs:3373 `always_reverses = meshes_of[i].len() > 1`.
8. T02.3 — mod.rs:4453-4458 `map_or(usize::MAX, …)`, then `u32::try_from(short).unwrap_or(u32::MAX)`.
9. T11.9 — planetary.rs:326 `1/(|R|(1−η)+η)`.
10. T10.1 — mod.rs:3467-3480 `Backlash::banded(0.0,1.0,1.0, …)` puts every mesh at the same band end.
11. T06.5 — contact.rs load_share: ramp is not normalised above ε = 2.
12. T09.1 — jgma.rs:139 `v >= lo && v < hi`.
13. T09.2 — metrology.rs:260 `half = nominal/(2 cos β_b)`; the base-tangent-plane projection is W·cos β_b/2 (checked by derivation), and pin_seat :504 lacks the cos β_b factor.
14. T04.6 — gear.rs:986-1000 `j = 2·fit·(inv α_actual − inv α_ideal)` with no σ; mesh.rs:354 has `kind.sign()`.
15. T12.1 — shape.rs:1904-1928: loop breaks after `plan` was replaced; `closed(&plan, &place(&v))` pairs an old v with a new plan; `.unwrap_or_else(|| settled.clone())`.
16. T12.16/T15.7 — shape.rs:1914 compares mm with `search.resolution * 1e-3`.
17. T03.4/T02.6 — tooth.rs:471, ring.rs:282, auto.rs:671 as quoted in PC-13.
18. T11.15/T15.12 — flow.rs:366 absolute pivot `> 1e-12`; :394-401 `fold(1.0, max)` floor; groupings.rs:242 its own `1e-9`.
Also: T17.9 (MeshFlow::paths written, flow never reads it — only doc lines mention it in flow.rs), T14.10 (auto.rs:1332 reaches `crate::train::TipRoom`), T11.17 (gear.rs:207 recuts `mean`).

Premises found wrong or overstated (details above): PC-7 (T04.3 "a bound", ρ monotone), PC-9 (T08.6 closed form), PC-4 (T06.7 "physical"), PC-23 (T15.5 pointer, T15.14 stale, L18 default, "144 mutants").

---------------------------------------------------------------------------------------------------
## 5. Gaps
---------------------------------------------------------------------------------------------------
Programmatic ledger check (python over ledger.json and the 21 task tables): all 780 confirmed/partly + 25 unverified + 1 unverifiable findings appear in some task table except **lens-tests-train#12**, which T16 declines by name. No refuted finding is cited. Every task id appears in the roadmap. 42 findings are assigned to more than one task (list reproducible with the script); the ones that matter are in §2.

**PC-30  P2 cure leaves ~15 `teeth.max(1)` silent clamps with no owner.** auto.rs:117,246,630; gear.rs:117,136,190,690,863,1076; ring.rs:197,657,658; tooth.rs:359; shape.rs:529,926,933.
After T01.6 refuses z = 0 at the boundary, these are dead guards that still silently compute z = 1 for any internal caller (e.g. an edit sizing a member — T15.15 notes "never a 0-tooth member"). T01.8/T02.6 name only shape.rs:463 and ring.rs:198. Fix: delete them with the gate, add `debug_assert!(z ≥ 1)` at the constructors; T02.6's law covers the rest. NEW (instances). Severity low. Confidence high.

**PC-31  P1 instances with no task.**
- hertz.rs:300: a failed ellipse reads pressure 0 inside max() (T08.11 rewrites the combination but not this).
- strength.rs:590-600: a ring ToothOutline with no fillet returns bracket (0,0) and junction/root 0 — absence as value in the bending seam.
- screw.rs:935/1005: `axial_centre(..).unwrap_or(0.0)` — face centre 0 when unknown (goes with R7).
- flow.rs:113 `known.unwrap_or(0.0)`; conditions.rs:181 `map_or(0.0, …)` — check whether each 0 is physical.
Fix: add to T02.9's list; each gets Option or a comment naming why 0 is physical. NEW (not in ledger by site). Severity low. Confidence medium.

**PC-32  P9/P10: the guard "conventions" (PC-11).** Unassigned; MAX_TOOTH_THICKNESS_FRACTION_OF_PITCH not in the ledger.

**PC-33  P9 instance lens-standards#6 is not cured by any Phase 1–5 task.** T15.3 keeps 0.02 ± 0.02 mm absolute and defers the fix to the optional T21.3 in Phase 6; the plan lists it under P9 as cured. Fix: R6, or state the zero-minimum-backlash default in state.md Known-approximate now, with size and sign.

**PC-34  P10: a bracket walk left out of T15.5's migration list.** shape.rs:944-951 (`grown`: doubling ×60 cap, `floor*(1+1e-9)` at :955/957) sizing the worm diameter. T15.7 names the 1e-9/1.000_001 edges but not this walk. Fix: migrate to `grow_bracket` with the limit stated, or close the bracket (at Σ = 90° the distance-vs-d₁ relation is algebraic).

**PC-35  "Every design decision is settled" is not true.** Open choices left in tasks: T08.2 (sweep from finite top land *or* rate on fillet tangency), T10.16 (bottom-clearance refusal *or* tip shortening), T15.4 (a) *or* (b), T13.9 ("look into" 8 Overdetermined offers), T13.12 ("first decide which rule is intended"), T11.3 (keep max as tie-break *or* drop it), T15.13 (inv_inverse *or* keep Tooth's radius), T04.12/T20.3/T20.4 ("better still"/"alternatively"), T08.12 ("if this is rejected"), T12.9 ("either fix the multistart or write the bound"). Each needs one decision before implementation.

**PC-36  T18.28's register gate cannot fail on its fault (P6).** It fails when a covered file changes unless the entry's commit hash is bumped — bumping the hash satisfies it without re-running the check. Nearly every Phase-4 task touches covered files, so it becomes a ritual. Fix: the register entry names a command; CI runs every entry's command whose files changed (or all of them nightly) and compares the recorded agreement.
