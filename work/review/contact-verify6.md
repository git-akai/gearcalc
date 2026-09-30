# contact-verify6: adversarial check of work/contact-model.md round five (and its §5 Rust plan)

Author: contact-verify6, 2026-09-29. Read-only on the repo, no cargo. Every process capped at 1.8 GB
(`RLIMIT_AS`), pools ≤ 4, every loop capped. Scripts and raw outputs: `~/.cache/gearcalc-work/contact-verify6/`.
Own code: `jumps6.py` (the verify5 detector plus a two-step variant), `iso6.py` (ISO 6336-2 from the standard,
no shared code), the sliding-zero locator in `t_pert6.py`, the planted-tangency locator in `t_tan6.py`, the
dense-max check in `t_strip6d.py`. Everything else runs the subject's round-five code as-is (evidence about it).

## Verdicts

| # | Claim | Verdict |
|---|---|---|
| 1 | Friction per panel makes the state continuous; one jump left (spur pitch point); y0 fix; Carter | **CONFIRMED** (continuity, y0 fix, spur closed form); Carter **PARTLY** |
| 2 | The report (region maxima, edge share, load-weighted L) is continuous | **CONFIRMED in the field; PARTLY in the implementation** (the region-max search can jump) |
| 3 | Face ends: mirror high by ~1.5 % (Guilbault bracket); keep the mirror and flag | Source **CONFIRMED**; the "bracket" is a one-point calibration, not a bound; recommendation defensible |
| 4 | Rim read from the gear; defaults 1.2 h_t / 3.5 m_n; −9.2…+7.2 % vs FE; C_M | Arithmetic **CONFIRMED**; "derived" **REFUTED** (ISO Y_B thresholds, repurposed); default is non-conservative |
| 5 | Matched worm: 0 failures in ~9 M; −33–34 % like for like | Failures **CONFIRMED**; −17 % was cross-field (subject right); **−33–34 % is inflated: −30.6 %** |
| 6 | ISO points −14.5…+11.9 % of σ_H on externals | **CONFIRMED** (two recomputed independently) |
| 7 | §5 Rust plan coherent; gates | **PARTLY**: coherent in outline; one stale dependency, name collisions, unstated rating reading, missing gates |

## New finding (most important): the trace's CAP silently truncates loaded lines

`trace.py`'s `CAP = 0.03` mm stops a valley's trace where its gap exceeds the seed's by 30 µm. Where the approach
D exceeds that and the line is not clipped by a face, the loaded line is cut while carrying load, tagged `cap`,
and **the `touch` guard does not see it** (it counts only `gap` ends). `t_cap6.py`, `t_worm6.py`, `t_scale.py`:

- **Involute worm wheel** (ZI 1/40, r_e 0.1, 2 N·m, D ≈ 75 µm): 142 loaded `cap` ends in 24 states, end loads
  0.17–0.80 of the line's peak. Raising CAP to 0.3 mm (only change; 1.0 identical) moves analyse4's field max
  **9590.3 → 9094.0 (−5.2 %)**, L 4.30 → 5.23 mm. Matched wheel 6323.4 → 6312.3 (−0.2 %). η unchanged (67.87 / 67.01).
- **Module homogeneity broken**: h20 at 20 N·m scaled ×10 (m 10, b 100, r_e 1, T×1000) reads p_max +1.5/+6.6/
  +1.0/+1.1/+0.2 % at five phases, D +1.7 %, loaded `cap` ends at 0.55–0.68 of peak; with CAP 0.3 it equals
  m 1 to 1e-9. ×0.1 is homogeneous (1e-9). The crate already has this law (`train/homogeneity.rs`, s = 0.1, 10).
- Unaffected: the ISO rows (D 8–41 µm, lines face-clipped: no loaded `cap` end at five phases each), h20, s0, c1,
  c10 scans (no `cap` in 4000 rows).
- Cure: trace until the gap exceeds the span Dmax (derived, iterate if an end is `cap`), never a mm constant;
  count a loaded `cap` end as a refusal like `touch`.

## 1. Friction continuity (`scan6.py` → `s6_*.jsonl`, `jumps6.txt`; `t_pert6.py` → `pert6.txt`; `zoom6.py`)

- **Own dense scans** (1000 phases on an offset grid (i + 0.3183)/1000, h20/c1/c10/s0): the verify5 detector
  flags only s0 at 0.7493 (D −4.09 %, p_max −2.19 %, F −4.27 %: the stated reversal) and one h20 p_flank suspect
  (0.4573) that bisects to a change shrinking with the width (4.2e-4 at 1.2e-4 p … 1.7e-9 at 1.2e-10 p). A
  two-step variant adds kinks only (h20 Fedge 0.506, c1 p_max 0.556 / p_flank 0.564: all shrink ∝ width).
  0 errors, 0 touch/nonconv/lost.
- **The subject's own s0 scan cannot see its stated jump**: its grid lands exactly on 0.750 (F 2522.26, between
  the one-sided 2559.8 and 2450.4), splitting the jump into two adjacent steps, so the single-step detector
  (F7's law as written) is silent; the two-step detector finds it (4.34e-2). F7 must use an offset grid and a
  two-step test, or assert the jump by bisection.
- **Own planted phases on h20** (not the subject's four): the zero of sliding exactly on panel boundary 19 of
  line −1 (0.41317 p), at panel centre 18.5 (0.44007 p), a pair starting to carry load (2 → 3 lines, 0.42470 p),
  a line end changing zone (0.45706 p). Every input a, x₁, b, b₁, β, Σ, T, **μ, r_e and the phase** at ±1e-9 moves
  D, p_max, p_flank, p_edge, F_edge, L by ≤ 2.8e-7 (D's rigid-approach sensitivity), proportional through 1e-5.
  **No step.**
- **y0 fix is right**: y0 is the true minimum (ends + nodes + golden), so `loaded` returns None only when the
  whole line's gap ≥ D (it carries nothing). Fine scan of 201 phases over [0.9968, 0.9988] p (the subject's
  case): n = 3 throughout, no suspect; the planted 2 → 3 entry is proportional. (Residual: golden in the best
  node's bracket can miss a narrower, deeper dip between other nodes; its load would be ~0.)
- **Spur reversal: physical for Coulomb, closed form exact.** Pinion driving: F = T/(r_b ∓ μρ) before/after the
  pitch point; (7.9874 − 0.06·2.908)/(7.9874 + 0.06·2.908) − 1 = −4.28 %, p ∝ √F: −2.16 %; single-pair zone at
  0.75 p (F values match T/(r_b ∓ μρ) to 0.03 %). Exactly at 0.75 the state is neither limit (2522 N: rounding
  of A ≈ 0 flips some panels).
- **Carter**: Johnson's creep law Q/μP = 1 − (1 − |ξ|/ξ*)², ξ* = μa/R, checks; with ξ = s/R near the pitch
  point the window is |s| < μa = 4.65 µm (a 0.0775 mm), 3.15e-3 of a pitch. It needs no new property (μ, E, ν
  through a, R, kinematics) and is C¹ at ξ*. But it is **dry micro-slip** physics: oil-lubricated traction near
  pure rolling is set by the lubricant (typically a wider transition), it is exact only for a Hertzian strip of
  quasi-identical materials (not a round's strip, not steel/bronze), and it makes friction depend on q (through
  a), so "exact/linear in μ" (T06.9) no longer holds. Name it "Carter (dry creep)", not "the physical transition".

## 2. Continuous report (`t_tan6.py` → `tan6_h20T20.txt`; `t_strip6*.py` → `strip6*.txt`)

- Field level: three planted phases where a round's tangency line leaves the strip of a heavily loaded station
  (h20, 20 N·m): phase, T, r_e, a, β at ±1e-9 change p_flank, p_edge, F_edge, p_max, L by ≤ 6e-9, proportional.
- Unit level (strip_report swept 4001 points through tangency, round end, relief end, load, r_e 0.02, small k0,
  an emptying flank): no jump, except **one real jump of the implementation**: with both members' rounds inside
  one strip, p_max/p_edge jumps **2.35 %** across 8e-17 in the round position (dense max 3067.8 on both sides;
  the report reads 2997.3 on one): the 48-node fixed grid + golden on the two best sampled maxima picks the lower
  of two peaks 1.5 µm apart in one bracket. Random realistic strips (265): one under-read, 1.3e-4.
  Cure: search per piece between the section's steps (as `load_on` already integrates).

## 3. Face ends
- Source real: Guilbault, "A fast correction for traction-free surface of elastic quarter-space", WIT Trans. Eng.
  Sci. vol. 66 (Tribology and Design 2010), pp. 37–48, doi 10.2495/TD100041. Abstract: max edge displacement
  error −21.90 % (ν 0.3) with the mirror (shear correction), −9.55 % with ψ; −11.30 % → +0.60 % at ν 0.15.
  The doc's figures match (it omits the +0.6 %).
- The ψ 1.561 image matches that one figure (edge displacement, for Guilbault's imposed "fluctuating pressure") at
  the face plane; a uniformly stronger image also softens inboard. It is a calibrated estimate; only the sign is
  proven (minimum energy). −1.44 % is the right order, not a bound.
- "Keep the mirror and flag": defensible (conservative sign, ~1.5 %, nil mid-face), but face-end maxima are
  common (two of three unrelieved ISO helical rows), and §5.3 has no gate or Note key for the flag.

## 4. Rim (`t_rim6.py` → `rim6_*.txt`; `rim5.txt`)
- Band arithmetic checks (derived/FE-local −8.6, −0.5, +7.2, −9.2 %); against the full FE it is −9.0 … +42.4 %.
- The defaults are ISO 6336-3's Y_B thresholds (the least rim not de-rated in bending): a sourced rule of thumb,
  not a stiffness derivation. "What the crate rates an unset rim at" is loose: the crate's Y_B = 1 covers every
  s_R ≥ 1.2 h_t including a solid gear, and its doc says the rim never reaches contact (train/mod.rs:1302).
- Semantics: the field reads s_R as depth to a **rigid** support, so a thinner rim is **stiffer** (FE 17.57 at
  2.7 mm vs 13.41 local at 5.4/15.2); a real thin rim on a web is softer (ISO 6336-1 C_R). The default is the
  stiffest body ISO calls thick.
- Effect (h20, 20 N·m, 24 phases, default vs solid s_R = r_f): unrelieved p_max 5944.6 vs 6055.0 (−1.8 %);
  **tip relief 5 µm: 4684.5 vs 4920.4 (−4.8 %)**, edge share 0.019 vs 0.023. Same sign as C_M: not conservative.
  It must be a named, visible option (source stated) with a Note when used.
- C_M: the +6 % relieved figure scales all of ISO's measured deficit onto the teeth; part of it (shaft, bearings,
  body wind-up) is common to all pairs and moves no load, so +6 % is an upper bound, not a size.

## 5. Matched worm (`t_worm6.py`, `t_worm6b.py`)
- Own 24-state run: 426 479 projections, 0 stalled/capped/failed, 848 far (0.2 %), 7715 edge: consistent.
- verify5's −17 % compared the matched wheel on round three's field with the involute wheel on round four's:
  the subject is right. On one field (round five as shipped) −34.1 %; with untruncated lines (CAP 0.3)
  **6312.3 / 9094.0 = −30.6 %** (r_e 0.1, worm 2 N·m). The other three rows were not rerun; expect ~3–4 points less.

## 6. ISO (`iso6.py` → `iso6.txt`)
- FZG-C 16/24: my σ_H 1522.6 / 1629.5 / 1522.6 (ε_α 1.4624, Z_ε 0.91970, M1 1.0702) = the subject's. C lies in the
  single-pair zone (ρ_B 10.44 ≤ ρ_C 13.97 ≤ ρ_D 17.58): single-pair Hertz at C = 1655.6 = 1/Z_ε (+8.73 %) = the
  field's 1655.6.
- 17/43 β20 m2: σ_H0 961.3 (ε_β 1.0887, Z_ε 0.81872, Z_β 1.03159) = the subject's. A rigid uniform line load at
  the phase (L 32.38 mm) reads C −4.4 %, B +8.9 %, D −7.6 %; the field is 3–6 % above it (+1.7 / +11.9 / −5.0 %),
  the expected sign for a compliance-weighted share. Range −14.5 … +11.9 % confirmed from iso4.txt.

## 7. §5 Rust plan
Coherent with the two modes, redesign X and the 10-01 rulings, but:
- F2 builds `FieldMember` "from `BuiltMember`", which redesign G deletes, and S-C runs after G.
- Name collisions: `Rating` exists (train/mod.rs:2961), `Piece` (train/edits.rs:111), `Section` (tooth.rs:57);
  `field/solve.rs` beside the root-finder `solve.rs`.
- Unstated: which figure a Field-mode rating (S_H) reads (field max is 2.7–4× σ_H unrelieved); the 09-30 ruling's
  "final optimisation run" against "the search never calls the field".
- C_M is a measured mesh correction: principle 10 puts it on the mesh, not "on the tooth"; the rim default is not
  a named option; `rim_thickness`'s doc must change.
- F5 "from tooth.rs" vs "the ring's rack tooth": the band was measured on stiff.py's rack-cut teeth.
- T08.11 "exactly": its instrument is an independent DC-FFT with a face-truncated domain, not the mirror field.
- Missing gates: the crate's homogeneity law on the field (fails today via CAP); no loaded line end on a trace
  stop; F7 on an offset grid with a two-step detector; region maxima equal a per-piece/dense max to 1e-9;
  convergence in N, dz and the θ grid; every constant derived or shown immaterial (CAP, EXT 0.2 mm, LAM 1.5,
  1 mm hob continuation, 1.6 mm bracket, 25 × 25 table; principles 4 and 11); the face-end Note fires; a relieved
  design reported at C_M 1 and 0.8.
