# Spike S-C: one contact geometry for parallel and crossed meshes

Author: spike-contact. Date: 2026-09-26. Branch: audit-ablation @ 8cc768b. Read-only on the repo.
Prototypes, all pure Python (numpy is not installed), are in `~/.cache/gearcalc-work/spike-contact/`:
- `geom.py`: an exact rigid-body gap between two face-bounded involute-helicoid gears on skew axes.
  It shares no code with the crate. The signed distance from any point to a flank family is
  `r_b cos β_b (F(P) − θ)` in closed form, because rotated involute helicoids are parallel surfaces.
- `screw.py`: a line-for-line port of `screw.rs`'s path: normal, branch, tangent line, zone and
  axial centres. It reproduces the shipped worm's curvatures, 0.081615 and 0.173242 /mm, to the digit.
- `patch.py`: the candidate load field.
- `run_*.py` and `t_*.py`: the tables below. Each result is saved beside its script as a `.txt` file.

## Verdict

**Adopt one model: the crossed path with its face bound carried explicitly (candidate b), and a
Hertz-calibrated slice field spread along the lengthwise direction and clipped to the field of
action (candidates a and c combined).**

- Parallel gearing is the κ_L = 0, z* = ∞ value of this model, not a branch.
- Every Σ = 0 closed form comes back exactly: the inv α_w distance law, ε_α and ε_γ, the ISO mean
  contact-line length ε_α b/cos β_b, a Z_ε-type share, and T06.8's field-weighted efficiency.
- screw.rs's curvatures, Hertz ellipse and BS 721 pitch-point thresholds also come back exactly,
  wherever the patch fits inside the field.
- Everything moves continuously in Σ.

Two decisions reverse the audit:
- **T07.4** (centre the face on the operating contact) is physically wrong near Σ = 0.
- **T07.2 and T07.3** (an empty face-bounded zone means ε 0, then refuse) would refuse pairs that
  mesh.

The `Path` trait stays as the *interface*. Its two implementations become one model with two exact
fast routes. The remaining gaps are listed with their size and sign in §5.

## 1. The boundary layer is physical, and a finite face makes it continuous (candidate b)

At Σ > 0 the crate uses the linear law, a₀ = (d₁+d₂)/2 + Σx·m_n. It is exact for unbounded
helicoids because the contact slides along a fixed line of action. At Σ = 0 the parallel law,
inv α_w, applies. The two differ by D₀ = a_lin(0) − a_par, which is second order in the shift.

The crossed contact's axial offset from the mid-planes is z* (`axial_centre`). It scales as Δ/Σ.
For a *real* pair with faces that overlap, once z* leaves the face the rigid contact sits at the
**face edge**. That contact is an edge contact on one transverse slice, which is locally a parallel
pair.

Exact rigid geometry (`t_interp.py`) gives the zero-backlash distance on the pairs below. "model"
is the closed form given after the table.

| pair, x₁/x₂, face | Σ | a_lin − a_par | exact a₀ − a_lin | model | error |
|---|---|---|---|---|---|
| 17/23 β20, +.5/+.5, 10 mm | 0 | 0.110165 | −0.110165 | −0.110165 | 0 |
| same | 0.1° | 0.102415 | −0.109425 | −0.109422 | 0.00 µm |
| same | 1° | 0.034759 | −0.102941 | −0.102839 | 0.10 µm |
| same | 5° | −0.222203 | −0.076390 | −0.076035 | 0.35 µm |
| same | 10° | −0.450473 | −0.050048 | −0.048361 | 1.69 µm |
| same, 30 mm face | 5° | | −0.029012 | −0.026704 | 2.31 µm |
| same, 30 mm face | 10° | | −0.000022 | 0 | 0.02 µm |
| 17/43 β15, +.8/+.2, 12 mm | 0.25° / 3° / 12° | | −0.084902 / −0.067722 / −0.025844 | −0.084849 / −0.067226 / −0.024180 | ≤ 1.7 µm |
| 17/23 β20, −.3/−.3, 10 mm | 0.2° / 1° / 3° | | −0.080228 / −0.073560 / −0.057114 | −0.080221 / −0.073026 / −0.056311 | ≤ 0.8 µm |

The 20/40 β30 pair at +.3/−.3 has D₀ = 0, and the exact and linear laws agree at every Σ.

**Closed form.** The exact value lies in the middle column, and the model tracks it to 2.3 µm or
better, against corrections of up to 110 µm:

```text
u   = max over members i of (1 − b_i / (2|z*_i|))₊       z*_i: the zone end nearest member i's mid-plane
a₀  = a_lin(Σ) − D₀ u²                                   D₀ = a_lin(Σ=0) − a_par(inv α_w)
j_n = [2 sin α_n + (s_par − 2 sin α_n) u] (a − a₀)       s_par = ∂j_par/∂a at Σ = 0
```

- **The two ends.** In face, u = 0 and this is the crossed law exactly. As Σ → 0, z* → ∞, u → 1
  and this is the parallel law exactly. At Σ = 0, u = 1 is a value, so no dispatch is needed.
- **The play slope** (`t_slope.py`). Measured exactly it is 0.845 at Σ = 0 and 0.822 at 5° on the
  10 mm face, 0.770 at 5° on the 30 mm face, and 0.687 at 10° on the 30 mm face (in face). The
  formula gives 0.848, 0.818, 0.763 and 0.684. The slope changes regime where the contact reaches
  the face edge. That kink is physical, and a is continuous through it.
- **Play at a = a_par + 0.05** (`t_play.py`). Exact: 42.4 µm at Σ = 0, 43.0 µm at 0.01°, 71.9 µm
  at 0.5°. Today's crossed projection gives −40.6 µm at 0.01°, floored to 0. That is an **83 µm
  jump at Σ = 0**, and the pair is reported as jammed while it has play (added#59).

**What the audit gets wrong here.**
- **T07.4** needs faces offset by z* for its proposal to hold. z* is 150 mm at 1° and 1.2 m at
  0.01° on these pairs, and no assembly does that. If it lands, the 0.110 mm and 83 µm seams at
  Σ = 0 become permanent.
- **T07.2/T07.3** read an empty face-bounded zone as no contact. The exact geometry finds a
  zero-backlash contact at every Σ tested. "No contact" should be kept for faces that do not
  overlap at all.
- **added2#15.** Its ε dip, 0.18 at 0.56°, is the same misreading.

## 2. The load field (candidates a and c, combined)

Each tooth pair j at mesh phase t has a rigid contact point X(s_j) on the crossed path,
s_j = s₀(t) + j·p_bn. Its load spreads along the lengthwise direction e_L, the minor eigenvector
of the relative-curvature matrix `hertz::relative_curvatures` already builds. At Σ = 0, e_L is the
contact line. The segment `X + y e_L` is clipped to the field in closed form: two linear face
slabs and two quadratic tip cylinders.

```text
g_j(y) = κ_L (y − y*_j)²/2 (+ the §1 face-edge offset)           rigid lengthwise gap
q_j(y) = k_j (δ − g_j(y))₊  on the clipped segment                 line load, N/mm
k_j    = 3π E* / (2 R_D(κ_r², 0, 1))                               from the pair's own Hertz ellipse
Σ_j ∫ q_j dy = F_n                                                 one approach δ shared by all pairs
p(y)   = C_j √(q(y) E* κ_across(y)/π),  C_j = p₀ / p_line          C → 1 as the ellipse elongates
```

- **Why k is not a tuning constant.** The Hertz line load along an ellipse's major axis is exactly
  the parabola q₀(1 − y²/a²). So a Winkler slice field with k = q₀/(κ_L a²/2) reproduces the
  untruncated ellipse exactly: its length, peak and pressure. k depends only on the curvature ratio,
  not on the load.
- **Truncation by the face** is closed form. With r = L/(2a) and the patch symmetric,
  q_max = (P/L)(1 + r³/2) for r ≤ 1. Today the crate uses max(1.5r, 1)·P/L. The model is C¹ at
  r = 1, and at r = 0 it is the line value.
- **Where it is off the crate's max().** The largest gap is +14.8 % in q, or **+7.1 % in σ**, at
  the crossing. strength#7's independent DC-FFT half-space solve measured +5.9 % there, and
  +3.7/+1.4/+2.4/+0.8 % at 0.75/0.5/1.5/2 times the crossing curvature. The model gives +5.4,
  +3.6, +3.3 and +1.4 %, taking a ∝ κ^(−1/3), so it is slightly conservative everywhere the max() is not.
- **strength#7** said "there is no closed form to replace the max". There is one.

**Σ = 0 reductions** (17/43, β 20°, b 10 mm, m 1, 2 N·m; `run_sweep.py`, `run_pitch.py`):
- **Loaded pairs:** 2.583, against ε_γ = 2.5806 (48-phase quadrature).
- **Mean loaded length / (b/cos β_b):** 1.4919, which equals ε_α to 4 digits. That is ISO's mean
  contact-line length.
- **Share:** F·L_j/ΣL, T06.8's rigid split, and T06.5's rule with w ≡ 1. A stiffness weight w
  enters as k_j·w.
- **Pitch pressure:** 480.9 MPa shared, against 593.5 on one line. 593.5/√ε_α = 485.9, so this is
  ISO's Z_ε for ε_β ≥ 1, instantaneous rather than empirical. It is U4's LoadField, and it closes
  strength#6.
- **Efficiency, first order in μ:** the path average along the crossed line gives 98.777 %, which
  is the parallel formula exactly. The field-weighted value is 98.781 %. At near-integer ε_β the
  two agree, as T06.8 says they should.

**The crossed limit.** On the shipped worm (`run_worm.py`) the curvatures match the crate and the
ellipse lies inside the field (2a = 1.84 mm on a 4.69 mm wheel face). Each pair is therefore
screw.rs's Hertz ellipse exactly. The pitch-point efficiency and BS 721 thresholds do not depend on
the field.

**What moves for a worm is the sharing, not the geometry.** Pairs share by a common approach
instead of the path average's 1/ε bookkeeping, which T06.8 already calls non-conserving. The
first-order loss moves from 7.668μ to 7.956μ (+3.8 %), about −1 point of efficiency at μ 0.06.
Of that, +4.9 % comes from Hertz-compliant sharing inside the zone, and −1.1 % from patches that
spill past the zone ends: 2.28 loaded pairs against ε 1.83. The spill is the same rule that
produces ε_β at Σ = 0.

**Continuity**, 17/43 β20, 10 mm face, x = 0:

| Σ | ε_path | loaded pairs | 2a/L | crate σ pitch (single, max rule) | slice single-pair | field-shared pitch σ | η path | η field |
|---|---|---|---|---|---|---|---|---|
| 0.001° | 1.6637 | 2.583 | 382 | 593.5 | 593.5 | 480.9 | 98.777 | 98.781 |
| 0.5° | 1.6629 | 2.583 | 4.97 | 594.2 | 596.2 | 485.1 | 98.773 | 98.780 |
| 1° | 1.6621 | 2.563 | 3.03 | 594.9 | 601.8 | 491.8 | 98.764 | 98.771 |
| 2° | 1.6606 | 2.542 | 1.84 | 596.3 | 622.3 | 516.5 | 98.734 | 98.742 |
| 3° | 1.6591 | 2.542 | 1.37 | 625.3 | 655.7 | 556.2 | 98.691 | 98.701 |
| 5° | 1.6564 | 2.500 | 0.94 | 756.6 | 756.6 | 668.7 | 98.581 | 98.592 |
| 10° | 1.6510 | 2.188 | 0.57 | 984.5 | 984.5 | 932.6 | 98.214 | 98.244 |
| 45° | | | 0.19 | 1627.8 | 1627.8 | 1627.8 | | |
| 90° | | | 0.19 | 1430.7 | 1430.7 | 1321.6 | | |

- **ε_path** is the crossed normal-line count. Its Σ → 0 value is **exactly ε_α/cos² β_b**:
  1.4919/0.89675 = 1.6637. That sizes reference.md's "contact ratio: not compared" row. The model
  never reports it as ε; ε is the loaded-pair count.
- **The 30 mm face exposes a latent error.** At Σ → 0 the single contact line through the pitch
  point is clipped by the tips to 15.5 mm, while the crate rates a single pair on b/cos β_b =
  31.7 mm. The slice single-pair value, 507 MPa, is therefore 48 % above the crate's 342.6.
  Whenever ε_β exceeds about ε_α, the crate's line length is not a line the teeth have. The shared
  field, 278 MPa over 5 lines, is the physical value. strength#6 covers the missing Z_ε; the
  tip-clipped single line was not found in the ledger.

## 3. Each candidate on its own

- **(a) A face-sliced field of transverse slices.** Each slice is exact at Σ = 0: slices in phase,
  with the local centre distance and the helix mismatch. It fails the worm, because gear 2's
  transverse planes are not gear 1's at large Σ. **Keep its content, not its slicing.** Slice along
  e_L (lengthwise) instead of along gear 1's axis. That is the form adopted, and at Σ = 0 the two
  coincide.
- **(b) Crossed with an explicit face bound.** Yes. The boundary layer is the face-edge contact,
  and it is continuous (§1). It still needs a load model to give ε and σ.
- **(c) A line as the limit of ellipses along instantaneous contact lines.** Yes, with one
  qualification. The ellipse must be truncated by the field, not merely compared to it by max().
  The truncation is the Hertz-calibrated slice formula (§2).

The adopted model is (b) ∘ (c) with the slice idea of (a). No general tooth-contact analysis is
needed. The rigid geometry comes from quantities the crate already has: the normal, the path,
`axial_centre`, the relative curvatures and the rulings. The only new closed-form pieces are
clipping and the δ solve.

## 4. Why this is not the rejected numerical TCA engine

| concern (plan §1) | answer |
|---|---|
| boundary layer at Σ → 0 | physical face-edge contact, closed form above, exact at both ends |
| loses closed forms | Σ = 0 route: L(t) is piecewise linear and the shares and averages are T06.8's closed forms. Worm route: the Hertz ellipse per pair, as today. Neither is replaced by quadrature |
| 1-D against 2-D | both are one construction: the path line, spread along e_L, clipped to the field |
| Coulomb at zero slip | the loaded segment crosses the pitch line at one point only, so the friction force integrated over the patch is continuous. The 1.5 % pitch seam is a pointwise artefact of evaluating F_n at the pitch point. It closes if the parallel F_n also comes from the integrated balance, which should be an option: ISO excludes friction from F_bt |

**Cost.** Per mesh: phases × pairs × (one aspect solve + one clip) + one δ solve per phase.
- The aspect solve depends only on the curvature ratio, so it can be cached per pair.
- δ is monotone and piecewise, with closed forms in both limits: linear at κ_L = 0, and
  (4/3)k√(2/κ)δ^{3/2} untruncated. A bracketed root handles the one-sided case.
- About 16 × 3 × 50 iterations, estimated at 10–100 µs in Rust. The search keeps the Σ = 0 closed
  form.
- The Python prototype sweeps 11 shaft angles in 4.5 s.

## 5. Seams that remain, with size and sign

| seam | size, sign | status |
|---|---|---|
| distance-law closed form against exact rigid geometry | ≤ 2.3 µm (≤ 2 % of D₀), model a₀ slightly high | record; a 2-variable Newton on the face-edge involute removes it if wanted |
| play slope in the edge regime | ≤ 1 % of play | record |
| lengthwise gap is not quadratic far from the vertex | local curvature 9e-5 → 1.1e-4 /mm over 25 mm at Σ 5°, x .5 | affects patch shape only in the transitional regime; bounded by the a₀ check |
| Winkler ignores finite-line edge spikes | DC-FFT: interior −3.3 % in the line regime, ends higher | same as today, which is uniform F/L; record |
| compliance is Hertz only, no tooth bending c_γ | patch too short at small Σ, which is conservative for σ | option: series c_γ gives ISO K_Hβ crowned-gear behaviour as the Σ → 0 limit |
| rating location | the field max sits at the start of active profile (857 against 594 MPa at Σ = 0, rigid, no relief) | keep ISO's pitch and single-pair points by default; the field maximum is a named option |
| worm efficiency | −≈1 pt at μ 0.06 on the shipped worm, from sharing | an intended change under L/T06.8; gate it in the corpus |

## 6. What it subsumes, and what it overturns

- **Subsumed:**
  - Face and zone tasks: T07.2 (the zone becomes the field clip), T07.5 (the continuity width is
    retired; faces come from σ(b) inversion, which is continuous in Σ), T07.15 and T08.12 (the
    0.77 → 10 mm width jump).
  - Load sharing and single-pair points: T07.19 (the field replaces single-pair points; the 5 %
    peak seam goes, as the "field-shared" column shows), T06.5 (w ≡ 1, then stiffness weights),
    T06.8 (both halves) and U4/L.
  - Pressure and seam findings: strength#6 (Z_ε), strength#7 (the truncation closed form),
    added#28 (patch length), added#59 and the distance-law and play seams, and lens-feature-gaps#5.
    Crowning is κ_L with a fixed vertex, which gives ISO crowned K_Hβ for free.
- **Overturned:** T07.4. T07.3 is kept only for faces that do not overlap. added2#15's proposed
  "ε 0 + note" becomes face-edge contact.
- **Untouched:** worm naming and proportions (T07.9 and T07.16), and AGMA μ(v) (T07.20). The
  pitch-point screw formulas stay as they are.

## 7. Migration sketch (green at every step, numbers moving only where named)

1. **Tools first.** Land `tools/skew_gap.py` (geom.py) as an independent check that shares no code.
   Gate: the crate's crossed a₀ against its exact value whenever the contact is in face.
2. **Distance and play law** on `Path`: `zero_backlash(face)` and `play(a, face)` with u from
   `axial_centre`.
   - It removes shape.rs:539-557's branch and the floor at shape.rs:2631-2643.
   - Law: continuous in Σ, equal to inv α_w at Σ = 0 to 1e-12, and within 5 µm of the tool.
   - Corpus: crossed pairs near parallel move. Parallel pairs do not.
3. **`Field` in the core.** The path line, e_L and the clip polygon. At Σ = 0, L(t) in closed form.
   `line_length_at` becomes the clipped segment. It retires `ZoneLimit` and `face_widths_for`.
4. **`hertz::slice_patch`**: the truncated Winkler-Hertz patch. It replaces `max(σ_ell, σ_line)`.
   - Law: exactly Hertz for r ≥ 1, exactly σ_line at κ_L = 0, C¹ at r = 1.
   - Test: against strength#7's DC-FFT tool within +2 %.
5. **Sharing by a common δ.** It feeds bending share, rating and efficiency weights, one field for
   three consumers.
   - Law: shares sum to 1 at every instant (T06.5), and at Σ = 0 the mean loaded length is ε_α to
     1e-9.
   - Intended moves: parallel pitch σ × about 1/√ε_α; crossed σ with sharing.
6. **Efficiency** as the friction balance integrated over loaded segments. The fast route at
   κ_L = 0 is T06.8's closed form, held equal by a law. `contact::efficiency` becomes that route.
7. **Faces from σ(b)** (T07.15 and T08.12). Bending for point contacts once the patch spans the face.

## 8. Open questions for the owner

- **Tip-edge spill** on worms: +0.45 loaded pairs on the shipped worm. The alternative is to treat
  tips as relieved, which drops the spill. That is a flag, and it cannot be a separate model,
  because the same rule is what gives ε_β at Σ = 0.
- **Friction in F_n.** Should parallel meshes include friction in F_n, which closes the 1.5 % seam,
  or keep ISO's convention, which leaves it recorded?
- **Rating points.** ISO's points by default, and the field maximum as an option?

Reproduce: `cd ~/.cache/gearcalc-work/spike-contact`, then run:
- `python3 t_interp.py 17 23 20 0.5 0.5 10,30 0,0.1,1,5,10`
- `python3 run_sweep.py 0.001,1,5`
- `python3 run_pitch.py`
- `python3 run_worm.py`
- `python3 t_play.py`
- `python3 t_slope.py`
