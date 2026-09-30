# contact-verify5: adversarial check of work/contact-model.md round four

Author: contact-verify5, 2026-09-29. Read-only on the repo, no cargo. Every process was capped at 1.8 GB
(`RLIMIT_AS`), with at most 4 per pool. Scripts and raw outputs are in
`~/.cache/gearcalc-work/contact-verify5/`.

- **Own code, sharing nothing with the prototype:**
  - `bem2d.py`: a 2-D frictionless half-plane contact. It uses piecewise-constant pressure, the exact
    log-kernel panel integral, a Levinson Toeplitz solve and a contiguous active set. It reproduces
    Hertz p0 to 2e-8.
  - `profile_c1`: the exact C¹ section. A circle meets a round of radius r_e, joined C¹ to a flank
    circle, with the tip land beyond the round's 64° turn.
  - `t_zi.py`: a numerical check of the ZI helicoid.
  - `jumps.py`: a jump detector for phase scans.
- **Reused as an independent cross-check:** contact-verify4's `v1_spur2d.py` (its loads, exact round and
  tooth), in `t_spur2d.py`.
- **The subject's code run as-is.** This is evidence about it, not a check of it.
  - `a1_det.py`, `a2_seed.py`, `a3_flip.py`, `a3_lin.py`.
  - `b_scan.py` gives dense phase scans in `b_*.jsonl`, with `zoom*.py` to bisect a jump.
  - `t_c2d_check.py`, `t_c2dfrac.py`, `t_unitcost.py`.

## Verdicts

| # | Claim | Verdict |
|---|---|---|
| 1 | Line continuous in every input; the +18 % case agrees to 8 digits; linear in δ 1e-9…1e-3 | **PARTLY.** The valley is continuous and seed-independent, but the *state* is not. A per-panel friction sign jumps D by 0.1–0.2 % about 22 times per pitch. |
| 2 | Closed-form 2-D across contact; 0.12 %; spur edge 6585 → 5957 | **CONFIRMED** |
| 3 | β jump with r_e unset; narrow peak found; refuse r_e unset because r_e → 0 diverges | **CONFIRMED** |
| 4 | Σ series, face width through ε_β = 1, x monotone | **CONFIRMED**, with one small doc omission |
| 5 | Face ends as free surfaces by images | **PARTLY.** The mirror imposes a condition. The "≤ 4.6 %" is not a bound. |
| 6 | Tooth: FE 17.06 / derived 15.60 / verifier 11.61; derived 7–13 % too compliant | **PARTLY.** The FE is credible and the verifier's tooth is wrong, but the 7–13 % holds only at the hidden h_f = 1.4. |
| 7 | Matched wheel: "no new solve"; play; edge peaks 31–33 % below | **"No new solve" is REFUTED.** Play is **CONFIRMED** (ZI and axial). The −31–33 % is **not established** on round four's field. |
| 8 | Cost 124–986 s Python; Rust 5–24 s (2–8 s) | **CONFIRMED** as a measurement. The Rust figures are **PARTLY** confirmed: an estimate within about ±2×; the 2–8 s was not measured. |

## 1. Continuity (most important)

### The verifier's case (`a1_det.txt`)
The two centre distances are 31.925333 and a_par − 1.74e-7.
- They differ by **2.77e-10 mm**, not 2.8e-13 mm. The verifier's unit slip is repeated in `trace.py`'s
  docstring; the doc's §2.1 text has it right.
- At 0.8875 p: 1962.0756985694 against 1962.0756986405.
- At 0.7425 p: 1869.9351102 against 1869.9351089.
- ΔD = 0.343 × Δa, and the lines are 4.56499999 / 10.56057479 / 1.19624196 mm in both.
- A genuine 2.8e-13 mm change (a_par against a_par + 2.8e-13) moves p by 2.6e-9, which is the noise floor.

The claim holds to 9–10 digits.

### Seed independence (`a2_seed.txt`)
I displaced the anchor the subject returns by 1.37, −2.6 and 9.3 dz along d, by 0.02 mm across, and by
(0.5 dz, −0.05 mm), keeping it inside the field. Over 48 phases × 4 bases (h20, c10, s0, n20):
- |Δp/p| ≤ 1.3e-8 and |ΔD/D| ≤ 2e-11.

The valley is intrinsic.
- Caveat: a seed pushed more than EXT (0.2 mm) outside the field ends the march at once and drops the pair
  (D +218 % in one test). A real anchor is never outside, so this is robustness only.

### New defect: the state is discontinuous at every station's pitch-line crossing
Found by dense phase scans (1000 phases per pitch; `jumps.py`) and bisection (`zoom*.py`).

**Where it shows.** h20 (β20, r_e 0.1, 2 N·m, Σ 0), the doc's own base:
- **22 jumps per pitch in D (1.0–2.0e-3) and in the force F.**
- Across one bisected phase width of 9.3e-13 p (t/p 0.02394367254), D goes 1.28845 → 1.28670 µm (−0.136 %)
  and pmax goes 1742.83 → 1741.75.
- The same pattern appears at 20 N·m on every x/face scan (20–24 per pitch).
- None appears at Σ 1° or 10°, where sliding never vanishes.

**Cause.** `field.arms` takes the friction direction `vt/|vt|` at each station's point.
- When a station crosses the pitch line, its friction flips: one panel's 2μ × arm.
- The continuum integral is continuous; the midpoint rule is not.
- Diffing every station array across the jump (`zoom_dbg2/3.py`): only `tau` and then q, b and D differ.
  Nodes, points, sections and faces agree to 1e-9.

**Consequence for the §2.1 law** (`a3_flip.txt`). At that phase, a **1e-9** change of each input gives
|ΔD/D| **1.36e-3** and |Δp/p| **6.2e-4**:

| Input | a | x₁ | b | b₁ | β | T | Σ |
|---|---|---|---|---|---|---|---|
| |ΔD/D| at δ = 1e-9 | 1.36e-3 | 1.36e-3 | 1.36e-3 | 1.36e-3 | 1.36e-3 | 1.36e-3 | 1.2e-3 |

- That is six orders above linear.
- The 24-phase table passes only because no sampled phase sat within δ of a crossing.
- At generic phases the other inputs are linear down to a ~5e-9 floor (`a3_lin_*.txt`: T, μ, r_e, x₁, b₁;
  h20 and c1).
- Spur crosses the pitch point with the whole line at once: F jumps 4.4 % (s0 scan, t/p 0.749 → 0.751).
  That is Coulomb physics in this model, but it contradicts "continuous in every input" too.
- **Fix:** integrate μ sgn(v_s) over each panel exactly. v_s is linear along a panel, so split the panel at
  its zero.

**Also discontinuous by construction.** The proposed MeshReport's **flank and edge maxima** are maxima over
a class ("where the strip's peak sits"), so they jump when the class switches:
- s0: pedge 2958.6 → 0 with pflank 1510 → 2770.
- c10: pflank 427 → 1513 between adjacent phases.

The field max is continuous. Near the peak it carries a panel-scale ripple of 0.1–0.2 %.

**The doc's own table is not proportional everywhere.**
- Spur "β from 0": 1e-7 → 4.3e-9 but 1e-5 → 7.7e-6.
- "Σ from 0": 1e-5 → 1e-6 but 1e-3 → 2.8e-4.

These are the narrow edge window and a superlinear onset, not the stated "floor".

## 2. contact2d (`t_c2d_check.txt`, `t_spur2d.txt`)

**Against the own BEM** (M 1000/2000/4000; q 100 and 300; r_e 0.2/0.1/0.02; tT −30…+30 µm):
- On the same piecewise-quadratic section, contact2d agrees to **< 1e-5**.
- On the exact C¹ circle geometry it is **0.01–0.15 % low**: the parabolic idealisation of the round,
  sign known.
- For example, at q 100 and tT 0: 4841.2 against 4843.1. At q 300 and tT +10 µm: 11167 against 11183.

**Spur, the verifier's loads with the own BEM on the exact local geometry** (2000 phases):

| Case | Pointwise → exact 2-D |
|---|---|
| 20 N·m, r_e 0.2 | 5001 → **4673** |
| 20 N·m, r_e 0.1 | 6631 → **6075** (−8.4 %) |
| 60 N·m, r_e 0.1 | 11446 → **10277** |

- These match `x_v1c2d.txt` (4672 / 6072 / 10263) to 0.14 %.
- The prototype's field 5957 is 1.9 % below 6075, which is plausible.
- The peak sits about 0.2 µm on the flank side of the contact centre, 5.9 µm from the tangency, with
  c 11.1 µm.
- C¹ gives a finite pressure that converges in M.

## 3. The β jump and r_e unset (`b_b01_*.jsonl`, `t_spur2d_re.txt`)

**The peak is found.** At β 0.1° with r_e unset, a brute scan at a step of 1e-5 p gives 3649.9 at
t/p 0.88364, against analyse4's 3650.7.
- The peak is a kink with FWHM 0.0025 p, which agrees with the doc's ±1e-3 p.
- A whole-pitch scan at 1e-3 p finds nothing higher. It reads 3600.2: a 1e-3 grid under-reads by 1.4 %.
- The doc's small-β values rise linearly (3202.7 / 3207.8 / 3253.0 / 3650.7), so analyse4 finds the window
  at 0.01° and 0.001° too.

**r_e → 0 diverges** (own model, 20 N·m):

| r_e (mm) | 0.2 | 0.1 | 0.05 | 0.02 | 0.01 | 0.005 | 0.002 |
|---|---|---|---|---|---|---|---|
| Exact 2-D max (MPa) | 4673 | 6075 | 8004 | 11542 | 15094 | 19569 | 27345 |
| Pointwise over exact | 0.93 | 0.92 | 0.90 | 0.85 | 0.80 | 0.74 | 0.66 |

- The growth goes as about r_e^−0.37.
- Pointwise Hertz overstates more and more as r_e falls.
- Refusing the flank-convention figure in the expensive mode is right. Requiring a C¹ form with no hidden
  default is consistent with rules 1 and 5.

## 4. Sweeps (grids of 500–1000 phases, against analyse4)

**Σ.** The grids read 2019.2 / 2021.2 / 2290.9 / 6092.1 at Σ 0 / 0.01 / 1 / 10°, against analyse4's
2019.7 / 2021.4 / 2291.0 / 6092.1.
- analyse4 finds the maxima.
- The doc's 201-grid (2008.7 at 0.01°) was simply too coarse.
- No D or pmax jumps at Σ 1 or 10.

**x.** At 0.15 and 0.45 the grids read 5854 and 5677, which lie between the neighbours. The series is
monotone.

**Face.** At b 9.1 / 9.185 / 9.27 the grids read 6205 / 6173.0 (analyse4 6173.5) / 6143. The series is
smooth through ε_β = 1. There is a slope kink near 9.3–9.5 where the max moves from the face end to
mid-face. That is a max of two branches, not a jump.

**Omission.** The doc does not mention that `sweep4_edge` β 0 rows carry `nonconv` 1–3.

## 5. Face ends and images

**What the mirror imposes.** A mirror image about a plane gives zero shear *and* zero normal displacement
there: a frictionless rigid wall. A free end has zero normal *stress* instead. So the images impose a
condition; they do not model a free surface.

**Aligned spur ends.** Load plus image is an infinite plane-strain line. The 5-digit convergence at
24/48/96 panels is therefore by construction and says nothing about the free end.

**Sign.** Removing the wall can only soften the end (minimum energy), so the images overstate an end peak.
The sign is right.

**Size.** The doc's "high by at most √(1−ν²) = 4.6 %" is a plane-stress heuristic. It excludes the free
corner's 3-D compliance and the lower end q in a coupled line, so it is not a bound and is unmeasured.

**Helical and crossed lines.** `kmatrix` images the load along the line about the crossing point, which is a
plane normal to the line, not the face plane. For β_b 18.75° the true image line is inclined 37.5° to that.
The maxima sit exactly at these face-end stations (h20: `edge2 face`, z 4.99).

**Unequal faces.** Each body is imaged at its own face crossings.
- The narrower body's end is again a wall.
- The wider body continues as a half-space, a real edge that the Winkler tooth term bounds.
- The graded +3 % extrapolation is reasonable.

## 6. Tooth stiffness

**The FE is credible.**
- L1/L2/L3 agree within 0.06 %.
- It reads Timoshenko 0.995 and Hertz 0.04–0.08 %, and whole gear against sector.
- It shares stiff.py's outline, which is a common mode and unchecked.
- The verifier's 11.61 is wrong: an idealised root gives θ_f 0.118 against 0.177, a Sainsot foundation
  1.47× too soft. It is 26 % below ISO theory.

**The derived tooth's error depends on the body** (c′, from `fe_tooth.txt`):

| h_f | FE | Derived | Derived against FE |
|---|---|---|---|
| 1.4 | 17.06 | 15.60 | −8.6 % |
| 2.0 | 14.61 (14.94 less far rotation) | 14.87 | +1.8 % |
| 4.0 | 13.41 (less far rotation) | 14.38 | +7 % |

- The derived foundation under-responds to the body: −8 % from h_f 1.4 to 4.0, against the FE's −21 %.
- The "7–13 % too compliant" holds only at `derive_foundation`'s hidden hf_ratio 1.4 (and rim_modules 3.5).
  Under the owner's rules both are magic numbers; the gear body should be an input.

**ISO.** c′_th 16.03 × C_B 0.975 (h_fP 1.25) = **15.63** is ISO's theory for this rack.
- The derived tooth matches it to 0.2 %. The FE is +9 %.
- ISO's c′ of 12.50 adds C_M 0.8 (measured over theory), which no 2-D model contains.
- So for absolute compliance, which is what sizes relief, the spread is about 25 %. The doc's "−14 % stands"
  is not settled.

## 7. Matched worm wheel

**"No new kind of solve" is refuted.** RESTART.md:48 says: "A matched set is exposed only if it needs no
extra branch or solve." `mw_field.MWField` adds:
1. At every wheel-distance evaluation (6.8–7.0 M per analysis), a closest-point projection. It is a Newton
   in (v, φ, d) with a forward-difference Jacobian, backtracking, a box and a cap of 30. **It hit the cap 8
   and 31 times** in the edge runs (`mw_run_edge.txt`).
2. A new seed search: a conjugate-curve scan plus Brent.
3. A parabolic across-minimiser that **falls back to bracket plus Brent in 20–26 % of valleys**.
4. A hob-tip boundary clip, and handling of the developable's root sheet.

The involute wheel's d₂ is closed form. The comparison with the anchor search does not hold: that is one
solve per pair per phase, while this one sits in the innermost loop.

**Play.**
- ZI: my own numerical check gives n·(z×X) = r_b sin λ_b constant to 1e-10 on the involute helicoid, while
  a ZN-like helicoid varies. The law j = s_h/(cos α_n cos γ) is confirmed.
- Axial thinning is exact by kinematics.
- The centre-distance factor 0.54× is a minimum over 8 sampled phases (range 0.538–0.640), not phase-exact.
  I did not check it independently.

**Edge peaks: it matters.** They were computed on MWField(VField), round three's field. That field has the
seeding branch, pointwise Hertz at the step and a singular square end.
- Round four's own involute worm at the same load reads 9640.7 (`cost4.txt`, worm torque 2 N·m, on 1).
  Round three read 12013. That is −20 %.
- Like for like, the matched 7976 (on 1) is **−17 %** below the involute wheel, not −31–33 %.
- The matched figure has not had the step correction either, so its own change is unknown.

## 8. Cost

**Measured.** The Python CPU figures are consistent with my own scans (0.37 s per spur state and 0.66 s per
helical state).

**The Rust arithmetic reproduces.** Spur: 0.16 + 0.93 + 3.7 + 0.03 + 0.32 ≈ 5.1 s. Worm: 24.3 s.

**Unit costs, scaled by Python ratios to the distance evaluation's 35 ns (27× Python):**

| Operation | From Python ratio | Estimate used |
|---|---|---|
| G | 22 ns | 15 ns |
| Depth node | 16 ns | 50 ns |
| contact2d, average | about 4 µs | 2 µs |

- For contact2d, 8–18 % of calls take the 2-D path, at 1.0–1.6 ms of Python each. contact2d is 4–8 % of
  state CPU.
- The net is within about ±2×. The depth term may be over-estimated.
- The "2–8 s" figure for the b-independent kernel was not measured with images.
- The matched wheel's projection cost is not in §2.7's table.
