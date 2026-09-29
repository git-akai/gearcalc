# Unified contact model: phase-exact prototype

Author: contact-proto, 2026-09-26/29. Read-only on the repo; pure Python (no numpy).
- **Scripts and their outputs:** `~/.cache/gearcalc-work/contact-proto/` (README there).
- **Built on:** `work/plan.md` §5 (the owner's rulings), `work/spike-contact.md` and
  `work/review/spike-verify.md`.

**Verdict.** The model is right wherever it can be checked:
- **ε:** ISO 21771's value exactly.
- **Spur:** ISO 6336-2's pitch point and point B exactly.
- **Helical and ring:** the pitch value within +1.0 to +3.6 % of ISO with Z_ε (elastic teeth).
- **Worm:** the crate's curvatures, ellipse, efficiency table and locking thresholds, to the
  printed digit.
- **Continuity:** continuous through Σ = 0 for shifted pairs, where the spike's construction was
  not.

It is 10²–10³ times too slow for the search hot path. **Recommendation:**
- the search keeps closed forms;
- the field runs once per solve, for the rating and the report;
- the field is also the validation instrument, held to the closed forms by laws (§7).

**Three results need the owner before any default is set** (§6):
- the field maximum sits at the unrelieved start of active profile, +45–54 % over ISO on
  helicals, and it jumps +30 % between β 0 and 0.1°;
- the "tips relieved" option should be a relief profile, not a classification;
- the crate's worm wheel is an involute helical gear, which nobody hobs.

## 1. The model: every mesh, one construction

Signed tooth count (z < 0 is a ring) and the shaft angle Σ are parameters; nothing branches on
the kind of mesh. Files: `field.py`, `phase.py`, `carlson.py`, `stiff.py`.

1. **Exact rigid gap.** Rotated involute helicoids are parallel surfaces, so the normal distance
   from any point Y to member i's flank is closed form:
   ```text
   d_i(Y) = k_i (v_i(ρ) ∓ (ψ − n p_i)),    k_i = r_bi cos β_bi
   v(ρ)   = e/(2|r|) − sgn z (inv α_t − inv α_ρ),    e = m_n (π/2 − 2x tan α_n)/cos β
   ```
   The gap between the facing flanks near Y is g(Y) = d₁(Y) + d₂(Y) + R₁ + R₂. R_i is an
   optional tip relief, C_a(1 − u/L_a)₊², with u the depth below the tip. There is no parabola
   and no line of action.
2. **One line per tooth pair.** Tooth j of member 1 always meets void k₀ − sgn(z₂)·j of member 2.
   - The anchor A is the minimum of g over the pair's field: the flank patches of both members,
     faces and tips included. It comes from a damped 2-D Newton plus bracketed 1-D edge minima on
     a continuous feasibility margin.
   - The line is A + y·e_L. e_L is the minor eigenvector of K = κ₁u₁u₁ᵀ + κ₂u₂u₂ᵀ, where
     κ_i = sgn z_i · cos β_bi/ρ_ti (negative on a ring) and u_i = n × (axis_i projected).
   - The small root is taken stably, κ_L = det K/κ_A. It is exactly 0 at Σ = 0, where e_L is the
     contact line; at Σ > 0 e_L is the Hertz ellipse's major axis.
3. **Clip, closed form.** Two face slabs (linear) and four radial bands (quadratics): the tip
   and the flank start of each member. The flank starts at the generated form radius, not at r_b.
   The clip keeps the component that contains y = 0 and tags each end with its constraint.
4. **Load.** Winkler slices along each line: q(y) = (Δ − g(y))₊/c(y).
   - Δ is one approach per phase, from the **torque balance with friction in the normal force**:
     T = Σ∫q τ dy, with τ = ((Y−O) × (∓n ∓ μŝ))·axis.
   - The compliances are in series: c = c_H + c_t1 + c_t2.
     - c_t is `stiff.py`'s potential-energy tooth: bending, shear and compression over the
       generated profile, fillet included, plus the foundation term of Sainsot, Velex and Duverger
       2004. A ring is treated as a rack tooth, as ISO 6336-1 uses z_n2 = ∞.
     - c_H is the contact term, B″, below.
   - The pressure is p = C·√(q E* κ_A/π), where C = p₀/p_line comes from the pair's own ellipse.
     C is 1 for a line.
5. **Phase.** Every phase function is piecewise smooth over one pitch of member 1.
   - **Breakpoints** are where the signature changes: the set of loaded pairs, or the constraint
     at a loaded end (face, tip, root, gap). They are bracketed on a 24-point scan and bisected to
     1e-10 of a pitch.
   - **Means** are Gauss–Legendre per piece. This is exact for the piecewise-constant pair count
     and spectral for the rest.
   - **Maxima** are the piece ends plus the interior stationary points, found by golden section.
   - **ε** is the mean number of loaded pairs.

**The contact compliance B″ (new).** The spike's k_W = 3πE*/(2R_D(β²,0,1)) is exactly the
half-space *surface drop* from the patch centre to its end, u₀ − u_a. It is right for the patch
shape, but it tends to 0 like 1/ln(1/β) as Σ → 0. A finite tooth holds the material below depth
h, Weber's distance to the tooth centreline, and the tooth model already carries that material.
So subtract the same drop measured at depth h. Per body, per unit centre line load q₀ = πp₀b/2,
with k_b = 2(1−ν²)/(πE), from Dyson's potential of the Hertz pressure
(u_z = (2(1−ν)ψ − zψ_z)/4πG):

```text
c_H/k_b = (u₀ − u_a) − (w₀ − w_a)
  u₀ = a R_F(a²,b²,0)            u_a = a [R_F(a²,b²,0) − a² R_D(b²,0,a²)/3]
  w₀ = a [R_F(A,B,H) + ν/(3(1−ν)) h² R_D(A,B,H)],       (A,B,H) = (a²+h², b²+h², h²)
  w_a = (a/2)[2R_F(Aλ,Bλ,λ) − (2/3)a² R_D(Bλ,λ,Aλ) + (2/3)h² R_D(Aλ,Bλ,λ) ν/(1−ν)],
        λ = (h² + √(h⁴ + 4a²h²))/2
```

- **The Winkler term is recovered exactly.** u₀ − u_a = R_D(β²,0,1)/3, which is 1/k_W; checked to
  the digit.
- **Small patch (a ≪ h):** B″ → 1/k_W, so the Hertz ellipse's length, shape and peak come out
  exact.
- **Line (a → ∞, Σ = 0):** B″ → asinh(h/b) − ν/(1−ν)·h/(√(h²+b²) + h). That is the exact 2-D
  depth-referenced line formula; its b ≪ h limit is Johnson's ln(2h/b) − ν/(2(1−ν)), matched to
  1e-6.
- **Convergence to the line** goes as −0.29 h/a (`t_limit.py`): −2.3e-4 at a/h = 1250, −2.3e-7 at
  1.25e6. Over the same range the Winkler term alone diverges, 10.3 → 17.2.
- **So Σ → 0 is a value.** R_D/3 ≈ ln(4a/b) − 1 and its depth-h counterpart cancel.
- **The prototype** evaluates the line limit when κ_L = 0 exactly. That is a removable
  singularity, not a model branch. In Rust it is line(h,b) + ρ₁(β) − ρ₂(h/a), written as a
  regularised special function (§7).

**Aspect ratio, the one inverse.** q = A/B = R_D(0,s,1)/R_D(0,1,s), with s = κ² (DLMF 19.25.1). It
is solved by bisection on ln s over [ln q − 60, 0]. On 1e-16..1 q(s) is monotone and s ≤ q(s),
which bounds the bracket (`t_limit.py`).

**Internal gears use the same formulas.**
- Signed r, r_a = r + m(h_a + x) and a = r₁ + r₂ carry the ring.
- The ring's flank is concave through κ₂ < 0.
- It turns with its pinion: the ratio −z₁/z₂ is positive.
- ISO's x sign applies, and the tip band is on the far side.

## 2. The five defects left at the pause, and one more

1. **The k branch at Σ → 0.**
   - Replaced by B″, which is finite and continuous at Σ = 0.
   - The predecessor's "the series stiffness tends to the tooth's" was wrong: with k_W it tends to
     **0**. With B″ it tends to 1/(c_line + c_t), which is ISO's picture.
2. **Play.**
   - Exact: the minimum over phase of the anchor gaps, which is the oracle's computation.
   - Closed form, for the search: derived from the gap parabola,
     j = 2 sin α_n (a − a_lin) + K u²(a − a_ref)². Using d(u(a − a_ref))/da = 1, it gives
     **slope = 2 sin α_n (1 + 2D₀u/(a₀ − a_ref))**, which needs no parallel slope.
   - Against the exact slope (`play.txt`), derived / spike's interpolation: Σ 0 +0.60/0.00 %;
     Σ 1° +0.32/−0.12; 5° −0.61/−0.53; 10° −2.82/−2.35; 30 mm face −1.16/−0.57 and −0.39/−0.39;
     x −0.3 −1.26/−1.42; x 1 at Σ 6° −2.64/−3.66 %.
   - The derived law replaces the interpolation, but it is no more accurate: both carry the
     parabola's error, up to −3.7 %.
3. **The ε staircase.** The breakpoints are now exact (§3.2).
4. **The oracle** (`oracle.py`) is new, independent and exact per phase.
   - Interior stationary points come from a 2-D Newton; edge minima on both bodies (tip, faces)
     from bracketed golden section to 1e-13. Edge-on-edge contacts are the ends of the feasible
     intervals.
   - The minimum over phase comes from every local minimum of a scan, refined. A minimum of a sum
     of lower envelopes cannot sit at one of their kinks.
   - It **includes internal gears** through signed z.
5. **Base-cylinder tangency** is no longer needed: the anchor search replaces the line of action.
6. **(New) The spike's anchoring is wrong for shifted near-parallel pairs.**
   - As Σ → 0 with a ≠ a_ref, the line of action recedes to z* ~ D/Σ. Its lines, run back along
     e_L, land in the face on the base-tangent plane at the *reference* α_t, not the operating
     α_wt.
   - At x .5/.5, the spike's Σ → 0 ε_path × cos²β_b is 1.13, against the parallel 1.23 at a_lin.
   - Anchoring at the in-field minimum of the exact gap cures it: L/(b/cos β_b) is 1.36421 at Σ 0
     and 1.36425 at 0.01°.
   - The verifier's rows were all x = 0, where a_lin = a_ref, so they could not see it.

## 3. Validations

### 3.1 Oracle (`t_oracle1.py`, `t_oracle2.py`, `t_a0.py` → `a0.txt`)

Oracle play against ISO 21771 j_n (µm): 17/23 spur, a_par + 0.05: 34.5219 µm, 34.5219; 17/43 β20, x
.5/.2: 38.6366, 38.6366; ring 17/−43 β15, x .3/−.3, a_par − 0.05: 33.7489, 33.7489; ring 20/−60 β20:
37.7174, 37.7174.

- **The verifier's a₀ table** agrees on all 10 shared rows, to the printed 1e-6 mm.
- **New row, x 1/1 at Σ 6°:** the u² law is +6.40 µm off, 1.8 % of D₀.
- **Found: the unshifted ring 17/−43 has involute interference.** The ring's tip meets the line
  of action beyond T₁: it needs |r_a2| ≥ 20.687 and has 20.5. The oracle measures −1.63 µm; a ring
  x₂ of −0.3 clears it, bar −0.04 µm.

### 3.2 ISO 21771 ε (`t_field2.py`)

Case: 17/43 β20, b 10, rigid, light load.
- **Breakpoints** at 0.41581, 0.50451, 0.90770 and 0.99641 of a pitch. Their spacings, 0.0887,
  0.4032, 0.0887 and 0.4194, are the closed-form corners.
- **ε_γ:** 2.58060 against 2.580562. The 3.9e-5 is tip contacts loaded at Δ ~ 1e-10.
- **Mean length / (b/cos β_b):** 1.4918785 against ε_α 1.4918785 (difference 1.1e-8).

### 3.3 Tooth compliance against ISO 6336-1 (`stiff.py` → `stiff.txt`, E 206 GPa, q 300 N/mm)

At 17/43 x0, model against ISO: c′: 13.65, c′ 12.51 (C_M 0.8, C_B 0.975), +9 %; c′: 13.65, c′_th 16.03, −15 %;
c_γ: 19.79, 18.33 (from c′), +8 %; c_γ: 19.79, 23.50 (from c′_th), −16 %.

- **The rounded gate holds:** c′ ≈ 14 and c_γ ≈ 20.
- **Over a 5 × 3 grid** the model sits −9…+12 % around c′ and always 13–28 % below c′_th, which is
  where ISO's C_M puts real gears.
- **Split at the pitch point:** foundation 44 %, shear 27 %, contact 21 %, bending 7 %,
  compression 1 %.
- **Module-independent**, as a law.

### 3.4 ISO 6336-2 σ_H (`t_iso.py` → `iso.txt`, `iso_ring2.txt`; nominal, K = 1, steel, relieved)

**FZG-C gears** (16/24, m 4.5, x .1817/.1715, b 14, a 91.5, 302 N·m):
- **Pitch:** the field gives 1655.6 MPa, exactly ISO's Z_H Z_E √(F_t(u+1)/(d₁bu)), because C lies
  in the single-pair zone. ISO applies Z_ε 0.9197 and gets 1522.6, so the field is **+8.7 %**.
- **Field maximum:** 1771.8 at r₁ 35.403, which **is ISO's point B**. ISO's Z_B σ_H is 1771.8
  (0.0 %).
- **Loaded pairs:** 1.4624 = ε_α.

**Helical and ring pairs:**

| Pair | Pitch against ISO with Z_ε, elastic (rigid) | Field max against ISO σ_H0 |
|---|---|---|
| 17/43 β20 (m 2, b 20) | **+1.0 %** (−4.2 %) | 1479 against 961, **+54 %** |
| 20/60 β15 (m 4, b 40) | **+3.6 %** (−1.1 %) | 1213 against 838, **+45 %** |
| ring 17/−43 β15 (x .3/−.3) | **+1.6 %** (−0.1 %) | 1234 against 639 |

- **The helical maxima sit at the pinion's start of active profile**, for example r₁ 16.95 on a
  16.90 form radius. There κ ∝ 1/ρ_t, and ISO does not look there.
- **For comparison, the crate** is +32.7 % over ISO on a full-overlap helical (T08.3).

### 3.5 The shipped worm (`t_worm.py`, `t_worm2.py`)

The worm: 1 start, 40 teeth, d₁ 7, m 1, α_n 20°, Σ 90°; 4340 on C360, E* 70 811 MPa.
- **Curvatures:** the pair whose anchor passes the pitch point (to 1.6e-9 mm) has κ_L 0.081615
  and κ_A 0.173242, **the crate's**.
- **Hertz, worm torque 2 N·m, no friction:** 1.8448 × 1.1184 mm and 3940.3 MPa, the verifier's.
- **Hertz, wheel torque 54.953 N·m with μ 0.06, as the crate rates:** F_n 2951.2 N, 1.6328 ×
  0.9898 mm, **3487.4 MPa, the golden**.
- **Winkler slice:** q₀ 2711.18 against πp₀b/2 = 2711.18; ∫q = 2951.2 N.
- **The field's own force balance at the pitch point** equals the golden table to the printed
  digit:
  - worm driving, at μ 0/.02/.06/.10: **100.000 / 86.882 / 68.691 / 56.677 %**;
  - wheel driving: **100.000 / 84.993 / 55.254 / 25.874 %**;
  - locking μ **6.5104 forward and 0.1356 back** (BS 721).
- **The full field, with sharing** (phase-exact; `worm_field_*.txt`), η at μ 0.06, field max on
  the wheel torque: rigid, pure k_W: 2.55 pairs, 68.02 %, 3137 MPa; rigid, B″: 2.51, 68.05 %, 3314;
  **elastic: 3.31, 66.49 %, 2172**; elastic + 5 µm relief: 3.29, 66.67 %, 2105. Against the pitch
  point (68.69 %, 3487.4): sharing costs 0.6–2.2 pt of efficiency (the spike found ≈1) and lowers
  the peak 5–38 %; the peak sits at the worm's tip (r₁ 4.50) unless relieved.

### 3.6 Continuity sweeps (`sweep.py` → `sweep_all.txt`, `sweep_fine.txt`, `relief.txt`)

Setup: 17/43, m 1, 2 N·m, μ 0.06, elastic teeth, each pair at its exact zero-play distance.

**Σ at x 0** (Σ°: L/(b/cos β_b), η %, field max MPa): 0: 1.49188, 98.850, 768; 0.01: 1.49188, 98.851, 762; 0.1:
1.49170, 98.849, 762; 1: 1.49019, 98.818, 760; 3: 1.48731, 98.710, 777; 5: 1.48522, 98.581, 839; 6:
1.47376, 98.516, 878; 7: 1.41019, 98.448, 921; 8: 1.32540, 98.377, 962; 10: 1.16104, 98.230, 1040;
12: 1.02995, 98.072, 1104; 20: 0.73692, 97.368, 1298; 45: 0.46038, 94.663, 1704; 90: 0.44132,
80.571, 1783.

- It is continuous. The patch becomes shorter than the face between 5 and 12°.
- Jitter of ±1 % in the maximum below 2° comes from the 24-point scan.

**Σ at x .5/.5:**
- L: 1.36421 → 1.36425 at 0.01° → 1.36479 at 0.5°, which is **continuous**.
- The field max climbs steeply: 516, 517, 549, 676 and 803 MPa at 0, .01, .1, .5 and 1°.
- The lengthwise tilt κ_L·y* ∝ ΣD loads the face edge. That is physical: it is ISO's K_Hβ for
  helix mismatch.

**β 0 → 45 at Σ 0:**
- With tips relieved, L/(b/cos β_b) is continuous: 1.6211 at 0, 1.6182 at 0.1°, 1.6210 at 0.5°.
  η is smooth.
- **The field maximum jumps**: 1031 at 0, then 1338, 1337, 1335 and 1328 at 0.1, 0.5, 1 and 2°,
  then down to 387 at 45°. See §6.
- With tip contacts counted, L also jumps (1.705 → 1.615). A spur tip edge touches along the whole
  face, while the model lays a helical tip contact along e_L.

**Face through ε_β = 1** (b 6, 8, 9, 9.185, 9.5, 10, 12):
- L/(b/cos β_b) = ε_α to 1e-4 at every width.
- The field max is smooth: 1054, 861, 797, 789, 780, 764 and 714 MPa.

**Shift x at Σ 0 / 1°:** the field max is 1700 / 2284 MPa at x −0.3, 768 / 760 at 0, 570 / 781 at
.3 and 496 / 807 at .6. At x −0.3 it sits at the undercut form radius.

**Tips relieved (drop tip-anchored pairs) breaks continuity in Σ:** loaded pairs fall 2.5806 →
1.6113 between Σ 0 and 0.1°. At small Σ many lines have their minimum on the tip edge by a
hair. **Relief as a gap** (R above; C_a, L_a = 0.4 m_n) is continuous (`relief.txt`):

| Case | C_a 0 | C_a 2 µm | C_a 5 µm |
|---|---|---|---|
| spur → β 0.1/0.5/2°: field max | 1028 → 1287/1337/1328 | **724.6 → 725.5/724.9/724.4** | 759.1 → 759.0/758.6/757.2 |
| β20, Σ 0 → 0.01/0.1/1°: field max | 768 → 762/762/760 | **651.5 → 647.5/647.4/647.7** | 673.7 → 670.0/669.6/667.9 |
| β20, Σ 0 → 0.01/0.1/1°: L/(b/cos β_b) | 1.49188 → …/1.49019 | 1.43606 → 1.43634/1.43600/1.43344 | 1.27308 → 1.27310/1.27287/1.27121 |

A 2 µm relief lowers the helical field maximum by 15 % and removes the β = 0 jump; η rises by
0.15 pt (less load near the tips, where sliding is fastest).

## 4. Worm types (item 8; `wormtypes.py` → `wormtypes.txt`)

- **One surface, no branch.** ZA, ZN and ZI are one ruled helicoid,
  S(u,v) = R_z(v)(e, u cos μ, u sin μ) + p v ẑ, at three values of (e, μ):
  - ZA: e = 0, μ = α_x.
  - ZN: e = −0.1817, μ = 19.79°.
  - ZI: e = r_b, tan μ = p/e (the developable case).
- **Curvature in closed form.** With c = cos μ, s = sin μ:
  N = (cp − se, −usc, uc²); M = −c(cp−se)/|N|; N₂ = (u²sc² − e(cp−se))/|N|;
  K = −c²(cp−se)²/|N|⁴.
  - It agrees with `tools/worm_flank_curvature.py`'s finite differences to 7e-8.
  - For ZI it equals cos β_b/ρ_t to 1e-16.
  - With the involute wheel it gives the crate's 0.081615 / 0.173242, and p₀ of −4.19 % (ZN) and
    −3.24 % (ZA).
- **A matched set** (the wheel hobbed by a hob of the worm's type) is **line contact**.
  - The contact on each generator is closed form: quadratic in u, linear for ZI.
  - The wheel's curvature is a rank-one update, S₂ = S₁ − aaᵀ/(a₃₃ − a·v), with no wheel surface
    needed. Checked on parallel helicals to 1e-15 and on a finite-difference envelope to 1e-6.
  - **But the contact lines are space curves that are not linear in phase:** their second
    differences are ~1e-3 mm everywhere except a ZA worm's mid-plane. Clip limits, breakpoints and
    stationary points therefore need 1-D roots, and the load runs along a curve.
  - That is **an additional solve**, so under the ruling it is implemented and not exposed. This
    holds for ZI as well.
- **At the pitch point** κ_across is 0.142083 /mm for all three types, and falls from 0.1447 to
  0.1116 over lead angles of 5–25°.
  - Off the pitch point, ZN runs −6.9…+4.1 % in stress against ZI.
  - The old "ZN 1–15 % below ZI" holds only against the non-conjugate involute wheel, where it
    reaches −55 % at 25°.

## 5. Cost and the hot path (`t_cost.py` → `cost.txt`)

| Mesh | Pieces | States | Gap evaluations per state | Total | Python |
|---|---|---|---|---|---|
| spur 17/43 | 3 | 168 | 3 605 | 0.61 M | 13.5 s |
| helical 17/43 | 5 | 280 | 4 890 | 1.37 M | 11.2 s |
| crossed 17/43, Σ 10° | 4 | 224 | 6 247 | 1.40 M | 18.3 s |
| ring 17/−43 β15, tips counted | 25 | 1 400 | 9 070 | 12.7 M | 204 s |
| worm 1/40 (shipped) | 5–7 | 560–728 | 8 600–9 700 | 5–7 M | 32–57 s |

- **Where the time goes:** about 95 % is the anchor search: a 2-D Newton plus six bracketed edge
  scans per pair per state.
- **Rust as built:** at ~80 ns per gap evaluation (2 atan2, 1 acos, ~40 flops), 50–110 ms per
  mesh.
- **Rust with the fast routes:** closed-form anchors (the plane of action at Σ = 0; the line of
  action while the vertex is in the field; edge scans only when it leaves) and a Δ that is linear
  per line (∫dy/c precomputed). That comes to ~5 000 evaluations, **≈ 0.5 ms per mesh**.
- **Verdict:** not for the search hot path (today's closed forms take µs; the search tries
  thousands of candidates). Fit for the final solve (rating and report, once per mesh per case)
  and as the validation instrument.

## 6. Open issues (size and sign)

- **The field maximum is at the unrelieved start of active profile.**
  - It is +45–54 % over ISO σ_H0 on helicals, and 30 % higher at β 0.1° than at spur.
  - On a helical, the entering corner meets the load while the approach is still at its
    single-pair level. A spur line enters whole and shares at once.
  - The discontinuity is in the maximum over phase: a vanishing phase interval carries the peak.
  - The cure is physical, not a rule: tip relief as a gap (R above), which is ISO 21771's C_a and
    L_a.
  - **Owner's call:** either the default rating is the field maximum *with* a stated relief, or it
    is ISO's points with the maximum reported beside them.
- **Tip-edge contact as the default ("counted").**
  - Loaded pairs run 2.62–2.65 against ε_γ 2.58 at 2 N·m.
  - A tip contact is given the *flank* curvature, the spike's convention. A sharp corner is
    singular, so the model **underestimates** corner pressure.
  - "Tips relieved" as a classification is discontinuous (§3.6). Recommendation: express it as a
    relief (C_a, L_a) or as a tip-edge radius, a named and visible setting.
- **Interference is singular.** A ring 17/−43 with x₂ −0.3 and a pinion at x 0 gives p_max 1.2–1.4e4
  MPa at the pinion's base circle; the oracle measures −0.04 µm there. The rating must flag or
  refuse such a case (`flank_interference`) rather than print a number.
- **The start of the flank** is the generated form radius from `stiff.py`, which is marginal on
  17 teeth: the fillet meets the involute near r_b. The field maximum at the start of active
  profile is sensitive to it.
- **Uncoupled slices** overestimate a short patch's tooth compliance (worm 2a ≈ 1.6 mm, m 1),
  conservative for sharing; **c_H at a nominal load** (log dependence: c′ ±5 % over 100–1000 N/mm).
- **Rings:** tooth compliance as a rack tooth (no shaper fillet); a ring tip edge on the pinion
  can give κ_A ≤ 0 by flank curvature, and such a pair is skipped and counted (`Field.skipped`).
- **Breakpoints** are bracketed on 24 samples; at Σ = 0 they matched the closed-form corners.
- **The crate's worm wheel is an involute helical gear, so its contact is a point.** A hobbed
  wheel carries the load on lines, which is §4's matched set. The 3487 MPa point-contact rating
  therefore describes a wheel nobody hobs. **Owner's call:** is the involute-wheel worm a
  deliberate simplification to be stated, or a model to replace (implemented, not exposed, per
  the ruling)?

## 7. Rust migration plan (Stage 3, beside redesigns G, R, L and X)

Each step lands green, with the identity harness on everything not named.

1. **`tools/contact_oracle.py`** (from `oracle.py`) becomes the independent instrument.
   - Gates: the crossed a₀ and play (the u² law within 2 % of D₀), and ISO backlash to 1e-9 on
     spur, helical and ring pairs.
   - It replaces the planned `tools/skew_gap.py`, which was the spike's `geom.py`.
2. **`elliptic.rs`** gains:
   - `aspect(q)`, the bracketed inverse, replacing `hertz.rs::aspect_ratio` and
     `curvature_ratio`;
   - `depth_drop(a,b,h,ν)`, which is B″, with its line form and the regularised terms
     ρ₁(β) = R_D(β²,0,1)/3 − ln(4/β) + 1 and ρ₂. Both are continuous at 0 by a series below a
     stated threshold, inside the special function only.
3. **`gap.rs`** (new): d_i(Y) by the void function with signed z, the relief, the anchor search
   (closed form first, then the edges), and the clip.
   - It retires `screw.rs`'s `ZoneLimit`, `limited_by_face`, `face_widths_for`, `half_span` and
     `axial_centre`, and `contact.rs`'s path limits.
4. **`tooth_compliance.rs`** (new): from `stiff.py`, over `tooth.rs`'s generated profile (the real
   fillet), Sainsot coefficients cited; gated against ISO 6336-1 (§3.3); a ring needs redesign G.
5. **`field.rs`** (new): slices; the Δ solve (linear per line at κ_L = 0, otherwise a bracketed
   monotone root); the torque balance with friction; breakpoints, means and maxima. It replaces:
   `contact.rs`'s `load_share`, `efficiency`, `efficient_split`, `split_residual`; `screw.rs`'s
   `CrossedPath::efficiency`, `single_pair_bounds`, sampled `locking_friction`; `hertz.rs`'s
   max(σ_ell, σ_line); `strength.rs`'s single-pair points (ISO's C and B become probes).
6. **`BuiltContact`'s two arms** (shape.rs:2502) collapse into one `Field`. `MeshReport` gains the
   field maximum and where it sits, ISO C and B, ε, and the tip-contact share.
7. **Laws:** ε and the mean length equal the Σ = 0 closed forms to 1e-9; the pitch point at
   Σ 90° equals screw.rs's efficiency and locking to 1e-12; a spur's single-pair field equals
   ISO Z_B; continuity in Σ, β, x and face; module homogeneity; load conservation every phase.
8. **The search** keeps closed forms: the u² law and the derived play slope. A law holds each to
   the field within its stated bound.

**Audit tasks this subsumes:** T06.5 (share conserved by construction), T06.8 and T06.9
(efficiency is the field's torque balance, friction exact), T06.10; T07.2, T07.3 (kept only for
faces that do not overlap), T07.5, T07.14, T07.15, T07.18, T07.19; T08.11 (the truncated patch is
the Winkler clip), T08.12; redesigns U4/L and X; strength#6 (Z_ε), strength#7, added#28, added#59,
added2#15, lens-feature-gaps#4/#5.

**Overturned:** T07.4, as the spike found. **Not subsumed:** T07.9, T07.16, T07.20 (AGMA μ(v)) and
T08.3's bending bands.

## Owner's ruling (2026-09-28) on worm types, answered

Matched sets are worth adding only with no extra branch or solve. The flank type needs no branch,
but a matched set needs a solve (curved contact lines, not linear in phase), so ZA/ZN/ZI are
implemented and not exposed (§4). The ZI-only position (`docs/rationale.md#the-worm-is-a-zi-involute-helicoid`)
stands for the shipped involute-wheel model, subject to the question in §6.
