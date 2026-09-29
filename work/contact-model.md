# Unified contact model: phase-exact prototype (the expensive mode)

Authors: contact-proto (2026-09-26/29), contact-proto-2 (round two), contact-proto-3 (2026-09-29,
round three, this revision). Read-only on the repo; pure Python, no numpy.
- **Scripts and outputs:** `~/.cache/gearcalc-work/contact-proto/` (README there). This round's are
  `form.py`, `valley.py`, `t_edge.py`, `t_v.py`, `t_iso3.py`, `t_cost3.py`, `t_kernel3.py`,
  `t_hertz_edge.py` and `run_all.sh`. Round two's text is kept beside them as
  `contact-model.round2.md`.
- **Built on:** `work/plan.md` §5 (2026-09-30: two modes; the field model is the on-demand expensive
  mode, built for accuracy and completeness, with its cost recorded), `review/contact-verify2.md` and
  `review/contact-verify3.md`.

**Status: complete for round three.** Every table was run under the final code (`run_all.sh`,
`run_rest.sh`); each worker is capped at 2 GB.

**Verdict of this round.**
1. **One edge rule at every β.**
   - A gear's tip form is its own input: relief (C_a, L_a), an edge round r_e and the tip land, as one
     modification F(σ) measured along the flank normal.
   - The gap carries both gears' forms. A pair's line is the gap's valley, and every station takes
     the gap's own curvature. So spur, helical, crossed and worm edge contacts all meet the same
     1/r_e.
   - Verifier 3's +64 % and +109 % jumps are gone. At β 0 → 0.1° the field max moves ≤ 0.03 % at
     r_e 0.2 and 0.1, at 20 and 60 N·m (§2.1).
2. **The edge peak is 2-D Hertz on r_e**: within 0.3 % of an independent computation, at
   b/r_e = 0.07–0.17. The ring's √ law holds (§2.1).
3. **Relief and r_e are continuous as gear inputs**: +0.07 % and +0.19 % at β 0 → 0.1° with
   C_a 2 and 5 µm (§2.2).
4. **What the edge costs in the rating.** At 17/43 m1, 20 N·m, the field max is 3198 MPa with r_e
   unset (flank convention), 4989 at r_e 0.2 and 6585 at r_e 0.1.
   - It sits on the wheel's tip round (load-extended entry) and carries 7–12 % of the force.
   - C_a 5 µm, below the approach, only reaches 5650.
   - This is what the expensive mode exists to expose, not a defect of the rating.
5. **Special cases removed** (§2.4). One model-level case remains, the sharp-edge convention; the
   proposal is to require r_e in the expensive mode.
6. **The verifier's smaller items are fixed** (§2.3):
   - the width quadrature is exact to 1e-10 at any b/h;
   - the ring takes ISO's Z_D = 1;
   - the 1204 MPa location is corrected;
   - the ring's cost row is 0.65 s, not 4.6 s;
   - the fast-route kernel is recounted (§5).
7. **Worm wheel: both are kept; the owner chooses** (§4).
8. **Continuous in Σ, x, face width, load, r_e, C_a and L_a** (§3.2). Edge contact at light load is
   steep in Σ: +0.6 % per 0.001°, which is physical.
9. **Cost:** 1.6–32 s per mesh as built in Rust. The Σ = 0 fast route reaches about 45 ms only with a
   b-independent kernel (§5).

## 1. The model: every mesh, one construction

Signed tooth count (z < 0 is a ring) and the shaft angle Σ are parameters; nothing branches on the
kind of mesh.

1. **Exact rigid gap with the gears' own forms** (`valley.py:part`, `form.py`).
   - Involute helicoids are parallel surfaces, so d_i(X) = k_i(v_i(ρ) ∓ (ψ − n p_i)) with |∇d_i| = 1;
     the foot point X − d_i ∇d_i is exact and gives u = sg(r_a − ρ_f), the depth below the tip edge.
   - Each gear adds F_i(σ), with σ = u / sin θ_a. Here θ_a is the exterior corner angle between the
     flank and the tip land, cos θ_a = tan α / √(tan² α + 1 + r_a² tw²) at r_a, for either kind.
   - **F = relief + round + land**, and the whole form is C¹:
     relief C_a(1 − u/L_a)² for u < L_a; the round, tangent to flank and land, from
     σ_t = r_e tan(θ_a/2), F = r_e − √(r_e² − w²) for w = σ_t − σ ≤ r_e sin θ_a; the land, slope tan θ_a.
   - The gap is g = [d₁ + F₁] + [d₂ + F₂]. The tip is no longer a clip.
2. **One line per tooth pair: the valley of g** (`VField.valley`).
   - The anchor is the minimum of g over the pair's field, as before; it is only a seed now.
   - e_L is the minor eigenvector of the gap's curvature at the anchor, forms included.
   - Each station on the base line A + y e_L moves to the minimum of g across it, along e_A = n × e_L.
     - On a conjugate flank line that minimum is the line itself: three evaluations and a parabola.
     - Where the base line leaves a member through its tip, the minimum lies on that member's round,
       or at its sharp edge. The valley then follows the edge by itself.
   - Only faces and root/form circles clip (hard). At Σ = 0, e_A is transverse, so face clips are exact.
3. **Curvature of the actual local geometry at every station** (`VField.Kform`).
   - K = Σ_i [κ_i(ρ_f) u_i u_iᵀ + κ_F,i a_i a_iᵀ], where a_i = ∇σ_i is the across-edge direction and
     κ_F = F″/(1 + F′²)^{3/2}.
   - κ_F is exactly 1/r_e on the round and 2 C_a sin² θ_a / L_a² on the relief.
   - kA, kL and the Hertz shape C are each station's own; the normal ∇(d₁ + F₁) − ∇(d₂ + F₂) tilts on a round.
   - On a round the curvature is 1/r_e plus the flank's own, because the round is measured from the
     flank, as every profile modification is. That term, κ_f r_e, is 1–3 %.
4. **Load**: coupled lines with the tooth in series (round two, unchanged in kind).
   - Panels on the valley's arc length; kernel G(r) with depth reference h; Δ closed form per active
     set; a fixed point on b.
   - The seed is independent slices with the derived 2-D line compliance at the nominal load. Round
     two's +0.02 mm²/N placeholder was about 100× too soft.
5. **Phase**: breakpoints by a signature scan plus bisection, Gauss means, golden maxima.
   - **New:** the signature carries whether each loaded end lies on a round.
   - The round's tangency is a curvature step (1/r_e + κ → κ), so pointwise Hertz steps there, and
     the supremum is the round-side limit. Without the breakpoint the golden search missed it: 5324
     reported against 5646 on a fine scan.
6. **Tooth compliance**: `stiff.py` with the derived half-plane foundation (round two §2.4).

**The sharp-edge convention (r_e unset).**
- The gap is the kink's (F = −σ tan θ_a beyond the edge); the normal and curvature are the flank's.
  It is not the r_e → 0 limit, which is singular, as elasticity says a sharp edge is.
- It reproduces round two to the last digit: 3198.0 / 3211.7 at 20 N·m and 5537.8 at 60 N·m.
- A first version took the kink's normal from the tip land, rotated by θ_a ≈ 60°. That corrupted the
  torque arms (η 97.98 instead of 98.77 %) and was fixed.

## 2. This round's items

### 2.1 Edge contact: one rule at every β (`t_edge.py` → `edge_final.txt`, `hertz_edge.txt`)

Setup: 17/43, m 1, b 10, μ 0.06, elastic, Σ 0, no relief. Field max in MPa at β 0 / 0.1 / 0.5 / 2°:

| r_e | 20 N·m | 60 N·m | η at 60 N·m, % |
|---|---|---|---|
| unset | 3198.0 / 3211.7 / 3239.4 / 3566.4 | 5537.8 / 5532.5 / 5509.2 / 5752.0 | 98.774 / 98.777 / 98.769 / 98.759 |
| 0.2 mm | 4989.3 / 4989.1 / 5015.2 / 5420.6 | 8605.9 / 8603.9 / 8612.5 / 8847.7 | 98.856 / 98.856 / 98.855 / 98.846 |
| 0.1 mm | 6585.4 / 6585.2 / 6629.5 / 7240.9 | 11350.3 / 11346.9 / 11362.5 / 11733.8 | 98.804 / 98.806 / 98.805 / 98.795 |

- **Continuous at every r_e and load.** At β = 0 the spur tip line lies on the wheel's round and
  takes 1/r_e, exactly as the β > 0 edge valley does.
- **Round two's +64 %/+109 % was the β = 0 side missing 1/r_e.** It was not the β > 0 side being
  wrong.
- **The residual rise to β 2°** is +9 % (r_e 0.2) and +10 % (r_e 0.1) at 20 N·m. It is real: the
  edge contact shortens as (b tan β_b)² grows.
- **Where the peak sits.** It is always at the wheel's tip round (edge2) at a face end, against the
  pinion near its start of active profile (r₁ 8.00–8.02, r_b 7.99). The pinion's flank curvature
  there is 1.6/mm.
- **Edge share of the force:** 7–8 % at 20 N·m and 11–13 % at 60 N·m.
- **The mean loaded pairs fall with a round** (1.86 → 1.75 at r_e 0.2), because a round is also a
  relief: it starts σ_t = r_e tan(θ_a/2) below the tip.
- **Independent 2-D Hertz check** (`t_hertz_edge.py`, no prototype code). The relative curvature is
  1/r_e plus the flank curvatures at the reported radii, and p = √(q E* κ/π) uses the station's q.
  - The model's kA agrees to within 0.5 %, and its p to within 0.26 %, on all 23 edge peaks.
  - b/r_e = 0.07–0.17, so Hertz holds on the edge.
- **The ring's √ law** (verifier: 1.4175 predicted, 1.3976 observed). On ring 17/−43 β15 at 60 N·m,
  p(0.1)/p(0.2) = 5488.6/3975.4 = 1.3806.
  - The stations give √(kA ratio × q ratio) = √(10.036/5.037 × 83.3/87.1) = 1.3804.
  - At equal load it would be 1.4115. The 2.2 % gap is load shed by the sharper edge.
  - The verifier's κ₂ = −0.046 omitted the pinion's own flank term, +0.08/mm.
- **Σ, the crossed pair and the worm meet the same rule** (§3.2 and §4). The worm's peak moves onto
  the worm's tip round.

### 2.2 Tip relief and r_e as the gear's own inputs (`edge_relief_all.txt`, `v_La.txt`)

`Form(member, C_a, L_a, r_e)` belongs to each gear, and round two's shared `Field.relief` is gone.
At 20 N·m, L_a 0.4 mm, field max at β 0 / 0.1 / 0.5 / 2°:

| Form | Field max | Pairs at β 0 |
|---|---|---|
| C_a 2 µm, r_e unset | 2749.4 / 2770.6 / 2830.0 / 3141.3 | 1.846 |
| C_a 2 µm, r_e 0.1 | 6221.1 / 6225.2 / 6286.4 / 6912.4 | 1.793 |
| C_a 5 µm, r_e unset | 2258.8 / 2264.7 / 2315.4 / 2596.5 | 1.821 |
| C_a 5 µm, r_e 0.1 | 5649.5 / 5660.5 / 5749.8 / 6391.0 | 1.773 |

- **Continuous in β with both inputs set**: +0.07 % and +0.19 %.
- **Relief below the approach does not unload the edge.** Δ is about 10 µm, so 5 µm of relief cuts
  the edge peak by only 14 %.
- **Continuous in L_a and in C_a** (`v_La.txt`, r_e 0.1, 20 N·m):
  - L_a 0.2 / 0.4 / 0.8 mm: 5843 / 5650 / 5550 at β 0, and 6651 / 6391 / 6249 at β 2°.
  - C_a 0 / 0.5 / 1 / 2 / 5 µm: 6585 / 6495 / 6404 / 6221 / 5650 at β 0, and 7241 / 7161 / 7075 /
    6912 / 6391 at β 2°.

### 2.3 The verifier's smaller items

- **Width quadrature** (`t_kernel3.py` → `kernel3.txt`).
  - The depth term's integrand is analytic except at x = ±ih. An n-point Gauss–Chebyshev rule is
    therefore exact to E^−2n, with E = h/b + √(1 + h²/b²).
  - Round two's 4 points erred 4e-11 / 6e-8 / 1e-3 / 8e-2 at b/h 0.1 / 0.25 / 1 / 3. That is E^−8,
    as the bound says.
  - Now n = ⌈ln(10¹⁰)/(2 ln E)⌉ + 1: 4 nodes to b/h 0.06, 5 at 0.1 and 15 at 1, capped at 64.
  - The infinite-line check is 6e-14 at b/h 1 and 1.3e-12 at 3.
  - **Why the cap matters:** uncapped, the rule met stations whose kA was floored at 1e-12, where
    b/h ≈ 10³, and grew to 10⁴–10⁶ cached nodes. That was the process the memory watchdog killed.
  - A two-term Taylor form, f(0) + (b²/4)F′(h²), errs 1e-5 at b/h 0.1 and is b-independent (§5).
- **Ring, ISO's Z_D = 1.** `rating.iso` now returns ISO's Z_D = 1 for an internal wheel, with the
  signed-radius extension beside it, labelled as ours. The spur ring's wheel reading is then +9.2 %
  over ISO's σ_H,D = 659.3, not −7.0 % (§3.3).
- **The 1204 MPa location.**
  - Round two's "1204 → 3818 → 5336 at the pinion's tip edge" was wrong for 1204. That maximum
    (1205.0 now) sits at r₁ 16.622, the pinion's start of active profile, at the face end, where the
    ring's sharp tip edge meets it.
  - Only the r_e cases sit on the pinion's tip. Now: r_e 0.2 gives 3975 and r_e 0.1 gives 5489, on
    the pinion's round (r₁ 20.11–20.16, just below r_a 20.20).
- **Ring cost and fast-route kernel cost:** §5.

### 2.4 Special cases and branches

**Removed this round:**
- The edge continuation rule: `edge_lines`, `clip_skip` and the polyline pieces.
- Zero-length tip segments, with their `on` and `extra`.
- 1/r_e only on edge pieces.
- The `tips_relieved` flag.
- The tip bands of the clip.
- The per-pair Hertz shape.
- The shared relief.
- `Member`'s β = 0 form-radius branch. Its value is identical to the last bit.

All are replaced by §1's one gap, one valley and one curvature rule.

**The κ_A ≤ 0 skip:** 0 anchors and 0 stations in every run of this round, the rings included, even
with r_e unset. It is reachable only under the sharp-edge convention.

**Remaining, with the proposal for each:**

| Where | What | Kind | Proposal |
|---|---|---|---|
| `form.py` `if s.re` | the sharp-edge convention | **model** | require r_e in the expensive mode; then it and the kA ≤ 0 guard go |
| `derive_foundation` / `Member` z < 0 | a ring's tooth is the rack tooth on a stated rim | model substitution | the shaper-generated ring tooth (redesign G) |
| clip | faces and root/form circles are hard | geometry | faces: the same form rule along the lead (end relief, chamfer) when wanted; root: interference is flagged, not modelled |
| anchor | interior, tip-edge and face-edge searches | minimisation over a rectangle | not a branch; once r_e is required the tip-edge search is only a seed |
| `valley.gpt` | the root barrier (+1 mm) in the cross search | numerical | keeps the cross search on the flank |
| `valley.valley` | three-point parabola, else Brent | numerical | — |
| `clip` `qa < 1e-15` | a line parallel to an axis | degenerate geometry | the constraint is independent of y; kept |
| `arms` `nv > 1e-14` | friction direction at zero sliding | measure zero | — |
| `kernel.G` table ends, `shape_C` s < 1e-24 | series inside functions | value | — |
| wheel generation (§4) | involute wheel or matched wheel | **model** | the owner's call |
| `rating.iso` Z_D | ISO's own rule for internal wheels | ISO | reproduced as ISO states it |

## 3. Validations

### 3.1 Oracle, ε and breakpoints (unchanged; confirmed by verifiers 2 and 3)

- ISO 21771 j_n from the oracle against formula: 34.5219 / 38.6366 / 33.7489 / 37.7174 µm.
- The a₀ table matches 10/10.
- ε_γ is 2.58060 against 2.580562.
- Mean length / (b / cos β_b) = ε_α 1.4918785 at light rigid load.
- Ring 17/−43 x0 interferes: the corner goes 24.5 µm into the generated fillet. Flag against the
  generated root.

### 3.2 Continuity sweeps (`v_sigma.txt`, `v_re.txt`, `v_x.txt`, `v_face.txt`; 17/43 m1 b10, μ 0.06)

- **Σ at r_e unset** (β 20, 2 N·m), Σ 0 / 0.01 / 0.1 / 1 / 10°: 774.7 / 774.7 / 774.2 / 765.6 / 1047.3.
  η 98.827 / 98.828 / 98.828 / 98.819 / 98.277 %.
- **Σ at r_e 0.1: continuous, but steep** (fine phase scans, `dbg5`–`dbg7`). The field max is
  2309 / 2303 / 2318 / 2415 / 2484 at Σ 0 / 0.0001 / 0.001 / 0.003 / 0.01°, all at a_par.
  - A tilt of 0.01° is 0.9 µm across the face, against a 2 µm approach. An edge-loaded contact
    feels that, and a flank contact (flank convention) does not.
  - `t_v.py`'s Σ ≠ 0 rows run at r₁ + r₂, which is 1.45 µm inside a_par. That alone moves the Σ 0
    figure by +14 %. Compare Σ rows at one distance.
- **r_e** (β 0 / 2°, 20 N·m), r_e 0.4 / 0.2 / 0.1 / 0.05 / 0.02 mm:
  - β 0: 3831 / 4989 / 6585 / 8846 / 13397;
  - β 2°: 4109 / 5421 / 7241 / 9773 / 14870.
  - It is smooth and diverges about as r_e^−0.4…−0.5, since the load sheds as the edge sharpens.
    r_e → 0 is singular.
- **x** (β 20, 20 N·m), x −0.3 / 0 / 0.3 / 0.6:
  - r_e unset: 4825 / 2319 / 1720 / 1543.
  - r_e 0.1: 7687 / 6859 / 7415 / 6869. At x 0.3 the peak moves to mid-face, where the conjugate
    line crosses the wheel's tip edge.
- **Face width through ε_β = 1** (β 20, 20 N·m), b 8 / 9 / 9.185 / 9.5 / 10 / 12:
  - r_e unset: 2570 / 2445 / 2418 / 2334 / 2319 / 2178.
  - r_e 0.1: 7748 / 7238 / 7150 / 7026 / 6859 / 6765. At b 12 the peak passes to the pinion's round.
- **Load** is covered by 20 and 60 N·m in §2.1. **β** is covered to 2° in §2.1; β 20 is the rows
  above.

### 3.3 ISO 6336-2 at the standard examples (`t_iso3.py` → `iso3.txt`; μ 0, no relief)

Readings in MPa; the percentage beside each is against ISO. The pinion reads max(C, B), the wheel
max(C, D).

| Pair | ISO σ_H0 · B · D | Pinion, r_e unset | Wheel, r_e unset | Field max, r_e unset | r_e 0.1 m_n: pinion · wheel · field max |
|---|---|---|---|---|---|
| FZG-C-like 16/24 m4.5, 302 N·m | 1522.6 · 1629.5 · 1522.6 | 1648.5 (+1.2 %) | 1648.5 (+8.3 %) | 1770.9 (+8.7 %) | 1648.5 · 1648.5 · 4876 |
| 17/43 β20 m2, 60 N·m | 961.3 · 961.3 · 961.3 | 1051.3 (+9.4 %) | 960.2 (−0.1 %) | 1462.3 (+52 %) | 1077.8 · 975.8 · 4330 |
| 20/60 β15 m4, 500 N·m | 837.8 · 851.0 · 837.8 | 904.4 (+6.3 %) | 848.0 (+1.2 %) | 1194.2 (+40 %) | 923.7 · 868.3 · 4012 |
| ring 17/−43 x0/−.3, 60 N·m | 659.3 · 886.6 · **659.3** (Z_D 1) | 785.4 (−11.4 %) | 719.7 (**+9.2 %**; −7.0 % against the signed extension 773.9) | 12347 (interference) | 811.8 · 909.4 · 6151 |
| ring 17/−43 β15 x.3/−.3, 60 N·m | 639.1 · 647.2 · 639.1 | 683.6 (+5.6 %) | 656.0 (+2.6 %) | 1213.7 (+88 %) | 682.0 · 663.8 · 4005 |

- **The ISO-point readings survive the edge rule.** Stating r_e moves them by −0.2 … +3.9 %, because
  the round is a small relief near B and D. The interfering spur ring is the exception: +26 % at D.
- **The field max becomes an edge figure**, 3–5× ISO's largest. It is the quantity the expensive
  mode exists to expose.
- **Against round two's `iso2.txt`**, the r_e-unset rows agree within 1.5 % at the ISO points (17/43
  B: 1051.3 against 1036.8) and 3 % for the maxima. The difference is the valley and the
  resolution of the derived seed.

### 3.4 Rings (`v_ring.txt`; 17/−43 m2 b20, 60 N·m, μ 0.06)

| Ring | r_e | Pairs | η % | Field max | Where |
|---|---|---|---|---|---|
| β15 x.3/−.3 | unset | 2.633 | 99.464 | 1205.0 | pinion SAP r₁ 16.622, face end (ring's sharp tip) |
| β15 x.3/−.3 | 0.2 | 2.623 | 99.479 | 3975.4 | pinion's tip round r₁ 20.112 |
| β15 x.3/−.3 | 0.1 | 2.598 | 99.475 | 5488.6 | pinion's tip round r₁ 20.156 |
| spur x0/−.3 | unset / 0.2 / 0.1 | 2.030 / 1.967 / 2.006 | 99.36–99.40 | 12423 / 6194 / 13347 | pinion at r₁ 15.975 ≈ r_b: interference |

- The spur ring's figures are not monotone in r_e because they are governed by the involute's
  singular curvature at r_b, where the ring's tip meets the pinion. That is the interference of §3.1.
  The rating must flag it; no edge radius makes it a rating.
- kA ≤ 0: none.

## 4. Worm: involute wheel and matched wheel, side by side

The shipped worm: 1/40, d₁ 7, m 1, α_n 20, Σ 90, steel on C360, μ 0.06. η is taken at worm torque
2 N·m, and the maximum at wheel torque 54.953 N·m.

| Wheel model | Contact | Mean contacts | η % | Field max, MPa | Where |
|---|---|---|---|---|---|
| crate rating (pitch Hertz) | 1 point | 1 | 68.69 | 3487 | pitch point |
| involute wheel, valley field, r_e unset | points | 3.25 | 67.32 | 2212 | worm tip edge (r 4.500) |
| involute wheel, r_e 0.2 (both gears) | points | 3.13 | 67.77 | 8839 | worm's tip round (r 4.371) |
| involute wheel, r_e 0.1 | points | 3.21 | 67.66 | 12169 | worm's tip round (r 4.435) |
| **matched ZN** (hobbed by a ZN hob) | lines, 8.14 mm | 2.045 | 67.29 | 1842 | worm tip, flank convention |
| matched ZA | lines, 8.14 mm | 2.031 | 67.30 | 1875 | worm tip |
| matched ZI (the conjugate of the crate's ZI worm) | lines, 8.12 mm | 1.988 | 67.34 | 1766 | worm tip |

(Matched rows are round two's `matched.txt`, derived foundation; their kernel's b/h makes the old
width rule's error below 1e-5.)

- **Like for like.** Only the r_e-unset rows compare like with like. The involute field max is 20 %
  above the matched ZN max at the same kind of location, and η differs by 0.03 pt.
- **The matched wheel cannot take §1's edge rule.** It is the worm's envelope, known only on its
  contact line, so there is no gap off the line to carry the worm's round. Its maxima therefore stay
  flank-convention figures at the worm tip.
  - Giving it one needs the envelope's gap: the second-order surface at the line end, from the
    rank-one curvature update, extrapolated. That is a second geometry in `gap.rs`.
- **With a real edge the worm's peak is on its tip round**, 4–5.5× the flank-convention figure.
  - Point contacts on a 0.1–0.2 mm round are severe, and 28 % of the force is on edges.
  - For a worm the edge break is not a detail.

**What exposing the matched wheel would move:**
- **Rating:** the field max falls about 15–20 % at r_e unset, and the loaded length becomes 2 lines
  of 8 mm instead of 3.3 points.
- **Efficiency:** η changes by less than 0.05 pt against the involute field. Both are about 1.4 pt
  under the crate's pitch-point 68.69 %.
- **Lost with it:**
  - play and backlash, and misalignment or oversize-hob sensitivity, because there is no gap off the
    line;
  - the edge rule;
  - CLAUDE.md's "a worm is not special" (`screw.rs`'s one model for a worm and a crossed pair).
- **Needed with it:**
  - a wheel-tool parameter (a rack or involute hob, or the mating worm), crossing the boundary and
    the UI;
  - new golden worm files.
- **Cost:** 6–7 M meshing-equation evaluations per analysis. The contact lines are level sets of an
  explicit φ(u, v) (verifier 2), so bracketing from the previous phase cuts this about 20×.
- **Recommendation:** keep the involute wheel as the model. It is branch-free, has play, and takes the
  edge rule. Offer the matched set as a validation instrument, not a rating path, unless the owner
  wants hobbed-wheel ratings. Either way, the worm's r_e must be stated.

## 5. Cost (`t_cost3.py` → `cost3.txt`; whole analyses, 2 N·m, counted)

Rust estimate = counts × unit costs: local 35 ns, v 45, glob 25, G 15, a depth node 50, LU n³/3 at
1 ns. It is uncertain by about ×2.

| Mesh | r_e | States | Valley evaluations | Rust: transcendental · G · depth · LU | **Total** |
|---|---|---|---|---|---|
| spur 17/43 | unset / 0.1 | 180 / 300 | 0.13 M / 0.39 M | 0.72 · 0.17 · 0.66 · 0.03 s / 1.16 · 0.28 · 1.11 · 0.05 s | **1.6 / 2.6 s** |
| helical 17/43 β20 | unset / 0.1 | 298 / 479 | 23 M / 16 M | 9.1 · 1.1 · 4.4 · 0.2 / 6.9 · 1.7 · 6.6 · 0.3 | **14.8 / 15.5 s** |
| crossed Σ10 | unset / 0.1 | 179 / 538 | 17 M / 31 M | 6.8 · 0.5 · 1.9 · 0.1 / 12.7 · 1.0 · 3.7 · 0.2 | **9.3 / 17.5 s** |
| ring β15 | unset / 0.1 | 360 / 417 | 13 M / 7.9 M | 5.8 · 0.5 · 2.6 · 0.1 / 4.2 · 0.6 · 2.8 · 0.1 | **9.1 / 7.7 s** |
| worm 1/40 | unset / 0.1 | 299 / 659 | 38 M / 61 M | 14.9 · 0.6 · 3.6 · 0.1 / 23.9 · 1.2 · 6.6 · 0.2 | **19.2 / 31.9 s** |
| ring β15, field.py slices (derived foundation) | unset | 241, 4 pieces | – | 0.65 · – · – · – | **0.65 s** |

- **The ring's corrected slices row is 0.65 s over 4 pieces.** Round two's 4.6 s and 25 pieces were
  the withdrawn 2300×-compliant tooth.
- **The valley costs 5–10× round two's coupled field** off Σ = 0 on the flank: 16–61 M valley
  evaluations, from Brent's cross searches where the valley runs on an edge. A round adds pieces
  (tangency breakpoints): ×1.1–3 in states. Where the line stays on the flank (spur) the valley is
  three evaluations a station.
- **Fast route, Σ = 0.** Closed forms give the anchor, the valley on the flank (w = 0) and the valley
  on a round (the arc's tangency to a plane is closed form). The kernel dominates what is left.
  - **As built, the kernel is rebuilt every b-iterate:** about 27 builds a state (spur, counted), or
    7 ms a state. That alone is 0.7 s per mesh.
  - **The fast route needs a b-independent kernel, built once per line per state.**
    - The depth term takes the two-term Taylor form (error ≤ 1e-5 at b/h ≤ 0.1, `kernel3.txt`).
    - G takes its far field ln 4r + ½ + 1/(16r²), exact to 1e-9 beyond r 50, as A + b² B.
    - Only the near band is re-evaluated with b.
  - That costs 576 × 2 × (2 × 15 + 60) ns ≈ 0.1 ms a line-state, and LU about 0.15 ms a state.
  - About 100 states × 2.5 lines comes to **≈ 45 ms coupled**, inside the verifier's 35–80 ms band.
    The verifier's 80 ms is the as-built kernel with one build a state.
  - Σ ≠ 0 and the worm keep the cross searches: 2–5× more.
- **Verdict (unchanged in kind):** for the final solve and validation, once per mesh per load case,
  not for the search.

## 6. Open issues (size and sign)

- **The edge figure is now a property of the stated form**, and it is large. Unrelieved and
  edge-broken, the peak is 1.5–2× (spur and helical) and 4–5× (worm) the flank-convention figure.
  - The default rating stays at ISO's points, as decided.
  - The field max, with where it sits and on what (flank, round, face end), is reported beside it.
- **A curvature step at the round's tangency.** Pointwise Hertz steps there, and the supremum is the
  round-side limit.
  - A contact straddling the tangency would be lower by an amount of order b · ∂p/∂s, which is small
    at b/r_e ≤ 0.17 but not measured.
  - The relief's end at u = L_a is a smaller step, about 1 % in p, and is not a breakpoint.
- **A reporting glitch in `phase.analyse`:** when the maximum is a piece-end limit, its "where"
  label can come from the other side of the breakpoint. For example, the C_a 0.5 µm row reads
  "flank" at the value of the round. The values are right; Rust should carry the where with the
  value.
- **Face ends are square**: +2…+6 % end effect, and the peaks above sit at face ends. An end relief or
  chamfer would be the same form rule along the lead.
- **Tooth coupling along the face** (plate action) is not modelled.
- **The ring's tooth** is the rack tooth on a stated rim.

## 7. Rust migration plan (Stage 3; changes from round two in bold)

1. **`tools/contact_oracle.py`** (from `oracle.py`) is the independent instrument.
2. **`elliptic.rs`** gains `aspect`, `hertz_shape` (C) and `strip` (G, with the R_D derivative).
   **The width quadrature takes the Bernstein-ellipse node count, capped.**
3. **`form.rs`** (new): a gear's tip form F(σ) with its slope and curvature. It holds relief, round
   and land, all member inputs (principle 10), next to `params.rs`'s rim thickness.
4. **`gap.rs`** (new): the gap with forms, the foot point, the anchor, **the valley** and the clip
   (faces and roots). It retires `screw.rs`'s `ZoneLimit` family and `contact.rs`'s path limits.
5. **`tooth_compliance.rs`**: the derived foundation.
6. **`field.rs`**: coupled lines on valley arc length, the active set, Δ in closed form, **per-station
   curvature and C**, **round-tangency breakpoints**, means and maxima.
7. **Rating:** ISO's points by default (Z_D = 1 for internal wheels). `MeshReport` gains the field max,
   where it sits, **its surface (flank, round, face end)**, the edge share of the force, r_e and C_a.
8. **Laws:** β continuity at r_e unset, 0.2 and 0.1 and at C_a 2 and 5 µm; the edge peak equals 2-D
   Hertz on r_e; the sharp convention reproduces round two; plus round two's laws.
9. **Matched worm set:** only if the owner chooses it (§4).
