# contact-verify7: adversarial check of work/contact-model.md round six, its oracle and the §5 porting plan

Author: contact-verify7, 2026-09-29/30. Read-only on the repo and on `contact-proto/` (the oracle generator was
copied with its output redirected); no cargo. Every process capped at 1.8 GB, pools of 4 or fewer, every loop capped.
Scripts and raw outputs: `~/.cache/gearcalc-work/contact-verify7/` (`v7_*.py` → `*7*.txt`). Own code that shares
nothing with the prototype: `v7_unit.py` (DE/tanh-sinh quadratures, AGM, closed forms), `v7_state.py` (the spur's
closed form), `v7_bem.py`/`v7_creep2.py`/`v7_creep3.py` (contact-verify5's BEM), `v7_couple.py` (2×2 algebra
on the fork's FE numbers), `v7_carter.py`'s quadrature. The rest runs the subject's round-six code as evidence.

## Verdicts

| # | Claim | Verdict |
|---|---|---|
| 1 | The oracle: deterministic, correct, tolerances sound, the worm gap flagged | Deterministic and correct **CONFIRMED**; tolerances **PARTLY** (four record sets cannot be met by a correct port) |
| 2 | Whole valley, solution-derived span, NBEYOND 2, no mm constant, homogeneous | **CONFIRMED** for Coulomb; homogeneity **REFUTED under creep** (9.9e-6); the span's stated proof is false but the span is sufficient |
| 3 | Round five's single-phase maxima under-read to −33 %; √kz panels converge (N 24 within 0.25 %) | Finding **CONFIRMED** (1338.2 vs 1993.9); convergence **PARTLY** (tip rounds 0.1–0.33 %; a narrower member's face end −1.1 … −2.2 % at N 24); stale figures: yes, the ISO table's field-max column and "2.7–4× σ_H" |
| 4 | §2.3: every constant derived / converged / named | **PARTLY**: 4 of 5 spot checks hold; Chebyshev n 20 errs 3.2e-3 (not 4.4e-5) on the spur pinion near its base circle |
| 5 | Per-piece maxima = dense (5e-14); detector finds the spur reversal, nothing else | **CONFIRMED**, also on my own 300 strips (8e-14) and full-pitch h20 and c10 scans (every suspect a kink) |
| 6 | Creep exact curve; blank monotone, −12…−3 %; coupling 15–82 % | Curve **CONFIRMED** (independent BEM); implementation **PARTLY** (carter_slide cancels catastrophically on spur panels); coupling: significant, size and sign below |
| 7 | §5 sound as a port spec | **PARTLY**: order and wiring sound; P4 ⟂ P6 (the state oracle presumes stiff.py's tooth bit for bit), four gates contradict records, refine's objective unconverged at face ends |

## 1. The oracle (`contact-proto/oracle/`, 106 records)

**Deterministic** (`v7_det.py` → `det7_a.txt`). Rerunning `make_oracle.py` (redirected copy) reproduces all 104
non-phase records **bit for bit** (form 20, kernel 5, across 44, compliance 7, gap 5, state 22, matched 1); the source
md5s in `index.json` equal the files on disk. The two phase records (whole analyses, 50 min) are bit-identical too: **106 of 106**. Every state record rebuilt from its own `inputs` alone
(nominal included, no `set_nominal`, fresh object) is bit-identical (21 of 21, `tol7.txt`): no hidden state.

**Correct where something independent can say so.**
- Carlson R_F, R_D against double-exponential quadrature of their integrals: 1.6e-15. G′ (R_D closed form) against a
  substituted trapezoid of its integral: ≤ 3e-15. G_direct 2.9e-13; the G table 7.9e-11 (tol 1e-10). shape_C against
  Hertz's ellipse via my AGM: 1e-14; aspect consistent in q to 1e-16. line_limit, Hertz strips, member circles: exact.
- 8 across strips against contact-verify5's BEM (M 2000/4000/8000, Richardson): ≤ 4e-7, the two-round strip 4.8e-6.
- Gap records: in-action valleys at a_par read g = 1e-15 (zero-backlash conjugacy) on s0, h20, ring.
- States: torque equilibrium 8.5e-15 on all 22; the spur's one-sided Coulomb states equal F = T/(r_b ∓ μρ₁) to
  2.2e-16 with ρ₁ differing by exactly the roll between the phases (1e-11), so the pitch point is at 0.75 p; the creep
  state at 0.75 gives F = T/r_b to 4e-15; the ×10/×0.1 records sit ≤ 0.05 of their tolerance from the base.

**Tolerances: sound for iteration, not for interpolants.** Tightening the width fixed point (1e-7 → 1e-10) and the
span (1e-10 → 1e-13) moves every output ≤ 0.66× its tolerance (F on s0/0.3183), except `loss` on the creep state
(1.4×) (`tol7.txt`). But four record sets presume the Python's arithmetic, not its model:
1. **The tooth compliance interpolant** (`interp7.txt`, fresh process per variant): the Chebyshev order 20 → 40
   moves s0/0.62 by **1.9e5× its tolerance** (D 7.7e-5, p_max 4.1e-4), s0/0.74975 6e3×, h20T20 248×, h20 175×,
   c10 81×, ring 1.8×. Cause (`cheb7.txt`): on the spur pinion (form circle = base circle) the n 20 interpolant
   errs **3.2e-3** at u/H 0.997, against the claimed 4.4e-5 (the helical pinion's figure); the compliance record's 9
   radii never sample there. A port that "evaluates directly" (README) or reads G's generator (§5.1) fails every
   state, gap-derived and phase record by orders of magnitude.
2. **kernel/panel**: 4 of 25 records are off the exact integral by 3e-9 … 1.9e-8 (tol 1e-9) — the G table's 8e-11
   amplified by the cancellation between surface and depth terms (surface/total −48 … −731). With G_direct: ≤ 1.2e-10.
   A port with "G by quadrature or series" (README) fails them.
3. **compliance/foundation L\***: computed as −288(1−ν²)·I1_numeric/π; off the stated closed form 18(1−ν²)/π by
   8.7e-8 (tol 1e-9, basis "closed form").
4. **The two creep states**: rounding noise of 1e-5 in F and D (§6), tolerance 1e-10.

Smaller: the across records' c tolerance (rel 1e-12, "brent tol 1e-14") is tighter than its own rule — the brent
tolerance is absolute (1e-14 mm), so for the smallest stepped strip (c 3.7 µm) ten times it is 2.7e-11 relative.
The G table's effect on states is negligible (≤ 0.045× tol with a 4× finer table). The G-table and L\* items are
trivial to regenerate; items 1 and 4 need a structural fix (§7).

**The worm gap.** Every oracle state is converged (nonconv 0, span_nonconv 0; the worm's final dq 4.3e-8), so the
port is held to no non-converged value. The README does not mention the gap; §4/P6 do. The "85 of 353 states"
counts `_coupled` calls (intermediate span iterates included), not states: on 48 regular phases of the same worm
(`worm7.txt`) no call and no final solve stopped at a cap. The residue is at special phases (a pair entering, the
documented ~5e-8 floor against a 1e-7 stop), and P6's gate will find it.

## 2. Whole valley, span, homogeneity

- **Loaded lines** (`heavy7.txt`): 16 states at 10–100× the oracle's loads (h20/h20r/s0 200 N·m, c1/c10 20, ring 600,
  worm 20 and 60, h30x 400, sp31 300, c20 20) and one light (h20 0.02 N·m): **no cut, no refusal, no lost end**; every
  loaded end is a face, root or gap; the worm at 60 N·m runs its lines to both wheel faces.
- **The span**: re-solving the same pairs on {g < D + w(D − dlo)}, w 0.5 and 2, loads **no station with g > D** in all
  16. But the stated proof ("every term ≥ 0") is false: the depth-referenced kernel is negative far out (−ν h²/r³),
  and `stats['uneg']` counts net-negative coupling at loaded stations in 9 of 16 (up to 76). Sufficient in practice,
  not by the argument given; P6 should keep the wide-span check as a gate.
- **mm constants**: none in the model. Solver floors remain absolute: brent tol 1e-14/1e-15 mm on contact2d's c and m
  and the trace's roots, the seed's 1e-11 mm, the anchor's h 1e-6/gradient 1e-8 (seed only). Harmless for m ≥ 0.01;
  §5.3's constants check should list them (× m_n in the port).
- **Homogeneity ×10/×0.1** on 11 cases outside round six's six (ring ×2, c1, s0bb, h30x 23/57 β30 x±0.25 with tip
  and end relief ×2, sp31 13/31 x0.4 bronze unequal faces ×2, c20 crossed 45°, worm wheel-driving): ≤ 2.5e-9 with
  Coulomb. **With creep (s0, 0.7495 p): 9.9e-6 at ×0.1, 7.0e-8 at ×10** — see §6.

## 3. Single-phase maxima

`conv7.txt`, `conv7b.txt` (p_max at N 24/48/96[/192], √kz panels, line reading; uniform N 96; round five's settings):

| State | round five N 24 | N 24 | N 48 | N 96 | N 192 |
|---|---|---|---|---|---|
| h20 2 N·m 0.1183 | **1338.2 (−32.9 %)** | 1988.4 (−0.27 %) | 1993.0 | 1993.9 | – |
| h20 2 N·m 0.3183 | 1642.2 (−17.5 %) | 1990.7 | 1989.3 | 1990.5 | – |
| h20 20 N·m 0.1183 | 5919.7 | 5939.7 (−0.10 %) | 5944.9 | 5945.9 | – |
| ring 60 N·m 0.62 | 3378.1 | 3382.1 (+0.07 %) | 3379.3 | 3379.6 | – |
| h30x 40 N·m 0.62 | 1941.8 (−6.1 %) | 2075.3 (+0.33 %) | 2069.3 | 2068.5 | – |
| s0bb 20 N·m 0.3183 (b 10/10.5) | – | 1886.6 | 1899.2 | 1904.2 | 1905.9 |
| sp31 30 N·m 0.62 (b 8/9) | 1890.9 | 1952.6 | 1977.7 | 1989.9 | 1994.4 |

- The finding reproduces (1338.2 against 1993.9). At tip-round peaks √kz panels hold N 24 within 0.1–0.33 % of N 96
  (the stated 0.25 % is exceeded twice, barely). **At the face end of the narrower member** (unequal faces, the
  image leaves the other body a square-ended punch) the peak converges algebraically: N 24 is −1.0 % (s0bb) and
  −2.2 % (sp31, extrapolated) low, N 48 −0.4/−1.0 %. P6's gate "N 24 vs 48 ≤ 0.3 %" fails there (−0.66 %, −1.27 %).
- The line reading is never below a 1601-point dense scan along the line (largest gap 0.0035 %).
- **Stale in the docs**: round six §3 carries round five §2.6's ISO table as "unaffected"; its ISO-point columns are
  mid-line, but its "Field max (where)" column (4400, 3769, 3423, 5667, 3409 MPa) was read with round five's panels
  and stations, and §5.2's "field maximum 2.7–4× σ_H" is drawn from it. Round five's face-end table (2010.8 …) and
  matched-wheel rows are listed for rerun in §5.4; the §2.6 field-max column and the 2.7–4× figure are not. The
  notch-research figures come from a 2-D mesh model (no lengthwise panels) and are unaffected.

## 4. Constants (§2.3), five spot checks

- NBEYOND 2: derived (the clip interval's Hermite tangents are central chords using X_{k+2}). Holds.
- KMAX = ⌈R/dz⌉ + NBEYOND + 1: derived (a valley point within 2 dz of the field is within the sphere + 2). Holds.
- set_nominal immaterial: nominal ×2, ×¼ on h30x, ring, worm, sp31, c1 moves every output ≤ 2.0e-9. Holds.
- KREF 2 = 24: bit-identical on the same five. Holds.
- **Chebyshev n 20 "4.4e-5 against direct evaluation": holds for the helical pinion (4.7e-5) and z 13 x 0.4 (4.3e-5),
  not for the spur pinion z 17 (3.2e-3 near the base circle; state p_max moves 4.1e-4).** Split the interval at the
  base circle or raise n where r_form = r_b.
- Also: the form's curvature "exactly 1/r_e on the round" (P1 gate) holds without relief; with tip relief the
  summed form's round reads κ r_e = 0.97–1.03 (all five relieved form records).

## 5. Maxima per piece and the detector

- Own generator (wider than the round's: k0 0.05–3, q 1–2000, 1–3 steps incl. overlapping rounds and reliefs,
  random flank interval), 300 strips, strip_report's region maxima against a 20 000-sample + golden reference
  (`strip7.txt`): flank −2.5e-14 … +5.8e-15, edge −1.4e-14 … +8.4e-14, no under-read beyond 1e-9.
- BEM agreement of contact2d's p on 8 oracle strips (§1).
- Detector: round six ran it only on s0 over 0.70–0.80 p (100 phases per grid); its claim is scoped to that window.
  My full-pitch scans on round six, 400 phases each on offset grids (`scan7_*.jsonl`, 0 errors, 0 cut, 0
  non-converged): **h20 2 N·m** on (i + 0.3183)/400: D, L, F silent; one single-step suspect (F_edge 0.823 p, 4.3e-4)
  and seven two-step suspects (p_max 0.535, p_flank 0.293, F_edge ×5); **c10** on (i + 0.7071)/400: one two-step
  suspect (L 0.289 p). Every one zoomed by 12 halvings (`zoom7_h20.txt`, `zoom7_c10.txt`) shrinks in proportion to
  the interval (e.g. 4.3e-4 → 1.3e-7 at 6e-7 p): kinks where a maximum or a pair's end changes region, no step.
  With the s0 result this confirms the law beyond the window: the one jump is the spur's Coulomb reversal.

## 6. Creep and the blank

**The exact curve is right** (`creep7b.txt`, `creep7c.txt`). For quasi-identical materials the corrective traction
on the stick zone [d, x_l] is a normal contact of the tilted profile μh − ξx. Loading contact-verify5's BEM (shares
no code) with the formula's Q\*(d) on μh − ξ(d)x puts its contact exactly on [d, x_l] (0.0 panels, 6 strips ×
2 directions × 3 stick sizes, Hertz and four round strips). Admissibility, which the round did not check: q\* ≤ 2μp
on the stick zone holds with margin (interior max q\*/2μp 0.05–0.42; leading-edge √ coefficients the same).

**The implementation is not** (`carter7.txt`). On a spur line the panel's sliding gradient B is rounding noise
(|B|/|A| 1e-13 … 1e-15), so `carter_slide` evaluates its window integrals at w = u − u0 ≈ ±1e15 and cancels
(c² − a², c³ − a³): the panel mean errs **4e-4 (×1), 1.7e-2 (×0.1)** against quadrature of its definition. That is
the 9.9e-6 homogeneity failure and the noise in the two creep records; `panel_slide` (Coulomb) is written without
the cancellation and is exact. Cure: integrate in u about the panel centre (|A + Bu| directly), or Taylor in B/A when
|B| ≪ |A|; add an `across/slide` record with |B| = 1e-14|A| and regenerate the creep states.

**The blank** (the fork's `t_rim6_fe.txt`, recomputed in `couple7.txt` with the contact compliance): arithmetic,
monotonicity and the −12.1 … −2.8 % band confirmed from the tables. **The missing pair-to-pair coupling is
significant today**, for two pairs at equal rigid gaps, reading each pair alone (as the field does, local, less the
common rotation):
- **Split**: the pinion-tip pair (A) gets +1.7 … +6.8 % more than its exact share, the pinion-root pair (B, the more
  loaded, 0.57) −1.2 … −5.1 %: p on B −0.6 … −2.6 %, **non-conservative at ISO's point B** (the rated σ_H,field
  reading on the pair near the pinion root); a tip-round peak on A over-read by up to +3.4 % (conservative).
- **Approach D**: −8 % (seat r_f/1.4) … −28 % (thin rim on a narrow web), in the double-pair zone. §4 lists only the
  split (+2…+5 %); the approach error, and its sign on the rated reading, should be stated.

## 7. The §5 plan as a port spec

Sound: the dependency order (form → kernel → across → compliance → gap → coupled → report → phase → wiring →
matched), the from_spec adapter's field list, the two modes, reading σ_H,field at ISO's points, refine staying inside
the fast mode's admissible set. Defects:
1. **P4 against P6/P8.** P4 builds the tooth from G's generator (and the ring from its own profile, a model change);
   the state, gap-dependent and phase records presume stiff.py's virtual-spur tooth, its rack-tooth ring and the n 20
   interpolant bit for bit (§1: up to 1.9e5× tolerance). Make the compliance an input: record each member's
   Chebyshev node values (c_t and h_d on [0, H]) and form radius in every state/phase record, have P5–P8 read them
   through a `ToothCompliance::from_nodes`, and gate P4 separately at the interpolant's true error. Mark the ring
   record as the rack tooth.
2. **Gates that contradict records**: P1 "κ = 1/r_e on the round" (relieved forms 0.97–1.03); P2's panel records
   (G table); P4's L\* (quadrature); P6's homogeneity with creep on (carter_slide) and "N 24 vs 48 ≤ 0.3 %" at
   unequal-face ends.
3. **refine** minimises the field maximum, which at an unequal-face end is 1–2 % low at N 24 and design-dependent;
   run it at N 48 and check the winner's convergence, or grade panels into face ends first.
4. Names: `Flank` exists (screw.rs:1394, private) — P10's `Flank::{Involute, Matched}` should be `FlankKind` or
   module-qualified. The rest of §5.1's list checks.
5. The P6 gate should add: the wider-span check (no load beyond D on {g < D + (D − dlo)}) in place of the false proof.

## Bottom line

The oracle is deterministic, matches everything independent I could put against it, and its iteration tolerances
are honest. It is not yet a spec for a correct port: the tooth-compliance interpolant (and the ring's rack tooth) must
be made an input or reproduced exactly, carter_slide fixed and its two states regenerated, and the panel/L\* records
regenerated from exact values. With those four changes (none touches the model) the port may proceed against it.
