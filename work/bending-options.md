# Bending options: every option's error against one named baseline (final table, 2026-09-30; three reviews applied)

For the owner's direction of 2026-10-03 (`plan.md` §5): item 1, every option's error against a named baseline; item 2,
the structured part of each error, with a call on whether it is substantial; the component model of item 3
(`bending-mechanics.md` §9); and item 4, the mode each option can run in. **This file chooses nothing,** and nothing in
the crate changed.
- **Marks:** [X] computed in the table round; [X, rev] a reviewer's run; [X, ed] the final editor's run; [R] read from
  the record or `bending-mechanics.md`; [C] recalled.
- **Scripts:** `~/.cache/gearcalc-work/bending-options/`. There `optrs/` rates every option at any load point and
  direction on the crate's own teeth, through the crate's own section searches (it reproduces `gear-cli fillet`'s DB
  and ISO, and `lww`'s LWW, to print precision). Beside it: `record_rows.py`, `sweep_models.py`, `path.py`,
  `direction.py`, `p2_direction.py`, `record_groups.py` and `reversals.py`. The editor's checks and copies of the
  reviewers' scripts are in `final/` (README there).

## The baseline

**The exact elastic fillet peak:** the largest boundary hoop stress σ_tt over the loaded side's **root land and
fillet** (`tools/fillet_bem.py`'s `root+` and `fillet+` elements), divided by F_t/(b m). On a traction-free boundary
σ_tt is the maximum principal stress.
- **The solve.** Plane strain; E drops out and ν = 0.3. `tools/fillet_bem.py`'s BEM (a direct Kelvin formulation with
  quadratic isoparametric elements, on three meshes, each finer at the last one's peak) solves it on the crate's own
  outline, as `gear-cli fillet … outline` prints it on the exact curves.
- **The load.** A cos² patch of half-width 0.05 m with resultant F_n = F_t / cos α_n. It sits at the crate's load
  point (HPSTC, or the tip where ε_α < 1) and acts along the crate's load direction (the involute normal).
- **The body.** Five teeth on a 10-module rim, with the far arc and the cuts held. At z 12 and 17 that rim would reach a
  shaft of 0.25 r_f, so there the whole gear is modelled on that shaft.
- **The record:** 228 teeth, in `tools/fillet_bem.txt`. Mesh convergence is 0.010 % at the median and 0.121 % at worst
  [R].
- **Cross-checks.** Two FEMs that share no code with the BEM agree within 0.10 % on 8 teeth and within 0.038 % on 12.
  A Trefftz solver agrees to 0.004–0.008 % on 3 teeth [R].
- **Known limits.** The ring body reads up to 0.55 % high (ring 40, five teeth against nine) [R]. A load at the tip
  corner needs the fillet read alone.
- **The domain, and why it is named.** At the record's load, three rules give the same peak on all 228 teeth
  [X, rev]:
  - the record's own rule (the whole loaded side, flank included);
  - P2's rule (the flank too, but not within 0.15 m of the load);
  - root land and fillet alone.

  Off the record's load they part [X, rev]:
  - **Turned ±6°:** P2's rule equals root+fillet to 0.00 %. The record's literal rule would move the peak to the edge
    of the patch on 71 of the 456 turned cases (median +9 %, max +33 %). That is the contact's own stress, not bending.
  - **Across loads raised up the tooth** (at the load point, near the tip, at the tip centre): P2's rule exceeds
    root+fillet on 11, 16 and 7 teeth, by up to 8.6, 11 and 76 %.
  - **The along load's compressive extreme** changes with the domain on 217 of 228 teeth, by up to 275 %. That is why
    no along-load figure appears here.

  The baseline is therefore root land and fillet. The load-position column (ᵃ) was read on P2's rule, and its agreement
  with root+fillet away from HPSTC is unchecked.
- **Off the record.** The sweeps, the path and the turned loads use the same BEM class on the same outline: a
  Flamant-subtracted fine mesh (P2 at level F), a point force and a 3-tooth sector.
  - Where both exist, it is within 0.45 % of the record, +0.03 % at the median [R].
  - Its body offset is **not** common to every option's comparison. Sector against whole body moves the peak −0.12 …
    −1.01 % over z 12–30 [X, rev], so the off-record teeth sweep carries up to ≈ 1 point of the reference's own z
    structure.
  - The reviewers' new sweeps (shift at a tight tool, notch at 14.5° and 25°, mate, α 25–31°, the rack) use the
    record's own solver and body instead (`fillet_bem.solve_tooth`, converged to ≤ 0.1 %).
  - The turned loads use the record's instrument, by superposition (note ᵇ).
- **What the reference can resolve.** A structure is a range, the difference of two errors, so the reference is good to
  **≈ ±1 point** on it: ±0.45 % off the record at each end, plus the body's ≤ 1 point in z.

## Figures, and one rule for every row

**Figures** are rated / baseline − 1, in %; **negative is unconservative**. A structured figure is the **range (max −
min) of that error across one design variable's span**, in points, with everything else held fixed. A sweep gives one
range. A bracket [median / max; k/N] summarises the record's groups of teeth that differ in that variable alone: k of
its N groups have a range ≥ 5.

A comparison of two designs reads (1 + e_A)/(1 + e_B) − 1 ≈ e_A − e_B. A uniform bias cancels exactly, so rule 2
concerns the range of e, not its median. The same rule is applied to every row:
- **S, substantial:** a range of **≥ 5 points anywhere it was measured**, in a sweep or in the worst record group.
  - The threshold is the size of one design step's true effect. Along these sweeps the exact peak moves 1.4 % per 0.1
    of shift, 2 % per degree of α, and 2–8 % per tooth at z 12–17.
  - So a 5-point structure reverses the ranking of designs 0.35 of shift or 2.5° apart, and misstates a full x or α
    sweep (≈ −21 %) by a quarter.
  - The threshold does not rest on the reference's level budget. The reference's ≈ ±1 point on a range sets the band
    below.
- **At the threshold:** a range of 4–6 points, within the reference's ±1 of the line. The call follows the figure as
  stated, but the reference cannot separate it from 5. Such calls are marked *(thr)*.
- **m, moderate:** 2–5 points, to be judged case by case. **n, negligible:** below 2 points.
- **Beside the maximum:**
  - The maximum is taken over sets of very different size and span: 85 two-member shift pairs, but teeth groups of 5–6
    spanning z 12–1000. The bracket's k/N therefore gives the fraction of groups at or above 5.
  - The comparisons table (Table 4) measures directly what rule 2 protects: every pair of record teeth, and how far
    each option misstates their ratio.
  - Table 5 gives the error's **direction** on each sweep: how much of the true effect the option credits.

## Table 1. What each option is, what it costs, and its constants

| Option | Mode | Cost at one load (wasm unless marked) | Fitted | Chosen (named) | Derived |
|---|---|---|---|---|---|
| **A. Dolan–Broghamer** (default today) | fast | µs | H, L, M (1942 photoelastic) | the Lewis parabola section; the axial term under the same K | — |
| **B. DB without its axial term** | fast | µs | as A | as A, less the axial term. Not a published model | — |
| **C. ISO 6336-3 Y_F·Y_S** | fast | µs | Y_S: Hirt's 1.2, 0.13, 1.21, 2.3 | the 30° tangent (60° on rings); h_Fe arm; q_s clamped at 8 | — |
| **C′. ISO, Y_S continued past q_s = 8** | fast | µs | as C, used past its range | as C, no clamp | — |
| **LWW, κ = 1** (P1) | fast | **9.4 µs per section** [R]; ≈ 1.5 ms under LinearRamp as built | none | c = 2x; κ = 1; the frame of the local symmetric wedge; the exponent 1 − λ(α_n) used as a power law | the wedge's Flamant and Carothers edge stresses; λ (Williams) |
| **LWW, κ = 0.910** | fast | as above | κ, on half the record (holdout worst −7.9 [R]) | as above | as above |
| **Neck components (hybrid)** ʲ | fast | **98–101 µs per section, 10.7× LWW in the same build** [X, ed]; the prototype's bisection 50× | none, but chosen among 13 variants on the record (so its record figures are in-sample) | reference point Q; the corner map's exponent 2(1 − λ) and pivot q0 = 1; notch length x (N, M) and c = 2x (V); κ = 1 (V) | Neuber's neck in N, V and M, fitted to each fillet point |
| **P2, fast BEM at L2** | **expensive only** (item 4: no cache in the live mode) | 32 ms per sector; 0.8 s for a whole z 17 | none | the body (3 teeth on a 10 m rim, or the whole gear); a 0.15 m window at the load; level L2 | the field equations |
| **D. Computed K_t** | **expensive only** | 0.1–1 s per member (the decision file's estimate) | none | any exact solver and its body | the field equations |

## Table 2. Level and spread against the baseline (rule 6: each bias's size and sign)

| Option | All 228: median (min … max) | IQR (width) | Share under | Ordinary 85 · middle 66 · tight 29 · rings 48 · undercut 41 (medians) |
|---|---|---|---|---|
| A. DB | −25.3 (−71.5 … +7.4) | −34.8 … −15.5 (19.3) | 96 % | −15.0 · −31.1 · −52.9 · −26.8 · −6.2 |
| B. DB, no axial | −10.5 (−65.5 … +15.4) | −23.4 … −2.9 (20.5) | 82 % | −2.3 · −18.0 · −46.4 · −10.8 · +3.7 |
| C. ISO | +6.0 (−57.5 … +31.1) | −2.5 … +17.7 (20.2) | 28 % | +6.4 · +2.4 · −29.9 · +26.3 · +2.0 |
| C′. ISO continued | +7.6 (−23.9 … +44.2) | +1.2 … +20.2 (19.1) | 21 % | +6.4 · +2.4 · −3.7 · +27.7 · +2.0 |
| LWW, κ = 1 | +10.1 (+0.3 … +19.1) | +7.0 … +12.3 (5.3) | 0 % | +9.6 · +13.0 · +12.3 · +5.8 · +8.7 |
| LWW, κ = 0.910 | +0.2 (−8.8 … +8.4) | −2.6 … +2.2 (4.8) | 48 % | −0.3 · +2.9 · +2.2 · −3.7 · −1.1 |
| Neck components | +3.1 (−0.7 … +14.4), in-sample | +2.0 … +5.5 (3.5) | 3 % | +2.0 · +5.4 · +5.5 · +3.5 · +1.5 |
| P2 at L2 | +0.1 (**−40.5** … +0.6) ⁱ; the 223 away from the tip corner −0.2 … +0.6 | 0.0 … +0.2 (0.2) | 21 % | +0.0 · +0.1 · +0.3 · +0.1 · +0.0 |
| D. Computed K_t | 0 by construction (the baseline's own solver); ≤ 0.6 against the true field | — | — | rings up to 0.55 high (the body) |

The neck model's median depends on a named pivot, q0 = 1 (ʲ): q0 = 0.25 … 4 reads −0.9 … +8.1 [X, rev]. It is a
choice as arbitrary as LWW's κ, not a gain of mechanics.

## Table 3. Structured errors, and the call

**3a. The load.**

| Option | Load position, LPSTC → HPSTC ᵃ | Load height ᵏ | Direction ±6° · ±14° ᵇ |
|---|---|---|---|
| A | 6.0 / 20.2 **S** | not measured | 3.8 / 9.1 · 10.4 / 23.5 **S** |
| B | 8.3 / 15.0 **S** | not measured | 5.8 / 11.1 · 14.3 / 28.4 **S** |
| C | 5.5 / 19.9 **S** | not measured | 9.4 / 13.8 · 23.2 / 36.1 **S** |
| C′ | 5.5 / 19.9 **S** | not measured | 10.1 / 20.8 · 25.1 / 53.8 **S** |
| LWW, κ = 1 | 3.8 / 9.6 **S** | 2.2 / 10.9 **S** | 1.3 / 4.0 *(m, thr)* · 3.4 / 10.7 **S** |
| LWW, κ = 0.910 | 3.5 / 8.7 **S** | 2.0 / 9.9 **S** | 1.2 / 3.6 *(m)* · 3.1 / 9.7 **S** |
| Neck components | 1.5 / 5.0 **S** *(thr; 5.01)* | 1.1 / 5.0 **S** *(thr; 5.05)* | 1.8 / 4.5 *(m, thr)* · 4.4 / 11.7 **S**; worse than LWW at every span, and on 160 of 228 teeth at ±6° |
| P2 | 0.0 / 0.2 n, but **S at the tip corner** (−30 … −40.5) ⁱ | not measured | 0.04 / 0.14 n (12 teeth) · not run |
| D | n | n | n |

**3b. The geometry.** The brackets are record groups [median / max; k of N groups ≥ 5].

| Option | Shift ᶜ: ρ_fP 0.25, 20° · 0.03, 20° · 0.03, 14.5° [85 pairs] | Teeth z ᵈ [30 groups] | Mate z ˡ: ρ_fP 0.25 · 0.03 · ring | α ᵉ: 14.5–25° · 25–31° [60 groups] | Notch ᶠ: sweep at 14.5° · 20° · 25° [35 groups]; c/ρ bins | Ring vs external ᵍ: step at ρ_fP 0.2 · 0.1 · 0.03 / class medians |
|---|---|---|---|---|---|---|
| A | 27.7 · 48.2 · 59.9 [10.3 / 27.7; 62/85] **S** | 20.0 [24.5 / 60.2; 30/30] **S** | 5.5 · 5.8 · 5.9 **S** *(thr)* | 16.5 · 22.1 [16.5 / 28.9; 60/60] **S** | 12.6 · 20.3 · 21.7 [21.7 / 45.3; 30/35]; 47.4 **S** | +13.2 · +19.7 · +28.6 / −11.8 **S** |
| B | 17.5 · 45.5 · 55.3 [7.6 / 25.6; 55/85] **S** | 19.6 [29.5 / 63.4; 30/30] **S** | 4.3 · 4.7 · 3.4 *(m, thr)* | 8.4 · 22.5 [12.8 / 25.1; 58/60] **S** | 15.0 · 24.9 · 27.1 [27.0 / 54.4; 33/35]; 48.1 **S** | +17.0 · +24.6 · +34.8 / −8.5 **S** |
| C | 30.5 · 25.5 · 24.3 [6.5 / 19.0; 49/85] **S** | 7.2 [13.5 / 62.6; 25/30] **S** | 6.4 · 4.6 · 0.2 **S** | 11.4 · 6.2 [8.0 / 17.6; 39/60] **S** | 18.3 · 20.8 · 23.1 [21.2 / 79.5; 31/35]; 33.9 **S** | +7.9 · +19.0 · +45.2 / +19.9 **S** |
| C′ | 30.5 · 29.8 · 30.0 [8.4 / 18.5; 56/85] **S** | 7.2 [18.1 / 30.7; 25/30] **S** | 6.4 · 4.6 · 0.2 **S** | 11.4 · 6.2 [10.0 / 18.1; 52/60] **S** | 18.3 · 20.8 · 23.1 [18.1 / 30.5; 31/35]; 12.1 **S** | +7.9 · +19.0 · +37.4 / +21.3 **S** |
| LWW, κ = 1 | 13.0 · 9.9 · 10.0 [2.6 / 7.2; 14/85] **S** | 7.0 [3.9 / 7.5; 11/30] **S** | 3.5 · 2.1 · 1.5 *(m)* | 0.6 · 4.9 [3.0 / 8.1; 18/60] **S** | 7.0 · 3.7 · 1.8 [4.9 / 8.7; 17/35]; 8.2 **S** | −7.1 · −7.2 · −5.1 / −3.7 **S** |
| LWW, κ = 0.910 | 11.8 · 9.0 · 9.1 [2.4 / 6.6; 9/85] **S** | 6.3 [3.5 / 6.8; 6/30] **S** | 3.2 · 1.9 · 1.3 *(m)* | 0.5 · 4.5 [2.7 / 7.4; 15/60] **S** | 6.4 · 3.4 · 1.6 [4.4 / 8.0; 13/35]; 7.5 **S** | −6.5 · −6.5 · −4.7 / −3.4 **S** |
| Neck components | 3.5 · 3.7 · **8.1** [0.7 / 10.6; 5/85] **S** | 3.4 [3.2 / 9.9; 9/30] **S** | 0.9 · 2.7 · 0.8 *(m)* | 1.2 · 3.3 [2.8 / 8.1; 16/60] **S** | **10.8** · 7.6 · 3.8 [5.1 / 12.1; 18/35]; 6.1 **S** | 0.0 · +0.7 · +0.7 / +1.5 **n** |
| P2 | 0.2 · — · — [0.1 / 0.3; 0/85] n | 0.4 [0.2 / 0.5; 0/30] n | not run | 0.5 · — [0.1 / 0.4; 0/60] n | — · 0.1 · — [0.2 / 0.5; 0/35]; 0.3 n | −0.2 / +0.1 n |

P2's sweep figures are L2 against its own level F, which measures self-convergence rather than error against the
baseline. Only its brackets and Table 2 are against the record.

**3c. The calls, one rule for every row** (S ≥ 5; *thr* 4–6; m 2–5; n < 2; "—" not measured):

| Option | Position | Height | Direction | Shift | Teeth | Mate | α | Notch | Ring | Substantial on |
|---|---|---|---|---|---|---|---|---|---|---|
| A | S | — | S | S | S | S *(thr)* | S | S | S | 8 of 8 measured |
| B | S | — | S | S | S | m *(thr)* | S | S | S | 7 of 8 |
| C, C′ | S | — | S | S | S | S | S | S | S | 8 of 8 |
| LWW, κ = 1 or 0.910 | S | S | S at ±14° (m at ±6°) | S | S | m | S (groups) | S | S | 8 of 9 |
| Neck components | S *(thr)* | S *(thr)* | S at ±14° (m at ±6°) | S (tight tool, 14.5°) | S (groups) | m | S (groups) | S | **n** | 7 of 9, two at the threshold |
| P2 | n; **S at the tip corner** | — | n (±6°, 12 teeth) | n | n | — | n | n | n | the tip corner only |
| D | n | n | n | n | n | n | n | n | n | none |

## Table 4. Comparisons: what rule 2 protects

Record pairs, without the five teeth loaded at the tip corner. The **misstatement** of a pair is |(1 + e_A)/(1 + e_B) −
1|, in points: how far the option misstates the ratio of the two designs' peaks.

| Option | Reversed, one-level pairs ʰ (of 417 with exact ≥ 5 %) | Reversed, all pairs (of 22,346 with exact ≥ 5 %) | Misstatement over all 24,753 pairs: median · p90 · p99 · max |
|---|---|---|---|
| A | 60 (14 %) | 3,649 (16.3 %) | 19.4 · 65.3 · 155.7 · 276.4 |
| B | 55 (13 %) | 3,259 (14.6 %) | 16.3 · 60.4 · 143.6 · 234.6 |
| C | 58 (14 %) | 2,444 (10.9 %) | 14.2 · 50.2 · 138.3 · 187.5 |
| C′ | 4 (1 %) | 2,003 (9.0 %) | 12.2 · 26.9 · 38.3 · 57.7 |
| LWW, κ = 1 or 0.910 (a scale cancels) | 0 | 66 (0.3 %) | 3.5 · 8.4 · 13.1 · 18.8 |
| Neck components | 0 | 30 (0.1 %) | 2.4 · 6.8 · 10.9 · 13.2 |
| P2 at L2 | 0 | 0 | 0.1 · 0.3 · 0.5 · 0.9 |

The neck model's worst reversal [X, rev] sets external z 12, 20°, x 0, ρ_fP 0.01 (exact 5.419, error +0.2) against
external z 17, 14.5°, x 0.5, ρ_fP 0.01 (exact 4.838, error +13.3). The exact gap is +12.0 %; the model says −1.0 %.

## Table 5. The error's direction: how much of each sweep's true effect an option credits

Model change / exact change between the sweep's ends, in %. 100 is exact; above 100 over-credits the design change;
below 0 has the wrong sign. This is where a structure turns into a bias for or against one kind of design.

| Option | Shift, ρ_fP 0.25 (exact −21.5 %) | Shift, ρ_fP 0.03, 20° (+47.3 %) | Teeth z 12 → 220 (−31.0 %) | Mate 12 → 1000 (−24.2 %) | α 14.5 → 25° (−20.9 %) | α 25 → 31° (+12.5 %) | Notch 0.01 → 0.38: 14.5° (−46.2) · 20° (−46.5) · 25° (−41.3) |
|---|---|---|---|---|---|---|---|
| A | 201 | **−58** | 145 | 119 | 164 | **−145** | 82 · 65 · 45 |
| B | 159 | **−38** | 140 | 112 | 130 | **−111** | 80 · 62 · 42 |
| C | **−17** | 32 | 90 | 119 | 55 | 63 | 74 · 72 · 62 |
| C′ | **−17** | 201 | 90 | 119 | 55 | 63 | 74 · 72 · 62 |
| LWW, either κ | 54 | 128 | 85 | 110 | 101 | 59 | 107 · 104 · 98 |
| Neck components | 87 | 104 | 92 | 98 | 104 | 71 | 111 · 108 · 105 |

Several sweeps are not monotone: shift, teeth, and α 25–31°. For those the ends understate what happens between
them. Shift at 14.5° with a tight tool is left out, because its exact peak falls 17 % and then rises 38 %.

## Notes

- ᵃ **Load position** [X]:
  - 11 teeth: external z 12, 17, 30 (four times), 60 and 1000; rings z 40, 60 and 100. The ε ≥ 2 tooth has no
    single-pair zone.
  - Five points on the involute normal, from LPSTC (1 p_b from the tip) to HPSTC (ε − 1 p_b). The figure is the range
    over them, as median / max over the teeth.
  - Exact: P2-F, read on P2's domain rule.
  - DB re-searches its parabola at each load. The crate's LinearRamp reading, with the HPSTC section held, gives
    6.1 / 22.0.
  - The neck model's 5.0 is 5.013, at z 30, 25°, ρ_fP 0.25: an ordinary fillet, so not a notch effect [X, rev].
  - **Outside the span**, where a ramp also reads:
    - At the lowest point: DB −8 … −80 (4 of the 11 unrated); ISO −12 … −51; LWW +0.5 … −31; the neck model −34 … −1
      (every wrench model shares the load's own near field); P2 within 0.3.
    - At 0.05 p_b from the tip: the closed forms move −10 … +13 points from their HPSTC value, and the neck model
      −3.4 … +2.4. P2's −0.5 … −21 there is L2 against its own F.
- ᵇ **Direction** [X]:
  - The load at the crate's point is turned ±6°, a friction angle at μ ≈ 0.1 (lubricated steel 0.04–0.1, dry polymers
    higher [C]); ±14° is μ ≈ 0.25.
  - Exact: the record's own instrument, superposing its unit across and along patch loads at the load point
    (`frame-critique/all.jsonl`). Superposition holds to 1.6e-8, and the sampled peak is within 0.05 % of the record.
    It moves ∓12–13 % per 6°.
  - The figure is the range over −δ, 0 and +δ, as median / max over the 228.
  - At ±10° [X, rev]: the neck model 3.0 / 7.7, LWW 2.3 / 7.0. The neck model's worst cases are middle and tight
    externals.
  - P2: 12 teeth, its half-plane kept on the flank's tangent.
  - With the load laid across alone: ISO reads −12 points at the median, LWW −1.7.
- ᶜ **Shift:**
  - External z 30, 20°, ρ_fP 0.25, mate 43, 23 shifts. The exact peak falls 21.5 %.
  - At ρ_fP 0.03 [X, rev; options rated X, ed]: x −0.375 … 1.0, 12 shifts, at 20° and at 14.5°. The x −0.5 point
    failed for memory.
  - At 14.5° the neck model reads +3.8 → +11.9. Of its 8.1 points, 7.8 lie between x −0.375 and 0.375, following
    undercut onset.
  - Brackets: the record's 85 external pairs x 0 → 0.5.
  - On ordinary fillets (ρ_fP ≥ 0.1) the pairs ≥ 5 are: neck 1 of 51, LWW 10 of 51. At ρ_fP ≤ 0.03: neck 4 of 34 with
    a worst of 10.6, LWW 4 of 34 with a worst of 7.2.
- ᵈ **Teeth:**
  - External z 12 … 220 (20°, x 0, ρ_fP 0.2, mate 17), 17 points. On z 12 … 40 at ρ_fP 0.25: DB 10.3, ISO 3.4,
    LWW 2.8, neck 2.6.
  - Brackets: the record's 30 groups over z 12 … 1000, of 5–6 teeth each. By fillet class:

    | Fillet | Groups ≥ 5: neck | Groups ≥ 5: LWW | Worst group: neck | Worst group: LWW |
    |---|---|---|---|---|
    | ρ_fP ≥ 0.1 | 2 of 18 | 8 of 18 | — | — |
    | ρ_fP ≤ 0.03 | 7 of 12 | 3 of 12 | 9.9 | 5.3 |

  - The neck model's (20°, x 0, ρ_fP 0.01) group runs +0.2 → +9.4 → +5.7 across z 12 → 60 → 1000; LWW's range on it
    is 2.7 [X, rev].
- ᵉ **Pressure angle:**
  - External z 30, 14.5 … 25°, 23 points. Then 25 → 31°, 5 points on the record's solver [X, rev]; at 32.5° the BEM
    failed (a division by zero). Over 25 → 31° the neck model drifts +1.9 → −1.4, toward unconservative.
  - Brackets: 60 record groups of 2–3 angles. Their worst sits at ρ_fP 0.01 for both LWW (z 150, x 0) and the neck
    model (z 60, x 0.5).
  - With ρ_fP ≥ 0.1: neck 5.5, LWW 5.4 (1 of 36 each).
- ᶠ **Notch:**
  - c/ρ is LWW's notch length over the local radius (c = 2x).
  - At 20°: ρ_fP 0.01 … 0.40 on external z 60, x 0, 23 points (c/ρ 26 → 4.9; exact −46.5 %; P2-F).
  - At 14.5° and 25°: ρ_fP 0.01 … 0.38 on the same tooth, 10 points [X, rev; options rated X, ed]. The record's own
    medians fall with α for the neck model: +5.4, +4.1, +2.5.
  - Brackets: 35 record groups of five tool radii; the record spans c/ρ 2.2 … 210.
  - The c/ρ-bin figure is the range of the external medians over five bins, from < 4 to > 32.
  - Below c/ρ 2, off the record (heavily undercut, tip-loaded z 8–12): LWW −9.6 … +1.1 [R]; the neck model untested.
- ᵍ **Ring vs external:**
  - The step across the rack along signed 1/z: external z 220 → ring z 2640, with the same α (20°), x and ρ_fP, mate
    17. At ρ_fP 0.2 on P2-F; at 0.1 and 0.03 on the record's solver [X, rev; options rated X, ed].
  - After the slash: the ring median less the ordinary-external median on the record. That is a difference of class
    medians, confounded by the other variables, not a comparison with everything else held fixed.
- ʰ **Reversed, one level:**
  - The record's pairs one level apart in one variable (z, α, x or ρ). Of these, 417 have exact peaks ≥ 5 % apart; the
    figure counts those the option ranks the wrong way round.
  - A: z 33, x 27. B: z 33, x 22. C: z 28, x 17, ρ 12. C′: x 3, α 1.
  - At tight fillets the exact peak **rises** from z 150 to 1000: by 7 % at ρ_fP 0.1, where A falls 3 %, and by 50 %
    at 0.01, where A rises 7 %.
  - Pairs one level apart are a small, easy subset. Table 4's all-pairs column is the one that tests comparisons of
    dissimilar designs.
- ⁱ **P2:**
  - Five teeth are loaded at the tip corner (z 12, 14.5°, x 0) and read −30 … −40.5. There is no corner particular
    solution yet.
  - Against an independent FEM: −0.08 … +0.44 % [R].
  - Its body is chosen: the record's rule, as above. A sector-only rule reads −0.1 … −1.0 % on pinions z 12–30 [R].
  - On rings, L2 and F differ by −9.8 … −20.9 % on the tip side, and F was verified near the corner on externals only.
    Every row's ring tip-side figure therefore rests on an unverified reference [X, rev].
- ʲ **Neck components** (`bending-mechanics.md` §9; scripts `~/.cache/gearcalc-work/bending-comp/`, cost in
  `bending-options/final/neckwasm`):
  - **What it is.** A hybrid, not a superposition.
    - The along force and the moment come from Neuber's deep hyperbolic notch. It is fitted to each fillet point's
      tangent and curvature and evaluated at a corner-mapped sharpness.
    - The across force is LWW's term. About the reference Q, that is Carothers' shear plus the moment V·x·cot a, × W.
    - The two come from different bodies, so the answer depends on where the wrench is split. At the exact peak
      station the median reads +2.9 about Q, +3.4 about the neck's centre and +7.3 about the wedge's apex; the
      ring-minus-ordinary step is +1.6, +0.8 and −6.3 [X, rev].
  - **Constants.** None is fitted. Chosen: Q; the map's exponent 2(1 − λ), an interpolation, since the record's teeth
    never reach the sharp regime it comes from; its pivot q0 = 1; N and M's notch length x; V's c = 2x; κ = 1.
  - **Which constant does what** [X, rev]:
    - q0 moves the median (0.25 … 4: −0.9 … +8.1) and hardly the structure (c/ρ bins 5.5–6.1).
    - c moves both. c = x reads −10.4 with notch 3.9, record ρ groups 7.9 and x groups 5.6; c = 4x reads +22.9 with
      notch 12.2.
  - **In-sample.** It was chosen among 13 structural variants scored on the record: neck, C, D, Va, Vc, Ve, Vn, E1,
    E2, G, DW, DW088 and QW [X, ed]. The record figures are therefore in-sample; the sweeps and path, run after the
    choice, and the reviewers' new sweeps are not. The worst figures match in and out of sample: 12.1 in sample
    against 10.8 and 8.1 out of it.
  - **Load height:** the across load at the load point, near the tip and at the tip centre, on the root+fillet domain:
    1.1 / 5.0 (LWW 2.2 / 10.9).
  - **Where the residual lives.** In V, LWW's term reused. At the exact peak it reads 1.10 of exact (0.95 … 1.31),
    rising with sharpness; M reads 0.97 and N 0.91. The model's median is V high cancelling M low. Replacing V alone
    by the exact response takes the ρ groups from 12.1 to 3.4 and direction from 4.5 to 1.6. Correcting M or N alone
    makes the structure worse [X, rev].
  - **Cost.** Measured in Rust and wasm32 on the 228 [X, ed]. It reproduces the Python prototype to 1e-6. The θ solve
    uses gear-core's Brent, ≈ 11 evaluations per station.
- ᵏ **Load height:** the range over the across load at the crate's load point, near the tip and at the tip centre, as
  median / max over the 228, on root+fillet. It was measured for LWW and the neck model only.
- ˡ **Mate z** [X, rev; options rated X, ed]:
  - External z 30, 20°, x 0, ρ_fP 0.25 and 0.03, mate 12 … 1000, 12 points.
  - Ring z 60, 20°, ρ_fP 0.2, mate 12 … 40, 8 points.

## What the table says (it does not choose)

- **Only P2 and D carry no substantial structure within the load span,** and both are expensive-mode options (item 4).
  P2 is still substantial at the tip corner (−40.5 %). Every tooth with ε < 1, and the far end of every ramp, reaches
  that corner.
- **Under one rule, every fast option carries substantial structure on most variables it was measured on:**

  | Option | Substantial on | Of those, at the threshold (5–6) | Moderate | Negligible |
  |---|---|---|---|---|
  | A | all 8 measured | mate | — | — |
  | B | 7 of 8 | — | mate (4.3–4.7, at the threshold) | — |
  | C, C′ | all 8 measured | — | — | — |
  | LWW, either κ | 8 of 9 | — | mate | — |
  | Neck components | 7 of 9 | load position, load height | mate | ring |

  Direction counts as substantial for LWW and the neck model at ±14°. At ±6° they read 4.0 and 4.5, which is moderate
  and at the threshold.

  The fast options differ in how often and by how much.
  - **Per pair (Table 4):** p90 misstatement 6.8 (neck), 8.4 (LWW), 26.9 (C′), 50–65 (A–C). Reversals out of 22,346:
    30, 66, 2,003, and 2,444–3,649.
  - **Per record group:** A–C′ reach ≥ 5 in 58–100 % of external groups per variable. LWW reaches it in 16–49 %,
    spread over ordinary and tight fillets. The neck model reaches it in 6–51 %, most of it at ρ_fP ≤ 0.03.
- **What splitting the load into components did:**
  - **Removed** the ring structure (0.0–0.7 against LWW's 5–7).
  - **Reduced** most of the ordinary-fillet structure: the record's groups ≥ 5 at ρ_fP ≥ 0.1 fell to z 2/18 and
    x 1/51, against LWW's 8/18 and 10/51. It also reduced, but did not remove, load position and load height (9.6 →
    5.0, 10.9 → 5.0).
  - **Kept or increased** the structure at tight tools and 14.5°:
    - shift 8.1 at 14.5°;
    - notch 10.8 · 7.6 · 3.8 against LWW's 7.0 · 3.7 · 1.8, which is 1.5–2.2× LWW at every angle;
    - record groups x 10.6 and z 9.9 against LWW's 7.2 and 5.3 at ρ_fP ≤ 0.03.
  - **Worsened direction** at every span.

  Its residual sits in the V term, which is LWW's own (ʲ, `bending-mechanics.md` §9.4).
- **The largest structures follow the notch and the load mix:** shift, load height, α and c/ρ. The neck model's worst
  z, x and α groups all sit at ρ_fP 0.01. LWW's worst α and x groups do too, but its worst z group is at ρ_fP 0.1.
- **What the constants do:**
  - **κ and q0 move the median, not the structure.** LWW's κ = 0.910 multiplies every range by 0.91, and q0 barely
    moves the neck model's. Rule 2 therefore does not depend on them; rule 6 (a bias's size and sign) does.
  - **c moves the structure.** V's notch length is a structural choice in both LWW and the neck model.
- **Cost:**
  - LWW runs at 9.4 µs per section at one load in wasm, and ≈ 1.5 ms under the ramp as built.
  - The neck model runs at ≈ 100 µs, measured. A ramp could reuse each station's three responses; that is not built.
  - P2 runs at 32 ms per sector and 0.8 s per small pinion. It is expensive only.
