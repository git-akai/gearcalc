# Unified contact model: phase-exact prototype

Authors: contact-proto (2026-09-26/29), contact-proto-2 (2026-09-29, this round). Read-only on the
repo; pure Python, no numpy.
- **Scripts and outputs:** `~/.cache/gearcalc-work/contact-proto/` (README there). This round's are
  `kernel.py`, `coupled.py`, `foundation.py`, `rating.py`, `matched.py` and the `t_*.py` named below.
- **Built on:** `work/plan.md` §5 (the 2026-09-29 decisions), `work/review/contact-verify2.md`.

**Verdict of this round.**
1. **Lengthwise coupling does not cure the helical jump; it makes it slightly worse.** The jump
   comes from the line construction: a helical line entering at the tip edge must continue *along*
   that edge. With that rule the field max is continuous in β at 20 and 60 N·m, with or without
   coupling (§2.1). The coupling is kept for the reason it was wanted: it removes the contact-
   compliance formula and its κ_L = 0 switch, and it reproduces the Hertz ellipse (O(N⁻²)).
2. **The two special cases are gone** (§2.2), but only with the tip-edge radius r_e as a stated
   input. With r_e → 0 (a sharp edge) the edge pressure is singular, as elasticity says it is.
3. **ISO comparisons are corrected** (§3.3). The field max is +8.7 % over ISO at B on FZG-C-like.
   The ring's M1 is 1.345.
4. **Found: the ring's tooth compliance was about 2300× too large.** Sainsot's fit fails at a
   rack tooth, so the previous round's elastic ring rows were invalid. A derived half-plane
   foundation replaces the fit (§2.4).
5. **The matched worm set needs no new kind of solve, but it is a second wheel geometry** (§4).
6. **Cost, counted over whole analyses:** 0.56–4.6 s per mesh in Rust as built with slices, and
   2–6.6 s coupled. The fast routes at Σ = 0 are estimated at 10–30 ms with slices and 20–70 ms
   coupled (§5).

## 1. The model: every mesh, one construction

Signed tooth count (z < 0 is a ring) and the shaft angle Σ are parameters; nothing branches on the
kind of mesh.

1. **Exact rigid gap** (unchanged). Rotated involute helicoids are parallel surfaces, so
   d_i(Y) = k_i (v_i(ρ) ∓ (ψ − n p_i)), with k_i = r_bi cos β_bi. The gap is
   g = d₁ + d₂ + R₁ + R₂, where R_i = C_a(1 − u/L_a)₊² is the optional tip relief.
2. **One line per tooth pair, now a polyline.**
   - The anchor A is the minimum of g over the pair's field, as before.
   - The segment runs along e_L, the minor eigenvector of K = Σ κ_i u_i u_iᵀ, and is clipped in
     closed form.
   - **New rule, one for every anchor:** where the segment leaves the flank through a tip edge, the
     valley of the gap continues along that edge (the tip helix's tangent), away from the segment,
     clipped by the faces and the other member's bands.
   - A tip anchor's segment may have zero length; its edge then runs both ways.
   - `CField.edge_lines`; `coupled.py:pair`.
3. **Load: coupled lines, tooth in series** (`coupled.py`, `kernel.py`):
   ```text
   Δ − g(y_i) = c_t(y_i) q_i + Σ_j K_ij q_j,   q ≥ 0
   K_ij = Σ_bodies (1−ν²)/(πE) [G(η₂/b_j) − G(η₁/b_j)] − (depth-h Boussinesq term)
   G(r) = (4/π)∫₀^{π/2} cos²θ asinh(r/sin θ) dθ,   G′(r) = (4/(3π))(1+r²) R_D(0, r², 1+r²)
   b_j  = √(4q_j/(πE*κ_A))/C,   C² = 3/(s R_D(0,1,s)),   s = aspect(κ_L/κ_A)²
   ```
   - The cross-section of each load element is the Hertz half-ellipse of its own local width. This
     is the Hertz ellipse's own kernel: the untruncated ellipse solves the continuous equations
     exactly.
   - The depth reference h is the tooth model's; it is where the tooth compliance takes over.
   - G is one special function of one variable. It is odd, G(0) = 0, and
     G ~ ln 4r + ½ + 1/(16r²); its derivative is closed form.
   - **Per phase:**
     - N panels on the rigid overlap of each line (N = 24);
     - one N × N Gauss solve per loaded line per iterate, with two right-hand sides;
     - Δ closed form per iterate, because q and the torque are affine in Δ on a fixed active set;
     - an active set for q ≥ 0;
     - a fixed point on b (logarithmically weak, 3–6 iterates).
   - **Why a linear solve is acceptable:** the kernel decays like h²/d³ beyond the depth h. The
     matrix is therefore diagonally dominant, and 24 panels hold the pressure to 0.1 %.
 4. **Phase** (unchanged): breakpoints by a signature scan plus bisection, Gauss means per piece,
   maxima by golden section.
5. **Tooth compliance** is `stiff.py`'s potential-energy tooth. The foundation term is now derived
   (§2.4); `derive_foundation`.

The previous B″ slice compliance (Dyson's potential, depth-referenced) is not needed now. It
survives as the check: the kernel integrated over an infinite line equals its line form to 3e-12
(`kernel.txt`).

## 2. This round's items

### 2.1 Lengthwise coupling and the β → 0 jump (`t_beta.py` → `beta_20_final.txt`, `beta_60_final.txt`)

Setup: 17/43, m 1, b 10, μ 0.06, elastic, tips counted, Σ 0. Field max in MPa at β 0 → 0.1 / 0.5 /
2°:

| Model, 20 N·m | C_a 0 | C_a 2 µm | C_a 5 µm |
|---|---|---|---|
| slices, e_L lines (the verifier's case) | 3188 → 3945 (+23.7 %) / 3959 / 4057 | 2729 → 3538 (+29.6 %) | 2225 → 2836 (+27.5 %) |
| **coupled, e_L lines** | 3198 → 4053 (**+26.7 %**) / 4200 / 4313 | 2738 → 3323 (+21.4 %) | 2231 → 3185 (**+42.7 %**) |
| slices + edge rule | 3188 → 3201 (+0.4 %) / 3229 / 3545 | 2729 → 2773 (+1.6 %) | 2225 → 2230 (+0.3 %) |
| **coupled + edge rule** | 3198 → 3212 (+0.4 %) / 3239 / 3556 | 2738 → 2782 (+1.6 %) / 2818 / 3118 | 2231 → 2237 (+0.3 %) / 2287 / 2554 |
| coupled + edge rule, **60 N·m** | 5538 → 5535 (−0.05 %) / 5512 / 5733 | 5017 → 5015 / 5027 / 5241 | 4407 → 4408 / 4428 / 4655 |
| field.py, 60 N·m (before) | 5586 → 6926 (+24 %) | 5079 → 6543 (+29 %) | 4489 → 5590 (+25 %) |

- **Why coupling cannot cure it.** The entering corner is a vanishing segment on a tooth that
  carries nothing else, so its own load is all that deflects it.
  - Coupled, a segment of length ℓ < h loses the depth-referenced line compliance
    (2/πE*) ln(2h/ℓ). It becomes *stiffer* per unit length, not softer.
  - The approach is still the single-pair Δ₁, so q and p rise.
  - The same holds for a plate-coupled tooth: a point load on a plate deflects less than a line
    load per unit length.
  - This refutes the verifier's hypothesis in claim 3 ("a coupled model would give a continuous
    limit").
- **What does cure it.**
  - At small β the conjugate line reaches the tip edge nearly parallel to it. The wheel's tip edge
    "ahead" of the corner is within (b tan β_b)²/2ρ of the pinion flank, so at β → 0 it touches
    across the face as a spur tip edge does.
  - The field.py line through the corner ran on along e_L and missed that edge, so Δ stayed at the
    single-pair level. That is the jump.
  - Continuing the valley along the edge makes Δ fall before the corner loads. It is continuous at
    every relief and load tried, **because the geometry is now right, not because relief is sized
    for the load** (the verifier's finding 1 is resolved).
  - The residual slope, +10…+16 % by β 2°, is real: the edge's contact length shrinks as
    (b tan β_b)² grows past Δ/k.
- **Continuity in Σ held** (`sigma2.txt`; β 20, 2 N·m, Σ 0 / 0.01 / 0.1 / 1°): field max
  744.7 / 744.3 / 744.6 / 744.0; L/(b/cos β_b) 1.5693 / 1.5697 / 1.5709 / 1.5584; η 98.823 /
  98.827 / 98.823 / 98.793 %.
  - A first version that laid a tip anchor's line only along the edge broke Σ-continuity (+20 % at
    0.01°), because at small Σ many anchors land on the tip edge by a hair. The one rule above
    fixed it.
- **Ellipse reproduced** (`t_kernel.py` → `kernel.txt`; bare half-space, κ_L/κ_A 0.1–0.5): q₀
  error −9.6e-3 / −2.4e-3 / −6.0e-4 at N 16 / 32 / 64 (O(N⁻²); Richardson < 1e-5); a within
  1e-4, p₀ −3e-4 at N 64.
  - Depth-referenced, the solve tends to Hertz as a/h → 0: p₀ +0.34 % at a/h 0.35, −5e-4 at 0.035
    (the panel floor). A finite a/h *should* depart from Hertz, since the tooth is finite.
- **What the coupling changes on long lines:** ≤ 1 % in the interior. At square face ends it adds
  +2…+6 % (the end effect, bounded by the series c_t; `t_c1.py`).
- **ε at light load, rigid:** 2.58179 loaded pairs against ε_γ 2.580562. L comes out −0.57 %
  because the prototype counts loaded panels; Rust must take L from the overlap roots, as field.py
  did.

### 2.2 The two special cases (`t_ring.py` → `ring_kA.txt`)

- **The κ_L = 0 switch is gone.** No contact-compliance formula is left to switch.
  - The ellipse enters only through C(q), whose value tends continuously to 1 (C = 1.1284 / 1.0187
    / 1.0021 / 1.000023 / 1.0000000024 at q 1 / 0.1 / 1e-2 / 1e-4 / 1e-8).
  - Its removable 0·∞ at s = 0 lives inside `shape_C`: below s = 1e-24 the value is 1 to double
    precision.
- **The κ_A ≤ 0 skip is gone, given a tip-edge radius r_e.**
  - Every skip is a tip anchor: 46 on the ring 17/−43 β15 and 5 on the spur ring.
  - A point on an edge has the edge's curvature, so K gains (1/r_e) e_A e_Aᵀ there. The edge pieces
    carry the same term. With r_e 0.2 or 0.1 mm there are 0 skips.
  - κ_L = det K/κ_A is then always defined, so the ZeroDivisionError of `cost.txt` cannot recur.
- **Unloaded remainder.** Stations with κ_A ≤ 0 remain on the non-conjugate stretch of a ring
  pair's segment (5157 of them). **None was loaded** over 24 phases of the ring β15.
- **Size and sign of r_e** (ring β15, 60 N·m):
  - the field max goes 1204 (flank convention) → 3818 (r_e 0.2 mm) → 5336 MPa (0.1 mm), at the
    pinion's tip edge;
  - the loaded pairs and the ISO-point readings do not move;
  - r_e → 0 is singular.
- **So r_e must be a named, visible input**: an edge break or a tip chamfer. The field max without
  it is an unrelieved-edge figure, not a property of the gear.

### 2.3 ISO 6336-2, corrected (`rating.py`, `t_iso2.py` → `iso2.txt`)

- **σ_H at B is Z_B σ_H0**, and σ_H0 carries Z_ε. On FZG-C-like that is 1.0702 × 1522.6 =
  **1629.5 MPa**.
- **M1 and M2 come from signed radii of curvature:** ρ₂ = T₁T₂ − ρ₁ for an external pair and
  −(ρ₁ + T₁T₂) for a ring, so one formula serves both.
  - Ring 17/−43 x0/−0.3: **M1 1.3447** (the verifier's 1.3447, not 2.315), M2 1.1739. ISO's σ_H at
    B is 886.6, not 1794.8.
  - 20/60 β15 (ε_β 0.824 < 1): Z_B = M1 − ε_β(M1 − 1) = 1.0157, so 851.0.
- **"FZG-C" is relabelled FZG-C-like** (tips untrimmed, as the verifier noted).

### 2.4 Tooth stiffness: spread, the fitted term, and a derived foundation (`foundation.py`, `t_found.py` → `found.txt`)

**Spread at 17/43 x0** (q 300 N/mm, E 206 GPa; N/(mm·µm)):

| Source | c′ | c_γ | Against ISO Method B c′ |
|---|---|---|---|
| idealised PE tooth (verifier; no generated fillet) | 11.6 | – | −7 % |
| ISO 6336-1 Method B, c′ = c′_th C_M C_R C_B (C_M 0.8) | 12.50 | 18.33 | 0 |
| model, Sainsot foundation (fitted) | 13.65 | 19.79 | +9 % |
| **model, derived half-plane foundation** | **15.60** | **22.48** | **+25 %** |
| ISO c′_th (theory, before C_M) | 16.03 | 23.50 | +28 % |

- **The spread is 11.6–16.0, about ±16 % around 13.8.**
  - Its sign is set by the foundation, which is 44 % of the compliance, and by the fillet
    geometry.
  - Over the 5 × 3 grid the derived model sits −3 … −13 % under c′_th.
  - ISO's C_M = 0.8 is ISO's *measured* correction from theory to real gears, so the derived model
    agrees with ISO's theory and is 25 % stiffer than ISO's rating value.
- **Effect on rating.** Stiffness enters σ_H only through sharing. The 17/43 β20 pitch reading
  moved −0.5 % → −0.1 %, and the field max 1430 → 1422, between the two foundations.
- **Sainsot, Velex & Duverger 2004's L*, M*, P*, Q* are fitted** polynomials in (h_f, θ_f),
  fitted to Muskhelishvili's annulus solution.
  - They fail outside the fitted range: at z = 400, P* = −0.21; at z = 4000 (the rack tooth used
    for rings), L* = −86 and M* = 1032.
  - That makes the ring's foundation 44.65 µm/(N/mm) against about 0.019: **2300× too compliant**.
  - The previous round's elastic ring rows (±% at the pitch, 2.54 pairs) are **withdrawn**. The
    corrected rings, with the derived foundation, give 2.03 and 2.57 loaded pairs (§3.3).
- **Derived alternative (prototyped).** The root section is a segment S_f on an elastic
  half-plane under the beam's section tractions (Weber 1949; O'Donnell 1960), with work-conjugate
  section displacements.
  - **L* = 18(1−ν²)/π = 5.214** exactly, since ∫∫xy ln|x−y| over the unit square is −1/16.
  - **M* = 2(1−2ν)(1+ν) = 1.040.** Both are closed form and need no reference.
  - P* and P*Q* are the mean shear and normal translations. A half-plane's are log-divergent, so
    they are referenced at the depth H below the section: the bore, H = r_f(1 − 1/h_f), or a
    ring's rim.
  - They come from Boussinesq and Cerruti integrated along the face. The formulas are in
    `foundation.py`'s docstring and are checked at s → 0 against the axis limits; the section
    averages are closed form except one 1-D quadrature.
  - It is valid for any z, the rack included. It is monotone in H: c′ 16.37 / 15.60 / 14.87 /
    14.38 at h_f 1.2 / 1.4 / 2 / 4, while Sainsot's fit gives 9.47 at h_f 4.
  - Muskhelishvili's annulus itself is a series, not a closed form: it is what Sainsot fitted.
  - The ring's rim depth is a stated input (3.5 m_n here; the crate has
    `MemberGear::rim_thickness`).

## 3. Validations

### 3.1 Oracle, ε and breakpoints (unchanged; confirmed by the verifier)

- ISO 21771 j_n, oracle against formula: 34.5219 / 38.6366 / 33.7489 / 37.7174 µm (spur,
  helical, two rings). a₀ table 10/10 rows; the u² play law is +6.40 µm off at x 1/1, Σ 6°.
- Interference: ring 17/−43 x0 needs |r_a2| ≥ 20.687 and has 20.5. The corner goes 24.5 µm into
  the generated fillet below r_b, which the oracle cannot see, so `flank_interference` must be
  checked against the generated root.
- Breakpoints at 0.41581 / 0.50451 / 0.90770 / 0.99641 of a pitch (the closed-form corners);
  ε_γ 2.58060 against 2.580562; mean length / (b/cos β_b) = ε_α 1.4918785.

### 3.3 The rating read at ISO's points, with the field max beside it (`iso2.txt`)

Coupled field, edge rule, tips counted, no relief, μ 0, derived foundation. The default reading is
the pinion's max(C, B) and the wheel's max(C, D), read off the field in the mid-plane.

| Pair | ISO σ_H0 · σ_H,B · σ_H,D | Field at C · B · D | Pinion · wheel reading against ISO | Field max against ISO's largest |
|---|---|---|---|---|
| FZG-C-like 16/24, 302 N·m | 1522.6 · 1629.5 · 1522.6 | 1648.5 · 1336.5 · 1236.7 | **+1.2 % · +8.3 %** | 1770.9, **+8.7 %** |
| 17/43 β20 m2, 60 N·m | 961.3 · 961.3 · 961.3 | 960.7 · 1036.8 · 905.1 | **+7.9 % · −0.1 %** | 1421.6, +47.9 % (SAP) |
| 20/60 β15 m4, 500 N·m | 837.8 · 851.0 · 837.8 | 847.9 · 903.9 · 810.2 | **+6.2 % · +1.2 %** | 1158.2, +36.1 % (SAP) |
| ring 17/−43 β15 x.3/−.3, 60 N·m | 639.1 · 647.2 · 639.1 | 655.5 · 683.1 · 575.3 | **+5.5 % · +2.6 %** | 1212.8, +87 % (face end, SAP) |
| ring 17/−43 x0/−.3 spur, 60 N·m | 659.3 · 886.6 · 773.9 | 591.8 · 785.4 · 719.7 | −11.4 % · −7.0 % | 12 347 (SAP at r_b: singular) |

- **At C the field agrees with ISO** within −0.1 … +2.6 % on helicals and rings. On the spur, C
  lies in the single-pair zone, and the field's +8.3 % is ISO's Z_ε, which ISO applies there and
  the field does not.
- **At B the field is lower on spurs.**
  - Tip contact under load (302 N·m, elastic) carries the two-pair zone past B, which ISO's rigid
    Z_B ignores.
  - The field max sits where the single-pair zone now starts (r₁ 35.75 against r_B 35.40).
- **The helical and ring maxima sit at the start of active profile (SAP)** or a face end, where ISO
  does not look. They are unrelieved-edge figures (§2.2).
- **The spur ring's 12 347 MPa** is the pinion's SAP at 15.975, next to r_b 15.97: κ ∝ 1/ρ_t → ∞.
  The rating must flag it; the default reading is unaffected.

### 3.4 The shipped worm, involute wheel (unchanged; closed forms confirmed by the verifier)

Curvatures 0.081615 / 0.173242; Hertz at 54.953 N·m, μ 0.06: 1.6328 × 0.9898 mm, 3487.4 MPa; the
golden efficiency table and locking μ 6.5104 / 0.1356. Full field (elastic, Sainsot): 3.31 pairs,
η 66.49 %, max 2172 MPa.

## 4. Worm types and the matched set (`wormtypes.py`; `matched.py`, `t_matched.py` → `matched.txt`)

- **The flank type is a parameter:** ZA, ZN and ZI are one ruled helicoid at three (e, μ), with
  closed-form curvatures. This holds for ruled types only; ZK and ZC are outside the family (the
  verifier).
- **Matched set, prototyped for ZN, ZA and ZI** (wheel hobbed by a hob of the worm's own type, same
  a and Σ).
  - **Lines:** the meshing equation N·v₁₂ = 0 is a quadratic in u per generator, closed form, and
    linear in phase (the verifier is right). The contact set at phase φ is the curve
    v ↦ X(u(v), v). It needs no anchor search and no wheel surface.
  - **Clip:** where the curve is inside both bodies:
    - worm tip and root (closed form in u);
    - worm length and wheel face (planes fixed in space);
    - wheel tip (a surface of revolution fixed in space);
    - the discriminant.
    Each end is a **bracketed 1-D root in v**, the same kind as the field's loaded-interval ends.
  - **Load:** conjugate, so g ≡ 0 on each line and Δ is closed form (no root). The load is coupled
    along the arc length. κ_across comes from the rank-one update at every station (0.142083 at the
    pitch point for all three types, as before).
  - **Phase:** the same breakpoint scan and bisection, Gauss means and golden maxima.
- **Results** (steel on C360, μ 0.06, worm torque 2 N·m for η, wheel torque 54.953 N·m for the
  maximum):

| Wheel | Lines (mean) | Line length (mean) | η | Field max | Where |
|---|---|---|---|---|---|
| **ZN matched**, elastic, coupled | 2.045 | 8.14 mm | 67.29 % | 1842 MPa | worm tip, r 4.50 |
| ZA matched | 2.031 | 8.14 mm | 67.30 % | 1875 | worm tip |
| ZI matched | 1.988 | 8.12 mm | 67.34 % | 1766 | worm tip |
| ZN, slices (uncoupled) | 2.045 | – | 67.30 % | 1744 | worm tip |
| ZN, rigid teeth, coupled | 2.045 | – | 67.60 % | 43 721 | line end at the wheel tip |
| involute wheel (crate), elastic field | 3.31 pairs, points | – | 66.49 % | 2172 | worm tip |
| crate rating, pitch-point Hertz | 1 | – | 68.69 % | 3487 | – |

- **Reading the table.**
  - The matched set carries the load on about 2 lines of 8 mm where the involute wheel has 3.3
    point contacts. Its peak is 15 % lower than the involute field and 47 % lower than the crate's
    rating.
  - The rigid coupled line has square ends, so it is singular at the wheel tip edge, as §2.1
    predicts. Only elastic figures are meaningful.
- **Additional branch or solve?**
  - **No new kind of solve:** a closed-form quadratic, bracketed 1-D roots, a linear Δ.
  - **But it is a second wheel geometry.** The field's gap d₂ is closed form only for a wheel whose
    flank is an involute helicoid (a rack- or involute-hob-cut wheel). A worm-hobbed wheel exists
    only as the worm's envelope.
  - The line then comes from the meshing equation instead of the gap's minimum. It is exact for a
    conjugate pair, but it has no gap off the line, so there is no play, no misalignment and no
    oversize hob.
  - As a parameter, this is "the wheel's generating tool: a rack, or the mating worm". It is the
    same kind of parameter as `ShaperCut`'s z → ∞ (CLAUDE.md rule 4).
  - It becomes branch-free only if the worm wheel is *always* the worm's envelope, replacing the
    involute wheel rather than sitting beside it.
  - **Owner's call:** replace the involute worm wheel (no extra branch, and play would need the
    envelope's gap), or keep it and do not expose the matched set.
- **Cost of the matched set:** 5.98–7.2 M meshing-equation evaluations per full analysis (both
  loads), with each curve scanned over the whole worm length. Bracketing each clip from the
  previous phase cuts this about 20×.

## 5. Cost, counted (`t_cost2.py` → `cost2.txt`)

Whole analyses (scan, bisection, Gauss, golden, nominal), nothing extrapolated. Rust = counts ×
unit costs in ns (hypot + atan2 35, acos + tan 45, cos + sin 25, G 15, depth panel 200, LU 1 per
flop), about ×2 uncertain.

| Mesh | Model | States | Anchors | d_i evals | Transcendental calls (local / v / glob) | G / depth panels | LU (Σn³) | Python | **Rust estimate** |
|---|---|---|---|---|---|---|---|---|---|
| spur 17/43 | field.py | 180 | 2 376 | 0.97 M | 9.0 / 5.3 / 4.3 M | – | – | 41 s | **0.66 s** |
| spur 17/43 | coupled + edge | 180 | 2 376 | 1.07 M | 9.4 / 5.4 / 4.4 M | 16.3 / 5.4 M | 3 713 (5.1e7) | 73 s | **2.0 s** |
| helical 17/43 β20 | field.py | 301 | 3 987 | 2.05 M | 7.8 / 5.5 / 3.5 M | – | – | 37 s | 0.61 s |
| helical 17/43 β20 | coupled + edge | 296 | 3 942 | 3.12 M | 9.5 / 6.5 / 3.4 M | 36.3 / 12.1 M | 10 531 | 117 s | 3.7 s |
| crossed Σ10 | field.py / coupled | 240 / 240 | 3 177 | 2.4 / 2.5 M | 7.5 / 5.1 / 2.7 M | – / 29.9 / 10.0 M | – / 10 181 | 45 / 100 s | 0.56 / 3.0 s |
| ring β15 | field.py (25 pieces) | 1 500 | 19 998 | 19.0 M | 60.7 / 41.5 / 22.5 M | – | – | 179 s | **4.6 s** |
| ring β15 | coupled + edge (6 pieces) | 360 | 4 779 | 4.7 M | 14.4 / 10.1 / 5.4 M | 66.3 / 22.1 M | 14 844 | 163 s | 6.6 s |
| worm, involute wheel | field.py / coupled | 358 / 419 | 4 761 / 5 571 | 4.4 / 4.4 M | 14.2 / 9.3 / 4.8 M | – / 66.3 / 22.1 M | – / 23 906 | 65 / 157 s | 1.0 / 6.6 s |

- **The previous round undercounted.** Gap evaluations are 1.6× its extrapolation on the spur (the
  verifier: 1.5–2.6×). They are also only a tenth of the transcendental work: the feasibility
  margins and flank points in the anchor search dominate. The old "50–110 ms per mesh" is wrong by
  6–40×.
- **The ring's 25 pieces were spurious signature changes** from 2300×-compliant teeth. With a sound
  tooth it has 6.
- **Where coupled time goes:** depth panels 54–67 %, the anchor search's transcendental calls 16–34 %. The LU is < 2 %.
- **Derived fast routes.**
  - **Σ = 0:**
    - the anchors, clips and breakpoints are closed form (plane of action; the verifier), which
      removes about 95 % of the transcendental calls;
    - the depth kernel is independent of b to O(b²/h²), so it is built once per discretisation,
      not per b-iterate (÷4–6);
    - maxima by Brent's method with derivatives, not 25-step golden sections (states ÷3).
  - **Estimate per mesh:** about 100 states × 2.5 lines × (24 stations × 0.4 µs + kernel about
    60 µs + LU 8 × 9 µs) ≈ **35 ms coupled** (×2 band: 20–70 ms), **10–30 ms with slices**.
  - **Σ ≠ 0 and the worm:** the interior anchor lies on the closed-form crossed line of action, and
    edge scans are needed only when it leaves the field. Allow 2–5× the Σ = 0 figure.
- **Verdict (unchanged in kind):** not for the search hot path (closed forms take µs); fit for the
  final solve (once per mesh per load case) and validation. Slices carry the interior within 1 %,
  so coupling can be confined to line ends and short lines if 35 ms is too much.

## 6. Open issues (size and sign)

- **The field max is an unrelieved-edge figure.**
  - Helicals are +36…+48 % over ISO at the SAP, rings +87 % at a face end and SAP, and a spur ring
    is singular near r_b.
  - It is now continuous in β and Σ. It is finite only through the flank-curvature convention or a
    stated r_e, and it rises as r_e falls (§2.2).
  - The default rating reads ISO's points (§3.3), as decided.
- **Line direction at an edge is a rule, not a solve.**
  - The valley continues along a tip edge. Face edges are not continued: there the gap grows at the
    κ_A rate across e_L, so their contact is within a Hertz width of the corner.
  - Edge pieces are straight tangents to the helix, which is exact as β → 0. At large β the edge
    load is short.
  - Coupling across the bend treats the polyline as straightened, which is exact as the bend angle
    (about β_b) → 0.
- **Tooth coupling along the face** (plate action) is not modelled (series per station); by §2.1's
  argument it would raise short-segment loads. Size not measured.
- **A ring's tooth** is the rack tooth on a half-plane rim (3.5 m_n, stated); shaper-cut fillets
  need redesign G. **Interference** must be flagged against the generated root (§3.1).

## 7. Rust migration plan (Stage 3, beside redesigns G, R, L and X)

Each step lands green, with the identity harness on everything not named.

1. **`tools/contact_oracle.py`** (from `oracle.py`) is the independent instrument: a₀ and play, ISO
   backlash to 1e-9, interference against the generated root.
2. **`elliptic.rs`** gains `aspect(q)`, `hertz_shape(q)` = C (C(0) = 1 inside the function) and
   `strip(r)` = G with its R_D derivative (Chebyshev in ln r plus the two asymptotes).
3. **`gap.rs`** (new): the void function with signed z, relief, the anchor (closed form at Σ = 0,
   the crossed line of action, then edges), the clip, and the edge continuation. It retires
   `screw.rs`'s `ZoneLimit` family and `contact.rs`'s path limits.
4. **`tooth_compliance.rs`** (new): `stiff.py` over `tooth.rs`'s profile with the **derived
   foundation**, not Sainsot's fit; gated against c′_th (−3 … −13 %) and Method B (C_M an option).
5. **`field.rs`** (new): coupled lines (N panels; depth kernel once per discretisation), the
   active set, Δ closed form, the torque balance with friction, breakpoints, means and maxima. It
   replaces the functions listed in the previous round (`contact.rs` sharing and efficiency,
   `screw.rs` path efficiency and locking, `hertz.rs`'s max of two, `strength.rs`'s single-pair
   points).
6. **The rating:** `MemberRating` reads the field at ISO's C, B and D by default. `MeshReport` gains
   the field max, where it sits and its kind (interior, face, edge), ε, the tip-contact share and
   r_e.
7. **Laws:** ε and mean length at Σ = 0 equal the closed forms; the ellipse converges at
   O(N⁻²); the infinite line equals the 2-D depth formula; the Σ 90° pitch point equals
   `screw.rs`; continuity in Σ, β (2, 20, 60 N·m), x and face; module homogeneity; load
   conservation; no loaded station with κ_A ≤ 0 given r_e.
8. **The search** keeps the closed forms (the u² law, the derived play slope), held by a law to the
   field.
9. **Matched worm set:** only if the owner replaces the involute wheel (§4). Then `gap.rs` takes
   the wheel's tool as a parameter.

**Subsumed audit tasks (unchanged):** T06.5, T06.8–10, T07.2–3, T07.5, T07.14–15, T07.18–19,
T08.11–12, U4/L, X, strength#6–7, added#28, added#59, added2#15, lens-feature-gaps#4/#5.

**Overturned:** T07.4. **Not subsumed:** T07.9, T07.16, T07.20, T08.3's bands.

**Owner's decisions of 2026-09-29, status:** ratings at ISO's points, field max beside
(implemented, §3.3); coupling (done, not the cure, §2.1); special cases (removed given r_e, §2.2);
Z_ε at B and ring M1 (§2.3); stiffness spread (±16 %, §2.4); matched worm set (owner's call, §4).
