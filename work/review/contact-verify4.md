# contact-verify4: adversarial check of work/contact-model.md round three

Author: contact-verify4, 2026-09-29. Read-only; no cargo. Scripts and outputs in
`~/.cache/gearcalc-work/contact-verify4/`:

- `v1_spur2d.py`: an own 2-D spur model of 17/43 at the wheel-tip entry. It shares no prototype code.
  - Exact involute distance functions (the unwrap coordinate).
  - A physical tip round: a true circle of radius r_e, tangent to the involute and the tip circle, solved exactly.
  - Common-approach load sharing, with tooth compliance from contact-verify2's `v1_stiff.Tooth` (potential energy plus Sainsot) and Weber's contact.
  - 2-D Hertz at each contact.
- `v2_relief.py`, `v2b_where.py`: relief C_a(1−u/L_a)² added to v1's gaps.
- `v2c_stiff.py`: the subject's tooth compliance against the potential-energy (PE) tooth at the same radii.
- `v3_width.py`: an own reference quadrature for the depth term. It tests the node rule, its cap, and the panel sign up to b/h 1e6.
- `s_runs.py` → `s_runs.txt`, `s_det*.py`, `s_dist.py` → `s_dist.txt`: **the subject's code run as-is** (evidence about it, not a check of it).
  - β → 0 closer than 0.1°; an untuned r_e.
  - Panel refinement.
  - Brute phase scans.
  - Σ and centre-distance sweeps.
  - `kernel.n_width` instrumented to record the b/h actually met.

Incident: my first `v3_width.py` had a grading loop that never ended and grew a list; it was the 5.2 GB watchdog kill at 10:28. Fixed (the grading stops at s·1e-6). Every later run was capped at 1.8 GB (`RLIMIT_AS`).

## Summary

| # | Claim | Verdict |
|---|---|---|
| 1 | One tip-form rule; spur tip line takes 1/r_e; β 0→0.1° ≤ 0.03 % | **CONFIRMED** at β ≤ 2° (see 7 for helical) |
| 2 | Edge peak = 2-D Hertz on r_e within 0.3 %; 3198/4989/6585 plausible | **PARTLY**: the magnitudes hold; the 0.3 % check is an identity; the peak is a limit at a curvature step and is not panel-converged |
| 3 | Relief continuity (C_a, L_a) | **CONFIRMED** smooth; **PARTLY** on size (it depends on tooth stiffness) |
| 4 | Width quadrature exact to 1e-10, capped at 64 | **CONFIRMED** safe; "any b/h" is wrong (holds to b/h ≈ 5.5) |
| 5 | Worm figures; matched wheel "cannot take the edge rule, no play" | Figures **CONFIRMED**; "cannot" **PARTLY** (a limit of how the prototype represents the wheel, not of the wheel) |
| 6 | Require r_e in the expensive mode | **CONFIRMED** with conditions |
| 7 | Round-tangency breakpoints; Σ-continuity | Breakpoints **CONFIRMED** for spur; Σ-continuity **REFUTED as established**. New defect: the line construction jumps discontinuously |
| 8 | 1.6–32 s per mesh | Arithmetic **CONFIRMED**; as a prediction **PARTLY** |

## 1. The tip-form rule: CONFIRMED (spur and low β)

**The formulation is sound.**
- One C¹ modification F(σ) along the flank normal, carried in the gap.
- The curvature across the edge is the graph curvature F″/(1+F′²)^{3/2}, not the Hessian of g. That is the right choice: the Hessian would carry a 1/cos³ factor, 11× at θ_a 64°.
- On the round, the gap's value off the zero set is measured along each flank normal, not the contact normal. It is off by 1/cos φ, about 10 % at w ≈ 45 µm. This slightly misplaces where load extends along the round. The peak is unaffected, because it sits at the tangency, where F′ = 0.

**The spur tip line gets 1/r_e.**
- At β 0 the model's kA is 6.731 and 12.163.
- My exact geometry gives 6.610 and 12.042, which is physical 1/r_e plus 1/λ₁.
- The difference is κ_f2 ≈ 0.10/mm: the prototype's "round measured from the flank" convention.
- Its effect is **+1.8 % / +1.0 % in κ and +0.9 % / +0.5 % in p. That is conservative.**
- Tangency radius: 22.3880 against an exact 22.3854 (r_e 0.2), and 22.4440 against 22.4434 (r_e 0.1). A few µm, which is negligible.

**β → 0, subject run as-is** (`s_runs.txt`), r_e 0.1 at 20 N·m:

| β | 0 | 0.001° | 0.01° | 0.1° |
|---|---|---|---|---|
| Field max, MPa | 6585.4 | 6585.4 | 6585.2 | 6585.2 |

- Untuned r_e 0.05: 8846.4 at β 0 and 8846.8 at β 0.1° (+0.005 %).
- A 1e-12 mm perturbation of a moves nothing at β 0.1° or 2° (0 of 40 phases).

**Own 2-D model against the prototype:**

| Load | Own: unset / 0.2 / 0.1 | Prototype: unset / 0.2 / 0.1 |
|---|---|---|
| 20 N·m | 3241 / 5004 / 6636 | 3198 / 4989 / 6585 |
| 60 N·m | 5593 / 8643 / 11454 | 5538 / 8606 / 11350 |

- Agreement is 0.3–1.3 %.
- In both models, the peak is the wheel round's tangency against the pinion at r₁ 8.00–8.01. There q ≈ 100 N/mm, the stiffness share of the entering pair.

## 2. Edge peak and Hertz: PARTLY

- **`t_hertz_edge.py` is not independent in q.** It feeds the model's own q into √(qE*κ/π). The 0.3 % therefore tests only kA.
  - The independent support is §1's own model, within 1.3 %.
  - That match is partly compensating. Mine is 2-D mid-face; theirs is a face-end panel, with the +2–6 % end effect the doc states.
- **b/r_e is the wrong validity test at the peak.** The peak is the round-side limit *at* the tangency, a curvature step, so half the Hertz strip lies on the flank.
  - Rolling one half-width b onto the round lowers p by **3.9 / 4.0 % at 20 N·m and 5.0 / 5.3 % at 60 N·m** (own model).
  - So the reported figure is an upper bound, high by up to about 4–5 %. The doc lists this as "not measured"; this is the measurement.
- **The face-end maximum is not panel-converged.** Spur, r_e 0.1, same phase:

  | Panels | 24 | 48 | 96 |
  |---|---|---|---|
  | Field max, MPa | 6585.4 | 6624.4 | 6678.7 |
  | Peak station z, mm | 4.792 | 4.896 | 4.948 |

  - The increments grow (+39, then +54), so the square face end looks singular under this kernel.
  - Every "field max at a face end" is therefore a figure at N = 24. Rust needs a defined evaluation (a distance from the face, or end relief), not "the max".
- **Magnitude is plausible:** q ≈ 100 N/mm at the entry is the stiffness share. p = √(qE*κ/π) with κ ≈ 1/r_e + 1/0.5 mm gives 5–6.6 GPa.
  - Note that 20 N·m on this pinion is already about 2.1 GPa at the pitch point.

## 3. Relief: continuous; how large the effect is depends on the tooth model

- **The prototype's series is self-consistent.** A linear-compliance fit (p/p₀)² = 1 − C(u)/δ₀, with δ₀ 14 µm taken from C_a 5, predicts:
  - C_a 0.5 / 1 / 2 µm to within 0.1 %;
  - L_a 0.2 / 0.8 to within 1.7 %.
- **My model gives smaller reductions.**

  | Case | Own model | Prototype |
  |---|---|---|
  | C_a 5 µm, r_e 0.1 | −8.8 % | −14.2 % |
  | C_a 5 µm, r_e unset | −12 % | −29 % |

- **The cause is tooth stiffness** (`v2c_stiff.txt`). The subject's derived tooth has ct₁ + ct₂ that is **1.41–1.47× stiffer** than the PE + Sainsot tooth at the same radii.
  - At C_a = 0 the peak depends only on stiffness *ratios* between pairs, so the two models agree.
  - Under relief the absolute stiffness governs, so they do not.
- **So** "5 µm of relief only reaches 5650" carries the foundation model's bias; it does not describe the edge itself.

## 4. Width quadrature: CONFIRMED safe; the wording is wrong

Reference quadrature against the capped rule:

| b/h | ≤ 5.5 | 10 | 30 | 100 | 1e3 | ≥ 1e5 |
|---|---|---|---|---|---|---|
| Rule error | ≤ 1.8e-10 | −4.6e-6 | −1.4 % | −17 % | −41 % | −49 % |

- The cap binds from b/h ≈ 5.5.
- Beyond the cap, the self-panel value stays positive and bounded. The capped value overstates the degenerate station's compliance by up to 2.6×, which is tiny in absolute terms.
- **The b/h actually met:** at most 0.14 (spur) and 0.062 (helical). **The cap never fired** (0 of 6–37 M calls).
- Fix the doc: "exact to 1e-10 up to b/h ≈ 5.5". A degenerate station (kA floored) has b/h about 1e5–1e6.

## 5. Worm

**The figures match the files:**
- involute wheel: 2211.8 MPa, η 67.320 %;
- matched ZN: 1842.3 MPa, η 67.290 %;
- r_e 0.2 and 0.1: 8839 and 12169 MPa. These equal C√(qE*kA/π), with kA = 1/r_e + 0.23 and C ≈ 1.003.

**"Cannot take the edge rule and has no play" describes the prototype's representation, not the physics.** `matched.py` never builds the wheel, but the wheel flank *is* the union of the contact lines over the hob's rotation.
- A gap to it is a closest-point projection onto that two-parameter surface.
- Play then follows from hob thinning, hob oversize or centre distance. With a hob identical to the worm the play is zero by construction.
- Because a hob's addendum exceeds the worm's, the worm's tip round meets a matched wheel as a relief. So the edge contact exists there too, spread along a line.
- The involute wheel's point-contact edge figures (q 616–659 N/mm on the round) therefore probably overstate a hobbed set's edge. Sign known, size unmeasured.

## 6. Sharp-edge convention: agree, with conditions

- It reproduces as a flank figure: my model gives 3241 against 3198.
- It is neither a limit of the family nor conservative against any real edge.
- **Requiring r_e is right, but:**
  1. r_e then dominates the headline: 3831 → 13397 MPa for r_e 0.4 → 0.02.
  2. A chamfer brings back sharp corners, so "require r_e" should read "require a C¹ tip form".
  3. There must be no hidden default. Under rule 1, a default is a Rust number.
- **Report the flank maximum and the edge maximum separately**, each with the form that produced it.

## 7. Breakpoints: work for spur. Σ-continuity: NOT established. New defect in the line construction

**The breakpoints work for spur.** `analyse` comes out at or above brute 401-phase scans: 6585.4 against 6547.1, and 4989.1 against 4982.8.

**The line construction is discontinuous (new).** Case: helical β20, r_e 0.1, 2 N·m, Σ 0.
- A **2.8e-13 mm** change of a flips state(0.8875 p) from 2075 to **2446 MPa (+18 %)**, and state(0.7425 p) from 2324 to 2057 (−11 %).
- This is deterministic and independent of history (`s_det2`).
- At the same phase, the two values of a give two different solutions (`s_det3`):

  | a | Approach D, µm | Pair −2's line |
  |---|---|---|
  | 31.925333 | 1.231 | 4.5 mm long, ends "face1" |
  | 2.8e-13 mm off | 1.565 | 0.73 mm long, ends "root1" |

- **The cause:** the anchor and e_L seeding picks a different line. The doc's "interior / tip-edge / face-edge anchor searches, not a branch" is a branch.
- **How often:** under a 1e-12 mm perturbation, 1 of 40 phases jumps by 10 % with r_e 0.1. Nothing jumps above 1 % at r_e unset, or at β 0.1° / 2°.

**Grid maxima against a** (401 phases, `s_dist.txt`):

| Offset from a_par | 0 | −0.17 nm | −350 nm | −700 nm | −1450 nm | +700 nm |
|---|---|---|---|---|---|---|
| Grid max, MPa | 2294.1 | 2446.2 | 2564.7 | 2643.4 | 2559.5 | 2294.0 |

- A second a within 2.8e-13 mm of the −0.17 nm point gives 2324.3.
- `analyse` at a_par gives 2294.7.
- The noise floor is therefore ≥ 6–7 %. Neither "1.45 µm → +14 %" nor the Σ series 2309 / 2303 / 2318 / 2415 / 2484 is resolved.

**A factual error in the doc:** 2484 at Σ 0.01° was run at 31.923880 (`dbg5`), not at a_par. At a_par, `dbg6` reads 2426.7.

**The fix:** seed the line so the result is continuous. Examples: e_L from the flank's conjugate direction, not from the anchor's local curvature; or all candidate lines kept and the valley deciding between them. Then re-run Σ, x and face at r_e 0.1.

## 8. Cost: PARTLY

- **The arithmetic reproduces:**
  - spur: 374.6 + 233 + 106.8 + 168 + 662.5 + 29 = 1574 ms;
  - worm r_e 0.1: 31.9 s.
- **It is an estimate** (unit costs, ±2×), not a measurement.
- **It does not include panel convergence.** At N = 96, the depth and G work grow about 16× and LU about 64×. For spur that is depth about 10.6 s and LU about 1.9 s.
- **It does not include the robust line seeding** that §7 requires.
