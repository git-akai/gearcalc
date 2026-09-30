# Unified contact model: phase-exact prototype (the expensive mode)

Authors: contact-proto and contact-proto-2…4 (rounds one to four); contact-proto-5 (2026-09-29, round five,
this revision; the matched wheel by contact-proto-5-worm). Read-only on the repo; pure Python, no numpy.
- **Scripts and outputs:** `~/.cache/gearcalc-work/contact-proto/` (README lists them; earlier rounds'
  texts are there as `contact-model.round{2,3,4}.md`).
- **Built on:** `work/plan.md` §5 (2026-10-01: two modes; the matched wheel implemented and not exposed;
  the expensive mode refuses an unset r_e) and `review/contact-verify5.md`.

**Status: complete for round five.** Every table was run on the final code (`trace.py`, `coupled.py`);
every process was capped at 1.8 GB, pools ≤ 4.

**Verdict of this round.**
1. **The state is continuous where the zero of sliding crosses a line** (§1.1, §2.1). Friction is the
   exact mean of Coulomb's direction over each panel, in closed form. At the verifier's phase and where the
   zero crosses a panel's centre or boundary, every input (a, x₁, b, b₁, β, Σ, T) at δ 1e-9 … 1e-5 moves D
   and p in proportion to δ; round four's code gives 1.36e-3 at δ 1e-9. Dense scans (4 × 1000 phases) find
   no jump. One genuine jump remains: **a spur line at the pitch point**, where Coulomb friction reverses
   on the whole line at once (−4.27 % load, −2.16 % p_max, exactly the closed form). Carter's creep
   would make it a ramp over 3.2e-3 p; it is recommended for Rust, not taken here. The scans also found a
   pair dropped while loaded (D +0.18 %), now fixed (§1.4).
2. **The report is continuous** (§1.2, §2.2): p_max and where; p_flank and p_edge as maxima over regions
   fixed by the forms' tangency lines; the edge's load share; a load-weighted length. p_flank now includes
   the flank beside a straddled tangency, so the flank's own level is ISO's points' job.
3. **Face ends: keep the mirror, state its bias, flag the case** (§2.3). The mirror is a smooth wall, so a
   face-end maximum is high: by 1.44 % against a free end bracketed from Guilbault's published −21.9 % edge
   displacement, 0.20 % more for the collinear image on a helical line. Mid-face maxima move ≤ 0.04 %, and
   a stated end relief makes the end immaterial (−0.005 %).
4. **The tooth reads the gear's own rim** (§1.3, §2.4). Unset, it is ISO 6336-3's least thick rim
   (1.2 h_t, 3.5 m_n for a ring). The derived c′ is −9.2 … +7.2 % of an independent FE over the rim and
   0.995× ISO's c′_th at the default. ISO's c′ = 12.50 carries C_M 0.8, measured over theory, which no model
   reproduces: the field's mesh is 1.28× stiffer; its maxima are low by < 1 % for it, but a relieved
   design's by 6 % (relief's benefit overstated: −21 % against −17 %).
5. **The matched worm wheel is robust and implemented, not exposed** (§2.5). Round four's caps and
   fallbacks came from the hob-tip cut; the envelope continued past it, a table seed and the traced
   field's section minimum leave 0 failures in 9 M projections. Like for like its maximum is 33–34 %
   below the involute wheel's: the verifier's −17 % compared two fields.
6. **`trace.py`'s docstring** says 2.77e-10 mm (the unit slip of 2.8e-13 is gone).
7. **ISO 6336-2 examples on the final code** (§2.6): at ISO's points −14.5 … +11.9 % of σ_H on the
   external gears, within 2.2 points of round three; two sharp-edge ring rows moved by up to 29 points
   against round three, from round four's traced field (checked), not from this round.
8. **Stage 3 migration plan** (§5): steps F1–F9 with their gating laws, the two-mode wiring, and the audit tasks.

## 1. What changed in the model (`trace.py`, `coupled.py`)

### 1.1 Friction is integrated over each panel
- Round four took Coulomb's direction at each station's centre, so a station crossing the zero of sliding
  flipped its whole panel's friction: D jumped 0.1–0.2 % about 22 times per pitch (contact-verify5 §1).
- The sliding velocity is affine in position (v₁ − v₂ = Ω × X + const, Ω = a₁ − (ω₂/ω₁) a₂), so along a
  panel's chord its tangential part is A + B u, u ∈ [−½, ½], with q constant on the panel. The panel's
  friction is −μ q ē, ē the mean unit direction, in closed form (`panel_slide`; w = u − u₀, h the line's
  offset from the origin, R = √(|B|²w² + |h|²)): ∫ (B w + h)/R dw = B R/|B|² + h asinh(|B| w/|h|)/|B|.
  - When A ∥ B (every Σ = 0 mesh) h = 0 and this is the exact integral of sgn(w): the panel is split
    where sliding is zero. When Σ ≠ 0 it is the exact mean of a smoothly rotating direction.
  - Differences are written without cancellation (w₂ − w₁ = 1); the mean sliding speed has the same form.
- The slices seed splits its Gauss rule at the same zero. Coulomb's own discontinuity stays inside the
  panel; the panel's force is continuous. `friction='point'` restores round four.
- A spur line (β = 0, Σ = 0) is parallel to the pitch line, so its whole friction reverses at one phase:
  the exact Coulomb force of the line, not a discretisation (§2.1).

### 1.2 The report is continuous
- `pflank` and `pedge` are the maxima of the pressure over two **regions**, the flank and the edge (a
  round and the tip land beyond it, of either member), fixed by the forms' tangency lines. The pressure
  is continuous for a C¹ form, and a maximum of a continuous field over a closed region whose boundary
  moves continuously is continuous. Round four classed each station by where its peak sat, so a whole
  station switched class: pflank 427 → 1513 MPa between adjacent phases (c10).
- `Fedge` is the load carried on the edge region: each strip's pressure integrated over its edge part
  (closed form for Hertz; Gauss between the section's curvature steps otherwise).
- `L` is the load-weighted length (∫q)²/∫q²: the length itself for a uniform line, and a line entering
  at q → 0 adds nothing. Round four counted loaded panels, so L jumped 20 → 10 mm as a spur line unloaded
  (s0), and that reached the seed through the nominal line load F/L.
- `at` (where the maximum sits) is information: a location jumps where two maxima trade places.
- A region's maximum is sampled on a fixed θ grid (48 nodes on [0, π], t = m + c cos θ) clipped to the
  region, plus its ends, then refined by golden section.

### 1.3 The foundation reads the gear's own rim (`coupled.rim_of`)
- The derived foundation is referenced at the depth of the gear's own rim, H = s_R, for either kind (the
  bore of an external gear, a ring's outer rim): the crate's `MemberGear::rim_thickness`.
- Unset, s_R is the thickness ISO 6336-3:2019 §9.3 first calls thick: 1.2 h_t for an external gear,
  3.5 m_n for a ring. That is also what the crate rates an unset rim at (Y_B = 1), so one field is read
  one way for bending and for stiffness. An external body is at most solid (s_R ≤ r_f: a worm is a shaft).
- Round four's hidden defaults were r_f/r_int = 1.4 for an external gear and 3.5 m_n for a ring.

### 1.4 A pair is no longer dropped while loaded
- `loaded` grew a pair's loaded interval from y0, the lowest trace node inside the field. With the
  valley's minimum at a clipped face end, the pair vanished while loaded once D fell below that node's gap
  (h20 0.9978 p: 0.47 N on 14 panels, D +0.18 %; found by the dense scan). y0 is now the valley's true
  minimum on the clipped segment (the ends and the nodes, refined by golden section): continuous.

## 2. This round's items

### 2.1 Continuity (`t_flip5.py` → `flip5.txt`, `t_scan5.py` → `scan5_*.jsonl`, `t_spur5.py` → `spur5.txt`)

**At the phases where friction's sign flips inside a line** (h20: 17/43 β20 r_e 0.1, 2 N·m, Σ 0): the
verifier's (0.0239437 p); where the zero of sliding crosses a panel's centre (0.0242536 p) and boundary
(0.0519576 p), bisected; where round four's pointwise friction flips (0.0242716 p: D −0.136 %, reproduced).

Every input moved by ±δ; the largest relative change of D and of the state's field max p:

| Input | \|ΔD/D\| at δ 1e-9 | 1e-7 | 1e-5 | \|Δp/p\| at 1e-9 | 1e-7 | 1e-5 | Round four at its flip, δ 1e-9 (D / p) |
|---|---|---|---|---|---|---|---|
| a (mm) | 2.75e-7 | 2.75e-5 | 2.75e-3 | 3–7e-9 | 3.3–4.8e-7 | 3.3–4.7e-5 | 1.36e-3 / 6.3e-4 |
| x₁ | 2.75e-7 | 2.75e-5 | 2.75e-3 | 0.6–4e-9 | 7.0–8.8e-8 | 7.0–8.7e-6 | 1.36e-3 / 6.3e-4 |
| b (mm) | 7–8e-11 | 7.0e-9 | 7.0e-7 | 0.7–3e-9 | 2.3–3.2e-8 | 2.4–3.2e-6 | 1.36e-3 / 6.3e-4 |
| b₁ (mm) | 6.5–8e-11 | 6.6e-9 | 6.6e-7 | 1.4–4e-9 | 2.4–3.3e-8 | 2.3–3.1e-6 | 1.36e-3 / 6.3e-4 |
| β (deg) | 5.6e-8 | 5.6e-6 | 5.6e-4 | 0.9–3e-9 | 6.8–10e-8 | 7.0–9.9e-6 | 1.36e-3 / 6.3e-4 |
| Σ (deg) | 4.0e-8 | 4.0e-6 | 4.0e-4 | 2–3e-9 | 4.8–7.2e-8 | 4.9–6.9e-6 | 1.21e-3 / 5.6e-4 |
| T (relative) | 9.5e-10 | 9.5e-8 | 9.5e-6 | 0.5–7e-9 | 1.2–1.6e-7 | 1.2–1.5e-5 | 1.36e-3 / 6.3e-4 |

- The ranges are over the four phases; each column is proportional to δ, and p sits at its 0.5–7e-9 floor
  (the strip-width fixed point's tolerance) at δ 1e-9. **No step anywhere.** D's large sensitivity to a
  and x is the rigid approach (ΔD = 0.343 Δa on a D of 1.25 µm).
- **Dense scans, 1000 phases per pitch** (the verifier's jump detector: a neighbour difference > 2e-4
  and > 8× both adjacent ones), on h20, c1, c10, s0:
  - no jump in D, p_max, F or L on any base. Round four's code, the same detector: h20 20 in D, 24 in F,
    12 in p_flank, 6 in L; c10 14 in p_flank and 30 in L; c1 4 and 9; s0 1 in p_flank and 2 in L;
  - two `pflank` suspects (h20 at 0.454 p, c1 at 0.562 p) bisect to steep but continuous stretches: the
    change shrinks with the interval (h20: 6.4e-5 at 1.6e-5 p, 2.0e-6 at 4.9e-7 p, 2.9e-9 at 4.8e-10 p; `zoom5_*.txt`),
    a maximum changing branch;
  - one more defect found on the way and fixed (§1.4): a pair dropped while loaded, D +0.18 %;
  - no refused span, no non-converged fixed point, no lost valley in 4000 states. (Round four's
    `sweep4_edge` spur rows carried `nonconv` 1 in 216–360 states, which that doc did not say.)
- **The one jump that stays: a spur line at the pitch point** (s0, 20 N·m, at exactly 0.75 p):
  F 2559.85 → 2450.43 N (−4.27 %), D 16.164 → 15.503 µm (−4.09 %), p_max 2103.96 → 2058.50 MPa (−2.16 %).
  - The whole line crosses the pitch line at one phase, so Coulomb friction reverses on all of it at once.
    Its size is the friction torque reversing: the load goes from T/(r_b − μρ) to T/(r_b + μρ), −4.3 % with
    ρ 2.91 and r_b 7.99 mm.
  - No quadrature removes it; physics the model leaves out does. Carter's 2-D rolling contact (Johnson,
    *Contact Mechanics* §8.2) gives the friction force μq[1 − (1 − |ξ|/ξ*)²] for creepage |ξ| < ξ* = μc/R,
    Coulomb beyond. Here (q 256 N/mm, c 0.078, R 2.08 mm) ξ* = 2.2e-3: ±4.7 µm of roll, **3.2e-3 of a
    pitch**. Recommended for Rust as a named option; not taken in this round.
- The verifier's two non-proportional entries in round four's table (spur "β from 0": 1e-7 → 4.3e-9 but
  1e-5 → 7.7e-6; "Σ from 0": 1e-5 → 1e-6 but 1e-3 → 2.8e-4) are the edge's narrow β window and a
  superlinear onset, not the floor round four called them.

### 2.2 The report (`strip_report`, `TField.state`)

- The field maximum, the flank and edge maxima as regions, and the edge's load share are continuous (§1.2),
  and the dense scans find no jump in them. Where the maximum sits is information.
- **What `p_flank` now means:** it includes the flank beside a straddled tangency, which the round raises
  (s0, 20 N·m: 4783 MPa against p_max 5940; round four's station-classed 2778). That is what the flank next
  to the edge carries. The flank's own level, away from any edge, is what ISO's points read (§2.6).
- **Recommended for `MeshReport.field`:** p_max with its location and region, the edge's load share,
  p_flank and p_edge as region maxima, and the ISO-point readings. The weighted alternative (a continuous
  weight between regions) needs a length over which the weight turns, which nothing in the model supplies:
  a magic number.

### 2.3 Face ends (`t_sens5.py` → `sens5.txt`, `t_sens5b.py` → `sens5b.txt`)

**What the mirror is.** A symmetric image makes the shear and the normal *displacement* vanish on the
plane: a frictionless rigid wall, not a free face (normal *stress* zero). The verifier is right, and round
four's "at most √(1−ν²) = 4.6 % high" was a heuristic, not a bound.
- **Sign.** Removing a constraint can only soften a body (minimum potential energy): a free end is more
  compliant than the wall, so the mirror overstates the load, and the pressure, at a face end.
- **Size of the displacement error, published.** The mirror method's maximum edge displacement is
  21.9 % below FE for an elastic quarter-space at ν 0.3 (Guilbault, *A fast correction for traction-free
  surface of elastic quarter-space*, WIT Trans. Eng. Sci. 66, 2010; −11.3 % at ν 0.15; his correction
  factor ψ on the mirror load leaves −9.6 %).
- **The collinear image on a helical line.** `kmatrix` reflects the load about the plane normal to the
  line; the face plane is inclined β_b to it. At the crossing the true image is 1/cos β_b as strong (a point
  d from the face sees it at 2d cos β_b, not 2d): 1.056 at β_b 18.75°. A weaker image is a stiffer end, so
  in end compliance: the half-space running past the face (round three) < the collinear mirror < the
  face-plane mirror < the free end. Both biases have the same sign.

**Bracket on the field** (analyse4, N 24; `image_scale` ψ on each body's image, an instrument: 1 the
model; 1.056 the face plane's strength; 1.561 a free end, edge displacement (1 + ψ)/2 = 1/(1 − 0.219)):

| Case (r_e 0.1 both, μ 0.06) | Where the max sits | ψ 1 (model) | ψ 1.056 | ψ 1.561 (free end) |
|---|---|---|---|---|
| helical β20, 2 N·m | the face end (z +4.99) | 2010.8 | 2006.8 (−0.20 %) | 1981.8 (−1.44 %), 25 µm inboard |
| helical β20, 2 N·m, end relief 5 µm over 1 mm | 1 mm inboard (z +3.94) | 2158.8 | – | 2158.7 (−0.005 %) |
| helical β20, 20 N·m | mid-face (z −1.80) | 5946.6 | 5946.8 | 5948.0 (+0.02 %) |
| helical β20, 20 N·m, end relief | mid-face (z −1.95) | 6067.9 | – | 6068.2 (+0.005 %) |
| spur, 20 N·m | mid-face | 5939.9 | – | 5942.0 (+0.04 %) |
| spur, 20 N·m, b₂ 10.5 (the pinion's ends only), graded | the face end (z −4.99) | 6036.3 | – | 5968.3 (−1.13 %) |

- **Where the maximum sits at a face end, the mirror overstates it by about 1.5 %** (1.44 % on h20, of
  which 0.20 % the collinear image; 1.13 % where only one body ends). Mid-face, a softer end moves load
  inward and the maximum rises by at most 0.04 %.
- **A stated end relief makes the end condition immaterial** (−0.005 %), because the relieved end carries
  almost nothing. At 2 N·m this relief (5 µm, four times the approach) shortens the line and raises the
  maximum by 7.4 %: an end relief is a design input with its own cost, not a numerical device.

**Recommendation: keep the mirror, state the bias, flag the case.**
- The mirror is one image per face, converged, exact for the condition it states, and ≤ 1.5 % high at a
  face-end maximum (nil elsewhere). Rust flags a maximum at a face end (`at`), so the bias shows where it
  applies; end relief is the member's input (`ToothForm::end_relief`, default none).
- A free end is Hetényi's quarter-space iteration: a correction field over each face plane per panel, an
  order of magnitude more kernel work to move a figure by 1.5 %. Guilbault's fast factor is itself 9.6 %
  off. The iteration is the instrument if a face-end maximum ever governs a rating; the exact face-plane
  image (0.2 %) would need a second kernel. Neither is worth its cost now.

### 2.4 Tooth stiffness read at the gear's own rim (`t_rim5.py` → `rim5.txt`)

Single-pair c′ (N/(mm·µm)) at the pitch point of 17/43 m1, the derived tooth against the independent
plane-strain FE of the whole gear with its bore clamped at r_f − s_R. "Local" removes the rigid wind-up
of the ring opposite the load, which is common to every pair in mesh and moves no load between them.

| Rim s_R (z17 / z43, mm) | FE c′ | FE local | Derived c′ | Derived / FE local | Derived / ISO c′_th |
|---|---|---|---|---|---|
| 2.07 / 5.79 (round four's r_f/r_int 1.4) | 17.06 | 17.06 | 15.60 | −8.6 % | 0.973 |
| 3.63 / 10.13 (r_f/r_int 2.0) | 14.61 | 14.94 | 14.87 | −0.5 % | 0.927 |
| 5.44 / 15.19 (r_f/r_int 4.0) | 10.10 | 13.41 | 14.38 | +7.2 % | 0.897 |
| **2.70 / 2.70 (default: 1.2 h_t)** | **17.53** | **17.57** | **15.95** | **−9.2 %** | **0.995** |

- **The error band of the derived tooth is −9.2 … +7.2 % over the rim**, and it has a trend: the derived
  foundation under-responds to depth (−8 % from the thinnest to the deepest body, against the FE's −21 %
  locally). It is too compliant on a thin rim and too stiff on a deep one.
- At the default rim the derived c′ is **0.995 × ISO's theoretical c′_th (16.03)** and +2.0 % over ISO's
  theory for this rack (C_B 0.975: 15.63).
- **ISO's c′ = 12.50 is theory × C_M 0.8 × C_B 0.975.** C_M is empirical: measured single-pair stiffness is
  20 % below theory (the gear body, shaft, bearings and mounting, and deviations). No tooth model contains
  it, the FE included. **Size and sign:** the field's mesh is 1.28× stiffer than ISO's measured-based c′
  (the FE's 1.40×): at a given load it deflects 22 % less than ISO's c′ says (29 % for the FE).
- **What C_M would do to the field** (`sens5.txt`: both teeth's compliance × 1.332, which lowers c′ by 0.8;
  analyse4, 20 N·m, r_e 0.1):

  | Case | Tooth as derived | × 1.332 (C_M) | Edge share |
  |---|---|---|---|
  | helical β20 | 5946.6 | 5995.2 (+0.8 %) | 0.048 → 0.054 |
  | spur | 5939.9 | 5975.4 (+0.6 %) | 0.051 → 0.058 |
  | helical β20, tip relief C_a 5 µm | 4693.5 (face end) | 4977.4 (+6.0 %, mid-face) | 0.018 → 0.025 |

  Softer teeth put more load on the edge. Unrelieved, the field's maxima are low by under 1 % for the
  missing C_M. **Relieved, by 6 %: relief buys −21.1 % on the derived teeth but −17.0 % on teeth as soft as
  ISO's c′ says** — the verifier's point that absolute compliance sizes relief, now with a size. Not
  conservative. Rust should offer C_M as a named, visible factor on the tooth, default 1 (theory), and
  report a relieved design at both.

### 2.5 The matched worm wheel (contact-proto-5-worm: `mw_tfield.py`, `mw5.txt`)

**Why round four's solve capped and fell back** (an instrumented copy on round four's field, 321 k
projections): 5.9 % had a finite-difference neighbour beyond the hob-tip cut, where the surface is
undefined, and 2.6 % pinned their foot on that edge; both were returned unconverged (0.15–3 mm off) and
turned into a +1 mm step in the gap. The cap was hit where a seed lay a hob turn away and Newton walked to
its ±2π box. The parabolic across-minimiser failed in 23.4 % of valleys: near a form's curvature step the
section is not a parabola, and the +1 mm steps broke it too.

**The fixes.**
- The envelope is continued analytically 1 mm past the hob's tip; the generated flank's edge is a field
  margin like the root circles. The gap no longer jumps.
- Newton on the normal-line equations with one-sided differences on the side that exists, no box on the
  hob angle, seeds from a 25 × 25 table of the pair's own surface, converged at 1e-12 mm; every loop capped.
- Valley points are the traced field's section minimum (one bracketed root), so no fallback path exists;
  the flank curvature is the rank-one tensor at the foot (the `flank_curv` hook).
- Over whole analyses (8.6–9.3 M projections, about one iteration each): 0 stalled, 0 failed; the cap is
  reached twice per analysis at r_e 0.1 by a probe outside the field creeping along the continuation's
  bound, and is counted as 'edge' there. The conjugate-line gap is 7e-15 mm; hob and axial thinning give
  play to 1.000000000 of the formulas.

**Like for like on this round's field** (analyse4 N 24, μ 0.06, r_e on both members; every maximum on the
worm's tip round):

| r_e | Load | Matched | Involute wheel | Change | η matched / involute |
|---|---|---|---|---|---|
| 0.2 | wheel 54.953 N·m | 5086 | 7570 | −32.8 % | – |
| 0.2 | worm 2 N·m | 5011 | 7539 | −33.5 % | 67.12 / 67.95 % |
| 0.1 | wheel 54.953 N·m | 6433 | 9633 | −33.2 % | – |
| 0.1 | worm 2 N·m | 6323 | 9590 | −34.1 % | 67.01 / 67.86 % |

- **The verifier's −17 % compared across two fields**: the matched wheel on round three's valley field
  (7976) against the involute wheel on round four's (9641). On one field the matched wheel reads 6323:
  the valley field over-read it by 26 %. Round four's −31…−33 % stands in size.
- The matched wheel carries 2.2 pairs on 9.6 mm of line against the involute wheel's 3.2 pairs on 4.2 mm
  (the involute wheel is not conjugate to the worm), and its efficiency is 0.84 points lower.
- Cost: equal in Python; the projection adds about 3–5 s in Rust. Only ZI was run; the runs predate the
  last two `trace.py` fixes, and a spot check after them reads 6432.93, unchanged.
- **Status (plan §5): implemented, not exposed.** The solve is now robust, which was the ruling's reason;
  exposing it is the owner's call.

### 2.6 ISO 6336-2 standard examples on the final code (`t_iso4.py` → `iso4.txt`)

As rounds two and three: μ 0, no relief, the derived tooth (this round's rim default); ISO's points read
where a pair's valley crosses the point's radius in the mid-plane, both one-sided limits in phase. The
form is r_e = 0.1 m_n on both members (the expensive mode's case); the sharp edge (refused in the
expensive mode) in brackets. Relative to ISO's σ_H at the same point:

| Case (load) | C | B | D | Pinion | Wheel | Field max (where) |
|---|---|---|---|---|---|---|
| FZG-C-like spur 16/24 m4.5 (302 N·m) | +8.7 % (+8.7) | −14.5 % (−17.8) | −13.3 % (−18.5) | +1.6 % | +8.7 % | 4400 edge (1770 flank) |
| helical 17/43 β20 m2 (60 N·m) | +1.7 % (+2.1) | +11.9 % (+11.7) | −5.0 % (−3.3) | +11.9 % | +1.7 % | 3769 edge, face end (1584 flank, face end) |
| helical 20/60 β15 m4 (500 N·m) | +3.9 % (+4.6) | +8.7 % (+9.6) | −0.7 % (−0.1) | +8.7 % | +3.9 % | 3423 edge, face end (1240 flank) |
| ring spur 17/−43 x0/−0.3 m2 (60 N·m) | −10.2 % (−10.2) | −10.0 % (−11.4) | +38.0 % (+38.0) | −10.0 % | +38.0 % | 5667 edge (12326 flank) |
| ring helical 17/−43 β15 m2 (60 N·m) | +3.7 % (+6.0) | +5.2 % (+8.9) | −11.1 % (−6.9) | +5.2 % | +3.7 % | 3409 edge (1161 flank) |

- **Against round three** (its valley field, round four's hidden rim), the r_e rows moved by at most
  2.2 points at every ISO point (FZG C +8.3 → +8.7, B −14.2 → −14.5; 17/43 B +12.1 → +11.9; 20/60 B +8.5 →
  +8.7; ring helical D −8.9 → −11.1). The ISO points sit mid-line, where this round's changes act only
  through the load share, as round four predicted.
- **The rings with the sharp edge moved more** (ring helical C/B/D +2.6/+5.6/−9.9 → +6.0/+8.9/−6.9; the
  interfering ring spur's D +9.2 → +38.0 %, now equal to its r_e row). Round four's traced field at round
  four's rim reads the same (`iso5r4.txt`), and this round's rim default moves them ≤ 0.1 point
  (`iso5r.txt`): it is round four's line construction on a sharp ring edge, which round four did not rerun
  and said would not move. The sharp edge is the convention the expensive mode refuses, and the ring
  spur interferes (the oracle flags it): at its D the ring's tip meets the pinion's flank, and no form
  cures that.
- **At ISO's points the field reads −14.5 … +11.9 % of σ_H on the external gears**, as in round three:
  above ISO at B on both helicals, below at B and D on the spur, near it at C.
- **The field maximum is 2.7–4.0× the largest σ_H on the external gears with r_e 0.1 m_n** (5.3–6.4× on
  the rings), always on a round, twice at a face end: edge contact without tip relief, which no ISO point
  sees. With the sharp edge (the flank convention) it is 1.1–1.6× on the external gears; that convention
  has no finite limit (round four).

## 3. Carried from round four

- The traced, seed-independent valley (≤ 1.3e-8, contact-verify5 a2); the continuous span; the exact 2-D
  across contact (the verifier's BEM: < 1e-5 on the same section); images; the sharp edge refused
  (r_e → 0 diverges as r_e^−0.37); the oracle (ISO 21771 j_n, a₀, ε_γ).
- Cost (2–16 min of Python per mesh per load; Rust 5–24 s, ±2×). This round adds one closed form per
  station and one to three `strip_report` calls per loaded station: h20 stays ≈ 1 s per state.

## 4. Open issues (size and sign)

- **The spur pitch-point reversal** (§2.1): −4.3 % of the load, −2.2 % of p_max, at one phase. Carter's
  creep removes it (window 3.2e-3 p); not yet taken.
- **The free end is a smooth wall** (§2.3): a face-end maximum is high by about 1.5 %; mid-face maxima
  and relieved ends are unaffected.
- **The derived tooth:** −9.2 … +7.2 % in c′ over the rim against the FE; the whole-mesh compliance is
  1.28× stiffer than ISO's measured-based c′ (C_M 0.8), which no model reproduces: maxima low by < 1 %,
  a relieved design's by 6 % (§2.4).
- **Tooth plate coupling along the face** is not modelled; it lowers every end and edge peak (round four).
- **The sharp edge** stays refused in the expensive mode (r_e → 0 diverges). A physical finite value needs
  local yielding, which is plan §5's singular-field track (2026-10-01, `work/notch-research.md`).
- **The matched worm wheel:** only ZI was rerun; ZN/ZA project the worm too and are untested on this field.

## 5. Stage 3: the Rust migration plan

The field model lands as the **expensive mode**, beside today's closed forms, which stay the **fast mode**.
It is additive: no fast-mode number moves, and every step lands under the identity harness (`gear-cli
identity`) with the fast mode bit-identical. It runs as plan §3's research track S-C, after **G** (one
generator: the form circle and root the field clips at) and beside **R**, **L** and **K**.

### 5.1 Two modes, one wiring

| | Fast mode (default) | Expensive mode (on demand) |
|---|---|---|
| What | today's closed forms at ISO's points: `contact.rs` (line path, load share, efficiency), `screw.rs` (crossed path), `hertz.rs`, `strength::contact_stress` | the field: every pair's line, the coupled load, the exact 2-D across contact, friction per panel, maxima over phase |
| When | every solve, the search, the panel's live figures | once per mesh per load case, at the final solve, when asked |
| Cost | microseconds | 5–24 s per mesh per case (estimate, ±2×; §2.7 of round four) |
| Inputs | as today | as today, plus each gear's form (`tip_relief`, `edge_radius`, `end_relief`) and `rim_thickness`, and a named stiffness factor C_M (default 1, theory; ISO's 0.8 the stated alternative, §2.4) |
| Refuses | as today | a gear with no stated `edge_radius` (`field.edge_radius_unset`): the sharp edge has no finite value (r_e^−0.37) |
| Reports | `MeshReport` as today | `MeshReport.field: Option<FieldReport>`: `p_max` and where, `p_flank`, `p_edge` (region maxima), `edge_share`, `efficiency`, `length` (load-weighted), the ISO-point readings, and the forms and rims used |

- One request type, `Rating { Fast, Field }`, on the case the caller asks about; the train's solve is
  unchanged. `Field` runs `rate` first, then the field per mesh on the rated loads (`CaseLoad`), so the
  field sees exactly the torques the fast mode rated.
- The search never calls the field. A law holds the two modes to agree where both apply (§5.4).
- Boundary: one new wasm entry point `field_rate(train, case, mesh)` (JSON in and out, pure), run in a
  web worker; a CLI row `gear-cli field` with its output recorded in the corpus on one small case.

### 5.2 Modules and data types (all in `crates/gear-core/src/field/`)

| Step | Module | Types | What it is | Replaces / absorbs |
|---|---|---|---|---|
| F1 | `form.rs` | `ToothForm { tip_relief: Option<Relief>, edge_radius: Option<f64>, end_relief: Option<Relief> }`, `Relief { depth, length }` | the gear's tip and end form as one C¹ function of one coordinate along the flank normal (relief, the round tangent to flank and land, the land); refuses a form that is not C¹ | new inputs on `MemberGear` (principle 10); the fast mode ignores them |
| F2 | `gap.rs` | `FieldMember` (from `BuiltMember`: signed z, helix, flank closed form, form, circles, faces), `Gap`, `Section { d, dz }`, `Valley { nodes, ends }` | the exact rigid gap of two members with their forms; the family of sections X·d = kΔζ (one d per mesh); the intrinsic section minimum (a fixed point, ≤ 8 rounds, one bracketed root per round); the trace; the ends as roots of the field margin on valley points | nothing in the fast mode; the anchor search becomes a seed only |
| F3 | `across.rs` | `Strip { c, m, pressure }`, `StripReport { p_max, t_max, p_flank, p_edge, edge_load }` | the frictionless 2-D contact of a piecewise-quadratic section in closed form (I₀ = 0, q = (E*/2) c I₁, p by logarithms); Hertz when no step lies inside; the region maxima | `hertz::line_pressure` inside the field (the fast mode keeps it) |
| F4 | `kernel.rs` | `Kernel`, `Panel` | the lengthwise influence G(r) of a strip along a line (closed-form derivative through R_D, `elliptic.rs`), depth-referenced; panel integrals; each body's mirror image at its face crossings; the Bernstein-ellipse width rule | T08.11's proposed DC-FFT instrument (this is its job, exactly) |
| F5 | `compliance.rs` | `ToothCompliance` (Chebyshev in depth below the tip) | the derived tooth: potential-energy beam over the generated profile (from `tooth.rs`) + the half-plane foundation (L* = 18(1−ν²)/π, M* = 2(1−2ν)(1+ν), P*, Q* depth-referenced at H = `rim_thickness`, default the rim ISO 6336-3 first calls thick) | T21.13's tooth model (its `Compliance` load share reads this) |
| F6 | `solve.rs` | `Station`, `Line`, `State { approach, lines, torques, loss }` | the coupled lines: stations on the valley, K = tooth in series + kernel + images, active set, the approach in closed form per iterate, the strip-width fixed point; friction per panel by the exact integral of Coulomb's direction (`panel_slide`), with Carter's creep as the named option that removes the spur reversal; the seed split at the zero of sliding; y0 the valley's true minimum | the fast mode's load share and efficiency inside the field (T06.5, T06.8 keep their fast-mode fixes) |
| F7 | `phase.rs` | `Piece`, `FieldReport` | breakpoints (signature scan, bisection), Gauss means per piece, maxima by golden section; ISO's points as phase roots | — |
| F8 | wiring | `Rating`, `MeshReport.field`, strings ×5, `field_rate`, `gear-cli field` | the two modes (§5.1) | — |
| F9 | `gap.rs` (optional flank) | `Flank::{Involute, Matched(Hob)}` | the matched (hobbed) worm wheel as a second flank kind, a closest-point projection; **implemented, not exposed** (plan §5, 2026-10-01) | — |

Each module is written once and serves every mesh: spur, helical, crossed, worm and ring are parameters
(signed z, Σ), not branches.

### 5.3 Order and gates (each step lands only when its laws fail at the base and pass at the head)

| Step | Laws that gate it (in the Rust suite unless marked) |
|---|---|
| F1 | C¹ at every joint (value and slope within 1e-12); curvature exactly 1/r_e on the round; the foot map exact; a C⁰ form refused |
| F2 | the gap equals `tools/contact_oracle.py` (the independent exact rigid mesh, ISO 21771 play and a₀) to 1e-12 mm; \|∇d\| = 1; seed independence: displacing the seed along and across the valley moves nothing beyond 1e-8 (the verifier's a2 law); the verifier's 2.77e-10 mm case agrees to 8 digits; every loop capped and counted |
| F3 | equals Hertz off a step to 1e-12; equals an independent discretised half-plane BEM (`tools/bem2d.py`, the verifier's) within 1e-5 on the same section and within 0.15 % on the exact C¹ circle (by hand); p finite and continuous for every C¹ form |
| F4 | the line limit; the Hertz ellipse reproduced at O(N⁻²); an aligned-end line with images equals the infinite line; the width rule exact to 1e-10 |
| F5 | c′ against the independent FE table (`tools/fe_tooth.py`, by hand) within the stated band over s_R (§2.4: −9.2 … +7.2 %); against ISO 6336-1 c′_th within 1 % at the default rim; the ring's rack tooth finite |
| F6 | equilibrium to 1e-12 (the torque balance); complementarity (q ≥ 0, gap ≥ 0 at every station); `panel_slide` against a brute-force integral; **continuity: every input (a, x₁, b, b₁, β, Σ, T) moved by ±1e-9 at the phases where the zero of sliding crosses a panel centre and a panel boundary changes D and p by ≤ 1e-6 relative, and by 1e-7 and 1e-5 proportionally** (the verifier's a3 law, §2.1); the verifier's h20 case to 1e-8; no refused span, no non-converged fixed point, no lost valley on the preset grid |
| F7 | **the dense-scan law: 1000 phases per pitch on h20, s0, c1, c10; every suspect of the verifier's jump detector (a neighbour difference > 2e-4 and > 8× both adjacent ones) in D, p_max, p_flank, p_edge, F or L bisects to a change that shrinks with the interval** — the one stated exception, the spur pitch-point reversal, asserted as a one-sided jump of its closed-form size (or absent, with Carter's creep); analyse's maximum ≥ the dense grid's; means converged in the piece count |
| F8 | the fast mode bit-identical under the identity harness; the field's ISO-point reading within the documented table of ISO 6336-2 (§2.6); the Σ → 0 and β → 0 series smooth (§2.5 of round four as laws); `check_wasm.sh` probes `field_rate`; strings fire both ways |
| F9 | play j = s_h/(cos α_n cos γ) for ZI exactly; the rigid gap on conjugate lines ≤ 1e-12 mm; the projection never stalls or fails on the worm grid, and reaches its cap only on the continuation's bound (outside the field) |

### 5.4 What it subsumes from the audit

- **As the reference its proofs need** (the task stays in the fast mode; its law reads the field):
  T06.5 (the load share conserves load: the field's coupled share is the reference), T06.8 (efficiency
  weighted by load per unit line length: the field's per-panel loss is the "discrete contact-line
  simulation" the task asks for), T06.9 (first order in μ: the field's balance is exact in μ), T06.10
  (the line length a pressure used: the field's load-weighted length), T08.3's σ_H comparison (§2.6),
  T08.11 (the finite-line-contact instrument: F4 is it), T21.9 (specific sliding from one expression:
  F6's panel sliding).
- **Replaced in the expensive mode**: T07.19 (one single-pair point: the field has none, it reads the
  maximum), T08.12 and T07.2/4/5/15's crossed face (the field's face is a clip on the valley, continuous
  in Σ — plan redesign **X**'s interval model is the fast mode's), strength#6 (Z_ε at B: the field reads
  the load it carries), T21.13 (compliance load sharing: F5 + F6), T21.11 (lead crowning: an end relief
  or a lead form on F1).
- **Spike S-C** (plan §3): closed by this plan — the unified model is the field; `Path`'s two
  implementations stay as the fast mode.
