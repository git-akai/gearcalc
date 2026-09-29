# Unified contact model: phase-exact prototype (the expensive mode)

Authors: contact-proto (2026-09-26/29), contact-proto-2 (round two), contact-proto-3 (round three),
contact-proto-4 (2026-09-29, round four, this revision). Read-only on the repo; pure Python, no numpy.
- **Scripts and outputs:** `~/.cache/gearcalc-work/contact-proto/` (README there). Round four's are
  `trace.py`, `b4.py`, `phase4.py`, `t_det4.py`, `t_cont4.py`, `t_sweep4.py`, `t_end4*.py`, `t_c2d.py`,
  `t_iso4.py`, `t_cost4.py`, the FE tooth in `fe/` and the matched wheel in `mw_*.py`. Round three's text
  is kept beside them as `contact-model.round3.md` (round two's as `contact-model.round2.md`).
- **Built on:** `work/plan.md` §5 (2026-09-30: the field model is the on-demand expensive mode) and
  `review/contact-verify4.md` (with `contact-verify3.md`).

**Status: ROUND 4 IN PROGRESS** (paused 2026-09-29 12:30 at the coordinator's request). Settled with
numbers: §1, §2.1 continuity, §2.2, §2.3, §2.4 and the x and edge sweeps (§2.5). Remaining: §3.

**Verdict of this round (so far).**
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

**An independent reference.**
- Plane-strain FE of the generated tooth: 9-node quads, the whole gear as a periodic ring, bore
  r_f/1.4 clamped.
- Checks:
  - element check: a Timoshenko cantilever within 0.5 %;
  - three meshes agree within 0.06 %;
  - a point load and a Hertz ellipse differ by ≤ 0.08 %;
  - the whole-gear value is bracketed by clamped and free sectors.
- It measures what `stiff.py` calls the tooth: the displacement along the load line of the load line's
  centreline crossing, per unit line load.

| r (mm) | FE | derived (prototype) | Sainsot | verifier (PE + Sainsot) |
|---|---|---|---|---|
| z17 8.0023 | 0.01626 | 0.01741 (+7 %) | 0.02218 | 0.02720 (+67 %) |
| z17 pitch 8.5 | 0.02226 | 0.02512 (+13 %) | 0.03033 | 0.03828 (+72 %) |
| z43 pitch 21.5 | 0.02085 | 0.02346 (+13 %) | 0.02741 | 0.03230 (+55 %) |
| z43 22.4434 | 0.05411 | 0.05776 (+7 %) | 0.06406 | 0.07891 (+46 %) |

(µm per N/mm.) Single-pair c′ at the pitch point, with the same Weber contact term: **FE 17.06**,
derived 15.60, Sainsot 13.65, verifier 11.61. ISO's c′_th is 16.03 and its c′ (C_M 0.8) is 12.50.

**Why.**
- **The verifier's 1.41–1.47× is all foundation.** At the pitch point it is 1.19 × 1.22:
  - 1.19 is Sainsot's fit against the half-plane;
  - 1.22 is the idealised root, which feeds Sainsot θ_f 0.118 instead of 0.177. That shrinks S_f and
    raises the foundation 1.29–1.51×.
- The straight root section itself adds 3 %; the shear coefficient 1–2 %.
- The derived model's own residual:
  - its beam part is 1.24–1.60× the FE, about half from integrating past the load line's centreline
    crossing, which the Weber term already covers;
  - its foundation is 3–9 % too stiff;
  - the net is +7…+13 %.
- ISO's c′_th is the solid-disc theory: the FE is 1.06× it at h_f 1.4 and 0.91× at h_f 2.0, so the
  body's size is worth ±10 %. C_M 0.8 is measured over theory, compliance outside any tooth model.
  Where it acts on every pair alike (a body's wind-up, 1e-5 here) it moves no load between pairs.

**Verdict.**
- The prototype's derived tooth is the right one, 7–13 % too compliant (sign known).
- The verifier's PE + Sainsot tooth is not: its −9 % for C_a 5 µm understates relief.
- The relief figure with the tooth scaled to the FE (`t_sweep4.py stiff`) is not run yet.
- **For Rust:** keep the derived foundation. Stop the beam integral at the load line's centreline
  crossing, the one change the FE supports. Gate the tooth against this FE (a law, as `stiff.txt` gates
  ISO), not against c′_th alone.

### 2.5 Sweeps on the traced field (`sweep4_x.txt`, `sweep4_edge.txt`; analyse4, N 24, images, 2-D across)

**β continuity (17/43 m1 b10, μ .06), field max at β 0 / 0.1 / 2°** (flank max in brackets):

| r_e | 20 N·m | 60 N·m |
|---|---|---|
| unset | 3202.7 / **3650.7** / 4072.0 | 5519.0 / **5810.6** / 6863.7 |
| 0.2 | 4612.8 [2535] / 4629.2 / 5189.8 | 7785.9 [4331] / 7801.3 / 8317.3 |
| 0.1 | 5956.9 [2778] / 5984.9 / 6757.1 | 10043.6 [4694] / 10070.3 / 10897.4 |

- With r_e stated, β 0 → 0.1° is +0.4…+0.5 % (round three: ≤ 0.03 %). The β 0.1° maximum sits on the
  face-end station.
- **Open defect: the sharp-edge convention (r_e unset) jumps +14 % (20 N·m) and +5 % (60 N·m) at
  β 0 → 0.1°**, also at the face-end station ('flank face'). Round three had +0.4 %.
  - Suspected: images or the face clip where the valley runs on the C⁰ kink.
  - Not yet isolated. It is one more reason to require a C¹ form (§2.2).
- The edge figures fall against round three by the step correction: r_e 0.1 is 5957 (was 6585); 0.2 is
  4613 (was 4989); 60 N·m r_e 0.1 is 10044 (was 11350). Edge share is 7–12 %.

**x (β20, 20 N·m), x −0.3 / 0 / 0.3 / 0.6:**
- r_e unset: 4493 / 2430 / 1808 / 1635 (round three: 4825 / 2319 / 1720 / 1543).
- r_e 0.1: 7302 / 5981 / 5761 / 5583 (round three: 7687 / 6859 / 7415 / 6869). It is now monotone:
  round three's x 0.3 jump to mid-face is gone.
- Flank max at r_e 0.1: 4441 / 2183 / 1700 / 1503. No fallback, non-convergence, lost valley or
  negative coupling in any run.

## 3. ROUND 4 IN PROGRESS: what remains, exactly

1. **Isolate the r_e-unset β jump** (§2.5).
   - Rerun `t_det4`-style at β 0 and 1e-3° with images off and on, at the face-end station.
   - Compare P['faces'] and the stations' ends.
2. **Σ at a_par** (`python3 t_sweep4.py sigma 3`, ~50 min; killed unfinished) and **face**
   (`t_sweep4.py face 3`, killed unfinished).
   - Report the Σ 0 / 1e-4 / 1e-3 / 3e-3 / 0.01° series against the verifier's 2426.7 at 0.01°, with the
     201-phase grid beside analyse4.
3. **Remaining sets:** `conv` (panels, helical and crossed), `stiff` (relief with ct × 1/1.1: the FE's
   verdict on C_a 5 µm), `relief`, `ring`, `worm`; `python3 t_iso4.py 3`; `python3 t_cost4.py` (cost
   measured: CPU s and counts at the converged N, with a Rust estimate labelled as one).
4. **Matched worm** (priority 5, contact-proto-4-worm; `mw_report.txt`, `mw_*.py`).
   - The hobbed wheel is a surface: the hob's envelope, reached by a closest-point projection (a capped
     2-D Newton), with the closed-form rank-one curvature. The checks hold to about 1e-13.
   - **No new kind of solve.**
   - Tips clipped as matched.py does: ZI 1.982 lines / η 67.341 / 1754.5 MPa; ZN 2.046 / 67.287 / 1852.1.
     Both are within 0.7 % of matched.txt.
   - Play:
     - hob thinning gives j = s_h/(cos α_n cos γ) exactly;
     - Δa +0.1 mm gives 0.043 mm against 0.0735 at the pitch point (not understood yet);
     - the ZN and remaining Δa rows were not run.
   - Edge, with rounds on both gears, **not rerun after the helper's last fix**: r_e 0.2 gives 6392
     (wheel's round) against the involute wheel's 8839; r_e 0.1 gives 8694 (worm's round) against 12169.
   - Cost: 240–285 s of Python per analysis, 750–920 s with rounds. Rust +1.7…6 s is an estimate.
   - Next: rerun the edge rows and the Δa rows under the final code.
5. Then fold the round-three sections still valid (oracle, ε, rings, ISO, worm table, Rust plan) back
   in from `contact-model.round3.md`, updated. Keep the doc ≤ 400 lines.
