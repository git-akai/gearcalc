# Unified contact model: phase-exact prototype (the expensive mode)

Authors: contact-proto (2026-09-26/29), contact-proto-2 (round two), contact-proto-3 (round three),
contact-proto-4 (2026-09-29, round four, this revision). Read-only on the repo; pure Python, no numpy.
- **Scripts and outputs:** `~/.cache/gearcalc-work/contact-proto/` (README lists them). Round three's text
  is kept there as `contact-model.round3.md`, and round two's as `contact-model.round2.md`.
- **Built on:** `work/plan.md` §5 (2026-09-30: the field model is the on-demand expensive mode) and
  `review/contact-verify4.md` (with `contact-verify3.md`).

**Status: complete for round four.** Every table was run under the final code (`trace.py`); every
process was capped at 1.8 GB.

**Verdict of this round.**
1. **Each pair's line is now a continuous function of every input** (§1.1, §2.1). The verifier's
   case (β20, r_e 0.1, 2 N·m, a change of 2.8e-10 mm) gives the same state to 8 digits, and D moves by
   exactly the rigid 0.34 × Δa. Every input (a, x, b, one face, β, Σ) at δ = 1e-9 … 1e-3 moves the
   24-phase maxima in proportion to δ, down to a 3e-9 floor, on a helical and a spur base.
2. **The curvature step at a round's tangency is treated, not smoothed away** (§1.3, §2.2): the
   across contact is solved in 2-D in closed form for the station's piecewise-quadratic section. C¹ is
   the exact regularity a tip form needs; round three's pointwise Hertz was high by 20 % at the spur
   edge peak.
3. **Face ends are free surfaces** (images, §1.4, §2.3). Where both gears end at one plane the maximum
   is converged to 5 digits at 24/48/96 panels (round three's 6585 → 6624 → 6679 diverged). Where only
   one ends (unequal faces) the edge is real; with graded panels it converges (increments halve) to
   +3 % over the aligned figure, and a stated end relief converges at 24 panels.
4. **Tooth stiffness: the prototype's derived tooth is the right one** (§2.4). An independent plane-
   strain FE of the whole gear puts the derived tooth 7–13 % too compliant and the verifier's
   PE + Sainsot tooth 1.46–1.72× too compliant; c′ is 17.06 (FE), 15.60 (derived), 13.65 (Sainsot),
   11.61 (verifier), against ISO's c′_th 16.03.
5. **The sharp edge (r_e unset) is refused in the expensive mode** (§2.5). Its β → 0 limit is
   continuous, but it is steep (+14 % by 0.1°) and it is a convention, not the r_e → 0 limit. Only C¹
   forms are rated.
6. **Σ, x and face-width sweeps are smooth** (§2.5). Σ 0 → 0.01° at a_par is +0.08 % (2019.7 → 2021.4).
7. **The matched worm wheel as a surface needs no new solve** (§2.6). A closest-point projection gives
   exact play (j = s_h/(cos α_n cos γ)) and the edge rule; the edge peaks are 31–33 % below the
   involute wheel's.
8. **Cost, measured:** 2–16 min of Python per mesh per load. The Rust estimate is 5–24 s, or 2–8 s with
   round three's b-independent kernel (§2.7).

## 1. What changed in the model (`trace.py`)

### 1.1 The line is traced on fixed sections, not seeded
- Round three laid a straight base line through the anchor along the anchor's own e_L and took the
  gap's minimum across it. The anchor search picks a basin (interior, tip edge, face edge), so a
  2.8e-10 mm change of a picked another anchor, another e_L and another base line; and the base line
  was clipped by the root circle where the valley was not. That was the verifier's +18 %.
- Now each pair's line is the **valley of the gap on a fixed family of sections**: planes X·d = ζ, with
  d one direction per mesh (the minor curvature direction at the pitch point: the conjugate line's
  direction at Σ 0), on a fixed grid ζ = k·Δζ (Δζ = b/40).
- **A section's valley point is intrinsic**: the point of member 1's flank in the plane where g is
  stationary along the section (e_S = d × n). It is a fixed point: a 1-D minimisation along the tangent
  line from the current point, repeated until the step is < 1e-10 mm (≤ 8 rounds); the base point it
  starts from does not change it. (A version that minimised along a line through an extrapolated base
  point left 1e-5 mm of history in every node.)
- **The trace** marches the grid both ways from the section nearest the seed, until the valley leaves
  the field by EXT = 0.2 mm or no minimum exists across. The anchor is only a seed.
- **The line** is the part of the valley inside the field and within CAP = 30 µm of its lowest gap; its
  ends are roots of the field margin evaluated on valley points, so faces and root/form circles clip
  the valley itself. Stations re-project on their own sections; the across direction of a station is
  n × τ with τ the valley's interpolated tangent.
- Termination: every loop has a cap (sections ≤ 6000 per branch; 10 bracket expansions; 8 fixed-point
  rounds), and the field bounds ζ.

### 1.2 The panel span is a continuous function
- Round three grew the span by ×1.6 when the loaded part touched it: a branch. The loaded part lies
  inside {g < D}, because every term of D − g_i = c_t q_i + Σ_j K_ij q_j is ≥ 0 (counted: no negative
  coupling at a loaded station in any run). So span = {g < dlo + 1.5 (D₀ − dlo)} from the slices seed D₀
  in one shot, and D ≥ Dmax is refused loudly (0 times in every run).
- The seed's line compliance now uses each point's own curvature. Round three's used the pair's kA at
  its anchor, which moves along a flat valley and made D₀, and so the panels, jump.
- Round three's "touch" was a false alarm: the loaded end inside the end panel.

### 1.3 The across contact is 2-D and exact for the section (`contact2d`)
- A station's section curvature is h″(t) = k₀ + Σ forms' graph curvature × cos², piecewise constant in
  the across coordinate t: steps at a round's tangency and end, and at the relief's end.
- The frictionless half-plane contact with that profile is solved in closed form. The contact is
  bounded at both ends, and with t = m + c cos φ:
  - centre m and half-width c from I₀ = ∫h′ dφ = 0 and q = (E*/2) c ∫h′ cos φ dφ;
  - pressure p(θ) = (E*/2)·conj[h′](θ), a sum of logarithms (h′ is piecewise linear in cos φ).
- With no step inside the contact the solution is Hertz, and the code uses Hertz there. The two agree
  at the switch, so there is no branch in value.
- **Checked** (`t_c2d.py` → `c2d.txt`) against an independent discretised half-plane contact (log kernel,
  240 panels, active set): within 0.12 % at every step position, a finite round and relief + round.
  - Hertz exact when the step is outside.
  - At the step, 4855 against Hertz-on-the-round 6597 MPa (q 100 N/mm, κ 2.08 + 10).
- The station's strip half-width for the lengthwise kernel is this c, so b is continuous too.

### 1.4 Face ends are free surfaces (images)
- Round three's kernel treated each body as a half-space continuing past the face: the square end was
  an edge of a flat punch, singular, and the maximum grew with the panel count.
- Each body's free end face is now the mirror image of the line's load about that body's face
  crossing (Hetényi's first step: no shear on the face plane; its normal stress is left, so the end
  point is high by at most the plane-strain/plane-stress factor √(1−ν²), 4.6 %, sign known).
- Both gears ending at one plane (Σ 0, equal faces): the line behaves as an infinite line at its ends.
- One ending (unequal faces, Σ ≠ 0): the other's half-space meets a square-ended body, a real edge,
  bounded by the tooth compliance in series. Its value depends on the end form, so an **end relief**
  (C_e (1 − d/L_e)² within L_e of a face, the form rule along the lead) is a member input.

## 2. This round's items

### 2.1 Continuity (`t_det4.py`, `t_cont4.py` → `cont4_h20.txt`, `cont4_s0.txt`)

**The verifier's case** (17/43 β20 r_e 0.1, 2 N·m, Σ 0; a = 31.925333 and 31.925333000277):

| Phase | Round three | This round |
|---|---|---|
| 0.8875 p | 2075 → 2446 MPa (+18 %) | 1962.0757 / 1962.0757 |
| 0.7425 p | 2324 → 2057 (−11 %) | 1869.9351 / 1869.9351 |

D: 1.232456029 / 1.232456124 µm, the rigid 0.34 × 2.8e-10 mm. The lines are 4.565 / 10.5606 / 1.1962 mm
in both.

**Every input** (24 phases; the largest relative change over the phases of the state's field max):

| Input | δ 1e-9 | 1e-7 | 1e-5 | 1e-3 |
|---|---|---|---|---|
| helical β20 r_e 0.1 2 N·m: a (mm) | 7.3e-9 | 7.3e-7 | 7.3e-5 | 7.2e-3 |
| x (both) | 8.4e-9 | 5.9e-7 | 5.9e-5 | 5.9e-3 |
| b (both, mm) | 4.2e-9 | 4.7e-8 | 4.8e-6 | 4.7e-4 |
| b₂ only (mm) | 3.5e-9 | 3.4e-9 | 5.8e-8 | 5.8e-6 |
| β (deg) | 2.7e-9 | 5.2e-9 | 5.2e-7 | 5.2e-5 |
| Σ (deg) | 4.3e-9 | 1.1e-7 | 1.1e-5 | 1.1e-3 |
| spur r_e 0.1 20 N·m: a | 1.8e-8 | 1.7e-6 | 1.7e-4 | 1.3e-2 |
| x | 1.5e-8 | 1.5e-6 | 1.5e-4 | 1.1e-2 |
| b | 1.4e-9 | 1.4e-8 | 1.4e-6 | 1.4e-4 |
| b₂ only | 1.4e-9 | 5.0e-9 | 7.4e-8 | 2.4e-5 |
| β from 0 | 1.9e-9 | 4.3e-9 | 7.7e-6 | 9.9e-4 |
| Σ from 0 | 2.5e-9 | 2.8e-9 | 1.0e-6 | 2.8e-4 |

- Every column is proportional to δ down to a floor of ~3e-9 (the b fixed point's tolerance).
- No run fell back, failed to converge, or lost a valley.


### 2.2 The curvature step at a round's tangency (`t_c2d.py`, `x_v1c2d.py`, `t_end4.py`)

**Which: a smooth form, or the step treated? The step treated; C¹ is the exact requirement.**
- In 2-D frictionless contact the pressure is p = (E*/2)·conj[h′] on the contact interval (§1.3), with
  h′ the section's slope.
- A C¹ form (h′ continuous, h″ stepping at a tangency) has a conjugate that is continuous: p is finite and
  continuous, and only its slope has a logarithm at the step.
- A C⁰ corner (h′ jumps: a chamfer's unblended corner, the sharp edge, a relief ending with a slope) has
  a conjugate with a logarithmic singularity: p → ∞ at the corner, whatever the load.
- So **C¹ is the exact threshold**. Every tip form in the expensive mode must be C¹ (a chamfer's corners
  given radii), and no C² blend is needed, because the solver takes the step exactly.
- A pointwise rule (Hertz with the curvature at the contact centre) is exact only when the step lies
  outside the contact width, and wrong inside ±c of it. That is where round three's peak sat: the
  round-side limit at the tangency.

**Size of the artefact.**
- Same station, spur 17/43 r_e 0.1, 20 N·m (`end4_*.txt`): pointwise 6563, exact 5255 MPa at the
  phase round three reported (−20 %).
- Over phase the true peak moves inside the round, where q has fallen. The analysed maximum is 5957,
  against 6585 in round three: **−9.5 %**.
- **Independent of our load model** (`x_v1c2d.txt`): the verifier's own 2-D spur model (exact circle
  round, its load sharing and its tooth), with contact2d replacing its pointwise Hertz, gives:

  | Load | r_e 0.2 | r_e 0.1 |
  |---|---|---|
  | 20 N·m | 5001 → 4672 (−6.6 %) | 6631 → 6072 (−8.4 %) |
  | 60 N·m | 8638 → 7902 (−8.5 %) | 11446 → 10263 (−10.3 %) |

  - Ours at r_e 0.1, 20 N·m is −9.2 % against our own pointwise, and 5957 against its 6072 (−1.9 %).
  - The verifier's "−4 to −5 %" (one half-width rolled onto the round) is too small. The exact
    solution needs the contact about c inside the round before p reaches round-Hertz, and q has fallen
    by then.

### 2.3 Face ends (`t_end4.py`, `t_end4b.py`, `t_end4c.py`)

**Why round three diverged.** Its kernel treated each body as a half-space running on past the face.
The load then stops abruptly on a continuing surface: a flat punch's edge, singular in elasticity. The
maximum grew with the panel count (the verifier's 6585 → 6624 → 6679).

**Aligned ends are not edges.** Where both teeth end at one plane, the free faces carry no traction.
With images they converge (spur 17/43 r_e 0.1, 20 N·m, the same phases, field max):

| Kernel | N 24 | 48 | 96 |
|---|---|---|---|
| half-space (round three), pointwise | 6585.1 | 6624.0 | 6678.4 |
| half-space, 2-D across | 5272.3 | 5301.8 | 5342.9 |
| images, pointwise | 6562.7 | 6562.7 | 6562.7 |
| images, 2-D across | 5255.2 | 5255.2 | 5255.2 |

- With images the maximum sits mid-face, not at the end. The line is uniform along the face, as a spur
  line should be.

**Ends that are not aligned are real edges**, whenever one body ends and the other continues. Spur,
r_e unset, 20 phases, the pinion b 10, the wheel b₂:

| Case | N 24 | 48 | 96 |
|---|---|---|---|
| b₂ 10.001, uniform panels | 3057.4 | 3057.5 | 3057.7 |
| b₂ 10.5, uniform | 3059.2 | 3066.3 | 3077.7 |
| b₂ 10.5, cosine-graded | 3109.1 | 3130.5 | 3140.5 |
| b₂ 10 (aligned), graded | 3057.3 | 3057.3 | 3057.3 |
| b₂ 10.5, end relief C_e 5 µm, L_e 1 mm | 3128.0 | 3130.0 | – |

- The edge is bounded: the tooth compliance in series makes it a second-kind equation. It is resolved
  only once the end panels are shorter than the Hertz width b. Graded increments halve (21, 10), so
  the limit is ≈ 3150 (Richardson), +3 % over the aligned end. Uniform panels have not converged at 96.
- The figure there is a property of the corner's form. A stated end relief (the form rule along the
  lead) moves the peak inboard and converges at 24 panels.
- **For Rust:** images always; cosine-graded panels where a line ends on only one body's face; end relief
  as a member input with no hidden default. Tooth plate coupling along the face is still not modelled.
  Its sign is known: it lowers any end or edge peak.

### 2.4 Tooth stiffness (`fe/fe_tooth.py` → `fe/fe_tooth.txt`, contact-proto-4-fe)

**The reference.**
- An independent plane-strain FE of the generated tooth: 9-node quads, the whole gear as a periodic
  ring, bore r_f/1.4 clamped.
- Checks: three meshes within 0.06 %; point load against a Hertz ellipse ≤ 0.08 %; a Timoshenko
  cantilever within 0.5 %.
- It measures what `stiff.py` calls the tooth: the displacement along the load line of its centreline
  crossing.

| µm per N/mm | FE | derived (prototype) | Sainsot | verifier (PE + Sainsot) |
|---|---|---|---|---|
| z17 at 8.0023 / pitch 8.5 | 0.01626 / 0.02226 | +7 % / +13 % | +36 % / +36 % | +67 % / +72 % |
| z43 at pitch 21.5 / 22.4434 | 0.02085 / 0.05411 | +13 % / +7 % | +31 % / +18 % | +55 % / +46 % |

Single-pair c′ at the pitch point: **FE 17.06**, derived 15.60, Sainsot 13.65, verifier 11.61. ISO's c′_th is
16.03 and its c′ (C_M 0.8) is 12.50.

**Why the models differ.**
- **The verifier's 1.41–1.47× is all foundation**, 1.19 × 1.22:
  - 1.19 is Sainsot's fit against the half-plane;
  - 1.22 is its idealised root, which feeds Sainsot θ_f 0.118 instead of 0.177.
- Its straight root section adds 3 %, the shear coefficient 1–2 %.
- The derived tooth is +7…+13 %:
  - its beam part is 1.24–1.60× the FE, half of that from integrating past the centreline crossing,
    which the Weber term already covers;
  - its foundation is 3–9 % too stiff.
- ISO's c′_th is solid-disc theory: the FE is 1.06× it at h_f 1.4 and 0.91× at 2.0. C_M 0.8 is
  measured over theory, compliance outside any tooth model.

**Verdict.**
- The prototype's derived tooth is right to +7…+13 % (too compliant).
- The verifier's tooth is 1.46–1.72× too compliant, so its −9 % for C_a 5 µm understates relief. The
  prototype's −14 % stands, to within the derived tooth's +7…+13 %.
- For Rust: keep the derived foundation, stop the beam integral at the centreline crossing, and gate
  against the FE table.

### 2.5 Sweeps on the traced field (`sweep4_x.txt`, `sweep4_edge.txt`; analyse4, N 24, images, 2-D across)

**β continuity (17/43 m1 b10, μ .06), field max at β 0 / 0.1 / 2°** (flank max in brackets):

| r_e | 20 N·m | 60 N·m |
|---|---|---|
| unset | 3202.7 / **3650.7** / 4072.0 | 5519.0 / **5810.6** / 6863.7 |
| 0.2 | 4612.8 [2535] / 4629.2 / 5189.8 | 7785.9 [4331] / 7801.3 / 8317.3 |
| 0.1 | 5956.9 [2778] / 5984.9 / 6757.1 | 10043.6 [4694] / 10070.3 / 10897.4 |

- With r_e stated, β 0 → 0.1° is +0.4…+0.5 % (round three: ≤ 0.03 %). The β 0.1° maximum sits on the
  face-end station.
- **The sharp edge's β jump is continuous, but steep and linear in β** (`t_bj4*.py` → `bj4*.txt`).
  - Field max at β 0 / 0.001 / 0.01 / 0.1°: r_e unset 3202.7 / 3207.8 / 3253.0 / 3650.7; r_e 0.1 5956.9 /
    5957.2 / 5959.6 / 5984.9.
  - The mechanism: the sharp tip edge sweeps across the face, and a partly engaged edge line loads its
    face end (q 128 against 99 N/mm) for ±1e-3 p.
  - A kinked edge's gap rises linearly in s·β; a round's rises at second order. Round three's +0.4 %
    missed this narrow peak.
- **So r_e unset is refused in the expensive mode.**
  - Its figure is the flank convention's, not the r_e → 0 limit (which diverges, §2.2).
  - It moves 140 % per degree of helix.
  - The fast mode keeps ISO's points.
- The edge figures fall against round three by the step correction: r_e 0.1 is 5957 (was 6585); 0.2 is
  4613 (was 4989); 60 N·m r_e 0.1 is 10044 (was 11350). Edge share is 7–12 %.

**x (β20, 20 N·m), x −0.3 / 0 / 0.3 / 0.6:**
- r_e unset: 4493 / 2430 / 1808 / 1635 (round three: 4825 / 2319 / 1720 / 1543).
- r_e 0.1: 7302 / 5981 / 5761 / 5583 (round three: 7687 / 6859 / 7415 / 6869). It is now monotone:
  round three's x 0.3 jump to mid-face is gone.
- Flank max at r_e 0.1: 4441 / 2183 / 1700 / 1503. No fallback, non-convergence, lost valley or
  negative coupling in any run.

**Face width through ε_β = 1** (β20, 20 N·m, r_e 0.1), b 8 / 9 / 9.185 / 9.5 / 10 / 12:
- field max 6674 / 6244 / 6174 / 6084 / 5981 / 5594 (round three: 7748 / 7238 / 7150 / 7026 / 6859 / 6765);
- flank max 2434 / 2273 / 2247 / 2222 / 2183 / 2035.
- The curve is smooth and monotone through ε_β = 1, with nothing at 9.185.

**Σ at a_par** (β20, 2 N·m, r_e 0.1; analyse4, with the 201-phase grid beside it):

| Σ (deg) | 0 | 1e-4 | 1e-3 | 3e-3 | 0.01 | 0.1 | 1 | 10 |
|---|---|---|---|---|---|---|---|---|
| field max | 2019.7 | 2019.7 | 2019.9 | 2020.2 | 2021.4 | 2034.2 | 2291.0 | 6092.1 |
| grid 201 | 2013.3 | 2013.2 | 2012.7 | 2011.3 | 2008.7 | 2034.2 | 2280.1 | 6091.5 |
| flank max | 724.8 | 724.8 | 724.7 | 724.5 | 723.8 | 716.2 | 733.9 | 1509.6 |
| η % | 98.869 | 98.869 | 98.870 | 98.870 | 98.870 | 98.878 | 98.936 | 98.469 |

- The series is smooth. Round three's 2309 / 2303 / 2318 / 2415 / 2484 and the verifier's 2426.7 at 0.01°
  were the seeding noise, not steepness in Σ: 0 → 0.01° is now +0.08 %.
- The step correction brings the Σ 0 value to 2020 (round three: 2309).
- analyse4 is at or above the grid everywhere (by at most 0.6 %).

### 2.6 Matched worm wheel as a surface (contact-proto-4-worm: `mw_surface.py`, `mw_field.py`, `mw_report.txt`, `mw_play.txt`)

**The wheel.**
- The hobbed wheel's flank is the hob's envelope W(v, φ_h), with the meshing root a closed-form
  quadratic.
- A gap to it is a closest-point projection: a safeguarded Newton capped at 30, with 2.1–2.5 iterations.
- Its curvature is the rank-one update at the foot: FD agrees to 5e-13 and wormtypes.conjugate to 1e-14.
- **No new kind of solve.** The projection is a bounded minimisation like the anchor search. The hob-tip
  boundary is a clip.

**Reproduction** (tips as hard clips, as matched.py; the rigid gap on conjugate lines ≤ 7e-15 mm):
- ZI 1.982 lines, η 67.341 %, 1754.5 MPa (matched.txt 1.9875 / 67.343 / 1766.2);
- ZN 2.046, 67.287, 1852.1 (2.045 / 67.290 / 1842.3).

**Play now exists** (j_t2 at the wheel pitch circle, 8 phases):
- **Hob thinning:** j = s_h/(cos α_n cos γ).
  - Exact for ZI (1.000000): n·(z × X) = r_b cos β_b on the whole involute helicoid.
  - 0.995–1.009 for ZN.
- **Axial thinning:** j = s_x, exact.
- **Centre distance:** +Δa gives 0.54× the pitch-point 2Δa tan α_x, and −Δa 1.07–1.12×. The gap closes
  where n·x is least: a hobbed set localises.

**The edge rule now applies** (r_e on both gears, wheel 54.953 N·m, final code, pointwise Hertz as in
round three's involute rows):
- r_e 0.2: 6075 MPa on the worm's round, against the involute wheel's 8839 (−31 %);
- r_e 0.1: 8120, against 12169 (−33 %);
- η 67.36 / 67.33 %.
- The rows run on round three's valley (VField). The step correction will lower both wheels.

**Cost:** 240–285 s of Python tip-clipped, 614–635 s with rounds, against 540–880 s for the involute
valley field.

**Recommendation:** rate the worm on the matched wheel when a hob is stated (the wheel's tool input:
type, thinning, addendum), and on the involute wheel otherwise. Both take one field and one edge rule; a
worm is special only in how its wheel's flank is found.

### 2.7 Cost, measured (`t_cost4.py` → `cost4.txt`; Python CPU seconds, one analysis at 2 N·m)

| Mesh | CPU s | States | ms/state | Distance evals | Section minima | 2-D across | G | Depth nodes | Σn³ | Rust estimate |
|---|---|---|---|---|---|---|---|---|---|---|
| spur r_e 0.1, N 24 | 268 | 360 | 743 | 4.5 M | 0.14 M | 0.16 M | 62 M | 74 M | 83 M | ≈ 5 s |
| spur r_e 0.1, N 48 | 698 | 360 | 1938 | 5.7 M | 0.18 M | 0.32 M | 249 M | 294 M | 664 M | ≈ 19 s |
| helical β20 r_e 0.1 | 966 | 500 | 1933 | 63 M | 0.91 M | 0.36 M | 143 M | 177 M | 218 M | ≈ 14 s |
| crossed Σ10 r_e 0.1 | 766 | 552 | 1387 | 61 M | 0.83 M | 0.24 M | 97 M | 109 M | 85 M | ≈ 10 s |
| ring β15 r_e 0.1 | 609 | 428 | 1422 | 43 M | 0.49 M | 0.26 M | 101 M | 124 M | 145 M | ≈ 9 s |
| worm 1/40 r_e 0.1 | 986 | 544 | 1812 | 82 M | 1.21 M | 0.56 M | 187 M | 348 M | 319 M | ≈ 24 s |

- **Measured:** 2–16 min of Python CPU per mesh per load, at the panel counts §2.3 shows converged
  (24 with images).
  - One run with 0 non-converged, 0 refused and 0 lost, except the ring: 2 valleys lost beyond the field.
  - Round three's r_e-unset spur took 1.6 s estimated; this round's sharp spur is 124 s of Python.
- **Rust is an estimate** (no compiler may be run here): these counts × round three's unit costs
  (distance 35 ns, G 15 ns, a depth node 50 ns, LU n³/3 at 1 ns, contact2d ≈ 2 µs).
  - It is 5–24 s per mesh.
  - The depth-referenced kernel is 60–75 % of it: images triple it, and it is rebuilt at every b
    iterate.
  - Round three's b-independent kernel (two-term Taylor depth term, far-field G) cuts that part about
    10×, to about 2–8 s.
- **The step correction and the trace are cheap.** 2-D across solves are < 0.6 M per analysis, and
  section minima about 1 M.
- **Verdict (unchanged in kind):** the expensive mode, once per mesh per load case at the final solve,
  not in the search.

## 3. Carried from round three

- **Oracle and ε:** ISO 21771 j_n reproduced; a₀ 10/10; ε_γ 2.58060 against 2.580562; the interfering
  17/−43 x0 ring flagged. The rings' √ law carries.
- **ISO 6336-2 points** (`iso3.txt`) were not rerun (`t_iso4.py` is ready, about 2 h).
  - Their stations sit mid-line, where §1.1 and §1.4 change nothing.
  - §1.3 changes a reading only where a tangency lies within a Hertz width of B, C or D: the
    interfering ring at D (+26 % in round three).
- The width quadrature is exact to 1e-10 up to b/h ≈ 5.5; its cap never fired.

## 4. Open issues (size and sign)

- **Tooth plate coupling along the face** is not modelled. It lowers every end and edge peak.
- **The tooth is 7–13 % too compliant** (§2.4).
- **Images leave the face plane's normal stress.** An aligned end is high by ≤ 4.6 %.
- **Unaligned faces** need graded panels or a stated end relief (§2.3).
- **The expensive mode requires a C¹ tip form on every gear**, with no hidden default (§2.2, §2.5).
- **The matched worm rows** still use round three's valley field (§2.6).

## 5. Rust migration plan (changes from round three in bold)

1. `tools/contact_oracle.py` is the independent instrument. **Add `contact2d`'s discretised check
   (`t_c2d.py`) and the FE tooth (`fe/`) as instruments.**
2. `elliptic.rs`: aspect, hertz_shape, strip, Bernstein-ellipse width rule.
3. **`form.rs`: C¹ tip form (relief, round, land) and an end relief. A form that is not C¹ is refused.**
4. **`gap.rs`: the gap with forms; sections X·d = kΔζ with d one per mesh; the intrinsic section minimum
   (a fixed point, ≤ 8 rounds); the trace; margin roots on valley points; face crossings.** The worm's
   matched wheel is an optional second flank, a projection.
5. `tooth_compliance.rs`: the derived foundation, **with the beam integral stopped at the centreline
   crossing, and gated against the FE table.**
6. **`contact2d`: the closed-form across contact (I₀ = 0, q = (E*/2)cI₁, p by logarithms); Hertz when
   no step lies inside.**
7. `field.rs`: coupled lines, **span in one shot (D < Dmax asserted), images at each body's faces,
   cosine-graded panels where a line ends on one body's face**, Δ closed form, means and maxima.
8. **Rating:** ISO's points by default. `MeshReport` gains field, flank and edge maxima, each with where
   and on what, and the forms that produced them.
9. **Laws:**
   - continuity in a, x, b, b₂, β, Σ at δ 1e-9 (the §2.1 table as a law);
   - the verifier's case reproduced to 1e-8;
   - images converged in N (24/48 within 1e-6);
   - contact2d equals Hertz off a step, and the discretised solve within 0.2 % on one;
   - the step correction against the verifier's 2-D model;
   - matched wheel play: j = s_h/(cos α_n cos γ) for ZI exactly.
