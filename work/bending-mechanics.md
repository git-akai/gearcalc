# Bending from mechanics: can the fast mode be frame-free? (research round, 2026-09-30; three reviews applied)

For the owner's question on Q10 (`work/decision-bending-default.md`): **is there a more generic fast model of the root
bending peak, built on mechanics, with no chosen frame?** Research only: it chooses nothing and says nothing further on
option A. Nothing in the crate changed.

Three research angles (analytic elasticity, reduced-order numerics, the frame critique) led to two prototypes (§6),
checked by three reviews: an independent FEM with an LWW rewrite, a critique of frames and constants, and an audit of
cost inside the crate. Their justified corrections are applied; what they left unresolved is §7. The pre-review text,
with the full pre-registration, is `~/.cache/gearcalc-work/bending-mech/bending-mechanics.pre-review.md`. The closed
form's prefactor is written κ (A in the scripts), apart from the decision file's options.
- **Marks:** [R] read · [A] abstract · [M] metadata · [C] recalled · [X] computed ([X, rev] a reviewer's run, [X, ed]
  re-run by the editor); the reviews and this synthesis read no paper. **Figures:** rated / exact − 1 in %, against
  `tools/fillet_bem.txt` (228 teeth); **negative is unconservative**; classes as `docs/state.md` (ordinary ρ_f/s_Fn ≥
  0.1, tight < 0.02). **Scripts:** `~/.cache/gearcalc-work/` (`bending-mech/`, `bending-mechanics/`, `frame-critique/`,
  `rom-research/`); the reviews' in the session scratchpad (`rev/`, `rev2/`, `rev3/`, `ed/`), which may not persist.

**Short answer.** The exact peak is frame-free (§1), and a fast model can keep its covariance and continuity. No closed
form built here is free of every chosen constant, and the exact solve costs milliseconds, not microseconds.
1. **Closed form, µs: LWW** (local tangent wedge × Williams, §6.2): covariant, continuous on the crate's domain, free of
   E and ν, built from exact wedge solutions; but its section family and load split come from the tooth's local
   symmetric wedge, its notch length (c = 2x) and prefactor (κ = 1) are chosen, and its exponent is the tool's α_n.
   On the record +10.1 % median (+0.3 … +19.1), none under, spread sd ln 0.036 (DB 0.237, ISO 0.208, ISO continued
   0.126). The margin is the record's domain, not the model's: c = x reads −17 %, all under; a load low on the path
   −28 to −31 %; the along load −13 %; c/ρ < 2 down to −9.6 %. 9.4 µs wasm per section at one load.
2. **The exact solve, made fast: P2** (Flamant-subtracted BEM, fixed topology, §6.3): frame-free, no fitted constant, a
   chosen body; at L2 ≤ 0.63 % from the record and ≤ 0.44 % from an independent FEM, but −40 % at the tip corner. 32 ms
   per sector in the shipped wasm build, 0.8 s per small pinion's whole body: under rule 3 and the main-thread design
   as they stand, option D made cheaper, not a live figure.

Every published closed form met here is a fit (Heywood, Aida–Terauchi's design formula, Dolan–Broghamer, ISO's Y_S);
the one exact analytic route, complex potentials on a conformal map, is exact only on the map's image of the tooth.

## 1. The peak, stated without a frame

**The quantity.** A traction-free boundary carries one in-plane stress, the hoop stress σ_tt(s), the maximum principal
stress there. The rating is `peak = max_s σ_tt(s) / (F_t/(b m))` over the loaded side's root land, fillet and lower
flank: no section, tangent angle, parabola or split, and continuous in every input even where its location jumps.
σ_tt(s) = ∫ G(s, s′)·t(s′) ds′, with G one adjoint solve per station (Maxwell–Betti); for a small patch it is g(s; p)·F,
a covariant influence function of the load point. E drops out exactly [X]; ν enters only through held boundaries
(Michell [C]), ≤ 0.28 % per 0.05 on the truncated body [X].

**The reductions a fast model makes, and their cost on the record [X]:**
1. **Patch to point force** ≤ 0.04 %. **Point force to wrench** (Saint-Venant) 2.3 % median, 4.0 % worst at HPSTC,
   growing as the load nears the fillet (along 17/43's path the peak falls to 2.729 at r 8.17, then rises to 2.914 at
   r 8.00); unmeasured at the lowest point on z 1000 or ring 60 (§7).
2. **Wrench to one moment with one K.** The across force adds ≈ 35–37 % of the bending nominal on ordinary fillets; the
   along force is concentrated 29 % less than the across (ordinary), 32–34 % (middle, tight), 44 % (rings). One K on
   (Y_F − axial) underrates by 11.5 % median on ordinary external fillets, 18.3 % on rings: the frame round's split of
   DB's −15.0 % is K_f's fit (−3.3 %) × this structure (−11.5 %) [X, frame round].
3. **Wrench to a conventional section.** Where it sits costs little (Y_F on the peak's chord is 0.95–1.08 of Y_F at
   30°); the radius read there costs a lot (ρ_F at 30° is 1.00–4.53× the fillet's minimum, 1.00–2.28× at the peak). The
   peak's tangent sits 19–62° from the centreline on external teeth (median 46°), 52–69° on rings. A chord cannot give
   the surface stress (σ_nn = σ_tt cos²θ on it); a section meeting both flanks at right angles, a wedge's arc, can.

**Laws any fast model must obey**, checkable without knowing the answer:
- **Linear and covariant** in F under rigid motion and the mirror; E-free; ν only through held boundaries. **These do
  not tell models apart:** DB's parabola and ISO's 30° pass them too, since the tooth's axis moves with the tooth [rev].
- **The sharp limit:** as ρ_f → 0 the peak goes as ρ^−(1−λ), with sin(2λγ) + λ sin 2γ = 0 and 2γ = 270° − α_n:
  1−λ = 0.4273, 0.4137, 0.3998 at 14.5, 20, 25° (the hp round's rounded rack converges on it, `notch-research.md` §9.3).
- **The record's log-slopes** −d ln σ/d ln ρ_fP: 0.26–0.50 ordinary external, 0.246–0.32 rings, 0.36–0.42 z 1000;
  DB's L is 0.109–0.200, ISO's power 0.23–0.47. **Continuity** in every input, through undercut onset and the rack.
- **Laws that tell models apart** [rev; run so far only on LWW]: in the sharp limit, exact/model independent of the
  load's height; Neuber's deep hyperbolic notch in tension and in bending with the exponent set to ½, whose crack limits
  are 0.90 (c/ρ)^½·N/c and 0.60 (c/ρ)^½·6M/c², c the ligament (from §4's Creager–Paris limits [X, ed]); in the flat
  limit, a concave fillet's peak at least the nominal stress.
- **Taper alone is small** (Michell: −11 … +4 %); the 2.4× gap to a beam is ≈ 1.7× notch, ≈ 1.4× shear or proximity.

## 2. Candidates

Accuracy: ordinary external median (min … max) · tight median · rings median. Cost per section and load, compiled.

| Model | Mechanics basis | Frame-free? | Fitted or chosen constants | Continuity | Accuracy on the record | Cost |
|---|---|---|---|---|---|---|
| **A. Dolan–Broghamer** (today) | Lewis parabola section; K_f photoelastic fit; axial term under the same K | no: parabola, split | H, L, M; 1942 photoelastic | continuous | −15.0 (−30.1 … +7.4) · −52.9 · −26.8 | µs |
| **B. DB without axial term** | as A | no | as A | continuous | −2.3 (−13.7 … +15.4) · −46.4 · −10.8; not a published model | µs |
| **C. ISO Y_F·Y_S** | 30° tangent (60° ring), h_Fe arm; Y_S Hirt's fit | no: 30°, split | 1.2, 0.13, 1.21, 2.3; q_s clamp 8 | kink at the clamp | +6.4 (−7.1 … +22.8) · −29.9 · +26.3 | µs |
| **C′. ISO, Y_S continued** | as C | no | as C, past its range | continuous | as C · −3.7 (−23.9 … +30.6) · +27.7 | µs |
| **LWW** (frame round; P1) | the wedge bounded by each fillet point's tangent and its mirror; wrench carried to its apex; exact Flamant + Carothers edge stress × (c/ρ)^(1−λ); max over the fillet | **covariant, not free of a chosen frame**: section family (axis-normal chords) and along/across/moment split from the local symmetric wedge; needs symmetry | c = 2x and κ = 1 chosen (c = x: −17 %); exponent from the tool's α_n; κ = 0.910, if centred, a fit | continuous on the crate's fillets (a ≤ 94.5°; pole at 2a = 257°) | κ = 1: +9.6 (+2.2 … +13.9) · +12.3 · +5.8, none under; κ = 0.910 (holdout): −0.1 · +3.6 · −4.1, every ring under | 5–6 µs native, 9.4 µs wasm; 0.85 ms native under LinearRamp as built |
| **Exact, fast: Flamant-subtracted BEM** (P2) | plane-strain direct BIE on the exact outline; Flamant particular solution on the load's tangent; 3 teeth, 10 m rim, or the whole gear | yes, but for the half-plane frame at the load point (fails at the tip corner) | none fitted; body and a 0.15 m window chosen | fixed topology; jitter ≤ 0.5 % at L2 | L2 ≤ 0.63 % vs the record, −0.08 … +0.44 % vs independent FEM; tip corner −40 % | 32 ms sector, 0.80 s whole z 17, shipped wasm |
| **Same, hp-graded** (`notch-research.md` §9) | knots graded toward every non-analytic point | yes | none | no count jumps if grading fixed | to a ≈ 5e-8 floor, not exponential on the tooth; the body dominates | 0.4–0.8 s wasm at ≈ 1000 DOF |
| **Complex potentials, conformal map** (Aida–Terauchi; Baronet–Tordion; Cardou–Tordion) | exact given the map ω; σ_t = 4 Re[φ′/ω′] | yes | map order a knob | if the map fit is | unmeasured; map fit is the budget (Cornell: ±10 %); half-plane body | map build unmeasured; µs per load |
| **Neuber deep notch + chord wrench + Heywood proximity** | exact statics on the mirror chord; Neuber at the local ρ; one empirical term | partly | 0.72, ν ≈ 1/4; post hoc | continuous | +1.1 (−0.9 … +14.3) · +26.5 · +9.7; without proximity −29.0 · −15.5 · −30.9 | µs |
| **Cornell's modified Heywood** | beam + fitted fillet and proximity terms | partly | 0.26, 0.7, 0.72, ν | if maximised | −10.1 (−14.0 … −0.8) · +9.3 · −7.2 as drawn | µs |
| **Aida–Terauchi design formula** | fit to their exact solutions; 30° section | no | 0.66, 0.40, 0.08, 1.15 | wrong 1/ρ asymptote | −6.8 (−13.2 … −2.3) · −1.1 (−31.9 … +109.7) · +4.5 | µs |
| **Surrogate table of exact peaks** | Chebyshev/sparse grid over one tool family | invariant output | 10³–10⁴ generated nodes | seams at undercut, pointed tip, root land | 0.02–0.12 % per smooth dimension; ≈ 0.6 % across the undercut kink | µs; **one tool family only** |
| **Certified reduced basis (SCRBE)** | Galerkin on greedy snapshots | yes | stored basis | breaks at topology changes | not built | est. 0.01–0.5 ms |
| **Trefftz / MFS** | exact field equations, least squares | yes | pole placement | truncated SVD discontinuous | 0.004–0.008 % where it converges; fails on tight and rack-like | QR 8–11× an LU |
| **D. Computed K_t** (on-demand) | any exact solver on the exact outline | yes | none | to solver precision | ≈ 1 %: mesh ≤ 0.12 %, body ≤ 0.55 % (§6.3) | 0.1–1 s (decision file's estimate); P2 32 ms per sector |

**Rejected:** curved-beam analogies (nearly linear in 1/ρ), Neuber's shallow/deep interpolation, Wellauer–Seireg,
Michell's wedge alone. The frame round's "centred −7.5 … +9.7 %" was exact/rated: as rated/exact, **−8.9 … +8.0 %**.

## 3. Protocol and pre-registration

Written before any run (full text in the pre-review file; §6.1 lists every departure). It cast P1 as a per-candidate
search figure, a role the search does not have (§6.4).
- **Shared:** the record's 228 teeth (external z 12–1000, mate 43, α 14.5/20/25°, ρ_fP 0.01–0.38; rings z 40–200, mate
  17, α 20/25°, ρ_fP 0.03–0.2; x 0 and 0.5), its load (F_n at the crate's load point: HPSTC, the tip below ε_α = 1) and
  body (five teeth, 10 m rim; z 12 and 17 whole); every class; L1–L4; the path of contact; sweeps in x, ρ_fP, α, z,
  signed 1/z and the load; the laws; cost native and wasm32 ± simd128, pure Rust.
- **P1 (LWW),** scratch crate `p1rs/`. With Q the foot of fillet point P on the axis, x = |PQ|, c = 2x, u = 2a (a the
  tangent's angle to the axis): `σ(s) = κ · [k_N F_along/x + k_V F_across/x + k_M M_Q/x²] · (c/ρ)^(1−λ(α_n))`, with
  k_N = sin u/(u + sin u), k_V = sin²u/(sin u − u cos u) − (1 − cos u)/(u − sin u), k_M = sin u (1 − cos u)/(sin u −
  u cos u): Flamant plus Carothers carried to the apex, analytic in u, no guard. Variants: V1 κ = 1; V2 κ the
  training-half median (external z 12, 30, 150; rings z 40, 100) scored on the rest; V3 κ the ρ → 0 limit on the
  rounded rack, mechanics only if inside V2's holdout range.
- **P2,** `p2.py` subclassing `tools/fillet_bem.py`'s BEM, ported to `p2rs/`: three teeth; Flamant's half-plane
  solution on the flank's tangent, the BEM solving the residual; element counts fixed per piece, nodes a function of the
  geometry alone; L1–L3 aimed at 1, 0.3, 0.1 %; 0.15 module round the load excluded. Decisive: ms in wasm at ≤ 0.5 %.

## 4. Validation data to hold it to

- **The record:** 228 teeth, mesh convergence 0.010 % median, 0.121 % worst; Kirsch, Inglis, Golovin, four stepped bars
  against `tools/shoulder_trefftz.py` (0.06 %), planted faults; every body check it runs within its 0.5 % gate, but a
  ring outside them moves −0.55 % from five teeth to nine (§6.3).
- **Independent of it:** the checker's FEM (8 teeth, ≤ 0.10 %); Trefftz (3 teeth, 0.004–0.008 %); this round's FEM
  (6-node triangles, midside nodes on the exact curve, Kirsch 3.0001): 12 record teeth ≤ 0.038 %, 4 off it ≤ 0.051 %.
- **Kernels:** Flamant (2e-15); Michell and Carothers [C]; the Williams eigenvalues; Neuber's deep notch, both limits
  re-derived (→ 1; Creager–Paris (4/π)√(a/ρ), (8/3π)√(a/ρ)), source unread. The multi-load set L0–L4 lies outside the
  repository and needs its own `--run` before gating. Reported, not gated: Peterson 3.4 (0.5–2 %), Cornell (≈ 5 %).
- **Not covered:** loads off HPSTC; shaper-cut external teeth; thin rims (`MemberGear::rim_thickness`; LWW cannot see a
  rim); asymmetric teeth (LWW needs a mirror); the helical virtual spur (P1 and P2 are 2-D).

## 5. Sources

- Aida & Terauchi 1962, 1st and 2nd reports, Bull. JSME 5: https://doi.org/10.1299/jsme1958.5.161 [R, eqs. 6, 9, 23,
  25–29] · https://doi.org/10.1299/jsme1958.5.170 [R, eq. 9] · 3rd, https://doi.org/10.1299/jsme1958.5.176 [R p. 176]
- Cornell 1981, J. Mech. Des. 103:447, NASA copy https://ntrs.nasa.gov/citations/19830011871 [R, eq. 18–19, Fig. 10,
  Table 5, on Baronet–Tordion]
- Baronet & Tordion 1973, https://doi.org/10.1115/1.3438264 [A] · Cardou & Tordion 1985
  https://doi.org/10.1115/1.3258691 and 1989 https://doi.org/10.1115/1.3259016 [A] · Richard, Paré & Cardou 1989
  https://doi.org/10.1115/1.3258998 [M] · Nicoletto 1992 https://doi.org/10.1007/BF00420588 [A]
- Heywood 1948, https://doi.org/10.1243/pime_proc_1948_159_031_02 [A: "an empirical formula"] · Wellauer & Seireg 1960
  https://doi.org/10.1115/1.3663042 [A] · Wilcox & Coleman 1973 https://doi.org/10.1115/1.3438262 [A]
- Williams 1952 https://doi.org/10.1115/1.4010553 [M] · Creager & Paris 1967 https://doi.org/10.1007/bf00182890 [M] ·
  Filippi, Lazzarin & Tovo 2002 https://doi.org/10.1016/s0020-7683(02)00342-6 [M] · Noda & Takase 1999
  https://doi.org/10.1046/j.1460-2695.1999.00230.x [A] · Neuber 1937, *Kerbspannungslehre* [C]
- Helsing, RCIP tutorial, https://arxiv.org/abs/1207.6737 [A, fetched] · Gopal & Trefethen
  https://arxiv.org/abs/1905.02960 [A] · Costa & Trefethen https://arxiv.org/abs/2107.01574 [A] · Huynh, Knezevic &
  Patera SCRBE https://doi.org/10.1051/m2an/2012022 [M]
- Timoshenko & Goodier; Muskhelishvili 1953 (Flamant, Michell, Carothers, rational-map closed forms) [C]

## 6. Prototype results (built in `~/.cache/gearcalc-work/bending-mech/`, README there)

### 6.1 What changed after the pre-registration

- **P2's mesh, twice.** A tip arc plus one curve failed on undercut teeth (an element across the re-entrant corner read
  −40 %); fixed counts on each of four pieces failed on tight racks (−12 %). What stands: tip arc, flank, one curve for
  fillet and root, the other side on the loaded side's counts; levels set on twelve teeth before the grid.
- **The tip-corner tooth** was first reported unreachable because `gear-cli fillet` prints the load direction as NaN or
  (±1, 0) on pointed teeth. **That is the CLI's printing, not the crate** [X, rev; re-run, ed]: it rebuilds the
  direction from `load_line_crossing − load_point`, which coincide on the centreline; the crate rates with `load_at`'s
  (DB rates `external 8 25 1.2 0.38 8` at 1.707). The corner was loaded by hand; the crate loads the record's z 12,
  14.5°, x 0 teeth (ε 0.91–1.005) and the flipped set (ε 0.69–0.93) at or within 0.01 of it.

### 6.2 P1: LWW, results

**Accuracy on the record** (`p1_analysis.txt`): median (min … max), % under.

| Class | V1: κ = 1 | V2: κ = 0.910, holdout | V3: κ derived | A: DB | C: ISO | C′: ISO, Y_S continued |
|---|---|---|---|---|---|---|
| external ordinary (85) | +9.6 (+2.2 … +13.9) 0 % | −0.1 (−4.7 … +3.1) 52 % | −17.0, all under | −15.0 (−30.1 … +7.4) 89 % | +6.4 (−7.1 … +22.8) 7 % | +6.4, as C |
| external middle (66) | +13.0 (+7.1 … +19.1) 0 % | +3.1 (−2.6 … +7.3) 15 % | −13.6, all under | −31.1, all under | +2.4 (−20.5 … +24.8) 39 % | +2.4 (−20.5 … +30.6) 39 % |
| external tight (29) | +12.3 (+6.0 … +18.0) 0 % | +3.6 (−2.0 … +7.4) 18 % | −14.5, all under | −52.9, all under | −29.9, all under | −3.7 (−23.9 … +30.6) 55 % |
| rings (48) | +5.8 (+0.3 … +11.7) 0 % | −4.1 (−7.9 … −1.1) 100 % | −18.8, all under | −26.8, all under | +26.3 (−7.5 … +31.1) 4 % | +27.7 (+18.3 … +44.2) 0 % |

- **All 228 at κ = 1:** +10.1 % (+0.3 … +19.1), none under; a rewrite sharing no code reproduces every row to 3.5e-7,
  and LWW/FEM equals LWW/record within 0.05 points on 12 teeth [X, rev]. Spread, sd ln: LWW 0.036, DB 0.237, ISO 0.208,
  ISO continued 0.126. The smallest margin, +0.25 % (ring 40/25/0/0.03), lies inside the reference's body uncertainty
  (§6.3), which runs in LWW's favour (+0.81 % against nine teeth).

**The constants are chosen, not absent.**
- **Notch length.** c = 2x is one of several constant-free lengths. With c = x, re-maximised: −17.3 % (−24.0 … −11.4),
  all under, sd ln 0.033 [X, rev; re-run, ed]; the apex distance x/sin a diverges as a → 0. Neuber's deep notch fixes a
  prefactor only with a length and a load mode (0.90 tension, 0.60 bending, §1), which κ = 1 over-reads by +11 % and
  +67 %. The +10 % on the record is a property of the record's range. **V2** needs κ = 0.910 (0.908 over all 228);
  holdout exact/LWW 0.848–0.988, worst −7.9 % (a ring), every holdout ring under.
- **V3: the sharp limit moves with the load height, so no single κ can be derived** [X, rev; 20° re-run, ed].
  Exact/LWW on the rounded rack as ρ → 0, moving only the load:

  | α | y = 0.9 | y = 0.5 (V3's) | y = 0.14 | y = −0.3 |
  |---|---|---|---|---|
  | 14.5° | 0.711 | 0.740 | — | 0.856 |
  | 20° | 0.726 | 0.761 | 0.808 | 0.906 |
  | 25° | 0.740 | 0.783 | — | 0.962 |

  At y = 0.5 all three fall outside V2's holdout, at y = −0.3 all three inside. Even where the exponent is exact the
  ratio depends on the wrench: the component weights are wrong under any one κ (the moment weighs too heavily against
  the across force), κ is a fit, and the earlier "over-reads by about 20 % past its sharp term" is withdrawn.

**What P1 takes from a frame and from the tool** [rev]. The tooth's axis pairs P with its mirror, which defines the
wedge, c and Q; the wrench is split about the axis (the k_N term is an axial term); the maximum runs over axis-normal
chords, the family Lewis's parabola maximises over. What beats a convention is that family and split come from the
exact local-wedge solution, though still weighted wrongly; it is undefined for an asymmetric tooth. The exponent is the
rack corner's, but at finite z the fillet rounds no corner of a definite angle (tangent turning 57–64° on rings, 65–70°
at the rack, undefined on undercut teeth): P1 is not a function of outline, load, E and ν alone.

**Structure one constant cannot remove** (V1).
- **Tooth count and shift:** z 12 +9.5 … z 1000 +13.2; rings +5.3 to +6.2 (20° +6.8, 25° +3.3); undercut +7.5 against
  +10.1. Along the x sweep the bias climbs from +3.6 to +16.6 %, since the exact peak falls 21.5 % and LWW 11.6 %:
  about half the credit for positive shift [rev].
- **Load components** (L2, L3 on the compressive extreme): L1 across at the load point +9.3 % (rings +4.9); L2 along
  +5.8 % (−10.1 … +30.1); **L3 along the centreline at the tip, moment-free, −13.1 %, 173 of 180 under** (rings
  −12.5 %); L4 across, lower, +3.2 %, 20 under. The FEM puts L3 at −6.2 % and −18.2 % on two teeth [X, rev].
- **Path of contact,** lowest point to highest, against P2 at F: 17/43 −5.3 → +6.8 %; ring 60/17 **−28.7** → +9.2 %
  (FEM on the record's body −27.5 %); z 1000/43 **−31.5** → +19.4 % (FEM −30.9 %), the peak on the fillet 0.41–0.50
  from the load. rom-research's ring 60 figure (−23.8 %) is not reproduced and not offered as corroboration. Cause: §7.
- **Tip corner of 17/43** (the pre-registered ε < 1 corner point, loaded by hand): LWW 4.555, 4.535, 4.496 at δ 0,
  0.01, 0.03 against FEM 4.271, 4.260, 4.226: **+6.6, +6.5, +6.4 %** [X, rev].
- **c/ρ below 2.** The flipped set, heavily undercut ε < 1 teeth tip-loaded by the crate, reference FEM-checked to
  ≤ 0.05 % [X, rev]: z 8 −2.6 %, z 10 +1.1 %, z 12 x −0.3 (c/ρ 1.38) **−9.6 %** (all 14.5°); z 12, 20°, x −0.3 −3.7 %.
  The crate admits tool radii to the full-round clamp (0.597 at 14.5°), past the record's 0.38; against the record's
  instrument only [X, rev]: z 12, 14.5°, ρ 0.6 (c/ρ 2.01) +0.7 %, z 14 +2.0 %, z 17 +2.9 %; four tip-loaded ε < 1 teeth
  at c/ρ 1.58–1.92 +2.6 … +22.9 % (not yet checked for §6.3's corner artefact). Below c/ρ ≈ 2 the sign is not set by
  c/ρ, and the margin thins where the crate admits teeth the record does not hold; at c < ρ the notch factor is < 1.
- **Pointed teeth loaded at the apex get no rating** [X, rev]: P1 takes the force direction from the load point to the
  line crossing (`p1rs/src/main.rs:101–108`), which coincide there. 27 of 224 small teeth (z 6–14, x 0.6–1.4) go
  unrated, all pointed, apex-loaded and ε < 1, which the crate rates today. Fix: the direction from `load_at`.

**Location, laws, continuity.** The maximum is within 0.27 ρ of the exact peak. Rigid motion 1.3e-11, mirror exact,
linearity 1e-15, E and ν absent, Python equals Rust to 1e-9, Michell's ratios and Williams' slope reproduced
(`p1_laws.txt`). Every sweep moves smoothly (largest step 0.82 points); signed 1/z steps 7.1 points at 0 where rack
becomes shaper (exact −26.6 %, LWW −31.2 %). One local maximum on each of 235 teeth [X, rev]. The kernel's pole
(2a = 257.45°) lies outside the crate's fillets (a ≤ 94.5°); ρ is taken as |ρ|, so a convex stretch would be notched.

**Cost** (per section, tooth and load in hand): 5.0–6.0 µs native median, 9.4 µs wasm at the shipped profile
(`opt-level = "z"`, `wasm-opt -Oz`). **Under LinearRamp** (≈ 160 load positions per section in `worst_over_cycle`) P1
as built redoes its scan at each: 0.85 ms native, ≈ 38× the crate's own ramp sweep, ≈ 1.5 ms wasm [X, rev]. The wedge
stress is linear in the force and M_Q, and M_Q in the load point: per-station coefficients would make that cheap.

### 6.3 P2: the exact solve with a fixed topology, results

**Accuracy** (`p2_analysis.txt`): 223 teeth (the five corner-loaded apart, below); max |.|, median in brackets.

| Level | Elements, sector (whole gear) | Unknowns, sector | Against a same-body F solve | Against the record |
|---|---|---|---|---|
| L1 | 100 (258–348) | 400 | 0.63 % (0.04) | 0.84 % |
| L2 | 128 (304–404) | 512 | **0.43 % (0.03)** | **0.63 % (+0.08)** |
| L3 | 178 (408–538) | 712 | 0.29 % | 0.51 % |
| F | 388 | 1552 | — | 0.45 % (+0.03); rings +0.08 at the median (three teeth against five) |

- **Independent check: the FEM** [X, rev], on 10 record teeth not corner-loaded: L2 −0.08 … +0.44 %, F −0.01 … +0.37 %
  (largest ring 40, mostly the 3- against 5-tooth body). "F matches rom-research's `m4`" is **not independent**
  (`fbem2.py` imports the record's BEM, as P2 does). E drops out; ν moves 0.13 % per 0.05; Rust equals Python to 2e-11.

**Body.**
- **The record's ring body reads high.** FEM on ring 40/25/0/0.03 [X, rev]: 2.7059 (3 teeth), 2.6953 (5, the
  record's), 2.6803 (9), 2.6804 (13). Five to nine moves **−0.55 %**, past the record's 0.5 % body gate (its self-test
  widens only ring 40/20/0/0.1); ring 100 moves −0.1 %. There, every model reads 0.55 points too unconservative.
- **P2's body is a chosen constant.** The sweeps' sector (rim min(10, 0.7 r_f), continuous) against the whole gear at
  L2: −0.12 … −1.01 % on seven pinions z 12–30, all unconservative [X, rev; extended, ed]; the record's whole↔sector
  switch jumps −0.21 % at z 30; helical members have no whole body (z_v = z/cos³β is not an integer).
- **A degenerate scale.** P2 inherits the record's BEM, singular at one unit of length (`notch-research.md` §9.4); the
  record's bodies are ≥ 5× larger, a live solve in arbitrary units is not, and P2 lacks the augmented system.

**The tip corner.** The half-plane particular solution is not traction-free on the tip land. The five corner-loaded
record teeth (z 12, 14.5°, x 0) read −15 to −40 % at every level. Against the load's distance δ from the corner (z 12 at
14.5° and 17/43, the latter corrected against the fillet-only and FEM reference [X, rev]):

| δ | 0 | 0.01 | 0.03 | 0.1 | 0.3 |
|---|---|---|---|---|---|
| L2 | −40 to −43 % | −28 % | −11 to −12 % | ≤ 1.1 % | ≤ 0.4 % |
| F | −40 to −42 % | **−11 to −12 %** | −0.6 to −1.1 % | 0.0 % | 0.0 % |

- **The record's patch instrument misreads a load at the corner** [X, rev; logs re-read, ed]: its `peak_ls` takes flank
  values next to the load, 5.64 at δ 0 (fillet maximum 4.2646) and 3.7963 at δ 0.01, below its own fillet maximum
  4.2539. The 228 are unaffected (on the corner-loaded and flipped teeth its peak is on the fillet, the flank maximum
  below it); a future tip-loaded reference must read fillet and root only.
- **A live P2 needs** Flamant's wedge solution at the corner, or a patch: every ε < 1 tooth is loaded there, and every
  LinearRamp sweep loads the far end of contact at d = 0 (`strength.rs:1821`), the tip corner wherever the mate reaches.

**Continuity.** L2's jitter against F ≤ 0.12 % (x), 0.07 (z), 0.09 (ρ), 0.29 (1/z), 0.50 (α, at the undercut onset).
Guards: the 0.15-module window (never near a maximum), the tip corner, the body rule; pointed teeth lose the tip piece.

**Cost** (`p2_native_bench.txt`, `wasm_bench*.txt`, `rev3/zbench.txt`), per new geometry, assembly plus pure-Rust LU:

| Level | Unknowns | Native | wasm, opt 3 | wasm, simd128 | wasm, shipped profile [rev] |
|---|---|---|---|---|---|
| L1, sector | 384–400 | 7–9 ms | 11–14 ms | 10–12 ms | 6.3 + 8.8 = 15 ms |
| L2, sector | 512 | 14–20 ms | 23–31 ms | 19–25 ms | 11.9 + 20.5 = **32 ms** |
| L3, sector | 712 | 29–31 ms | 55–73 ms | 38–51 ms | 18.6 + 56.1 = 75 ms |
| L2, whole z 17 | 1616 | 0.28–0.36 s | 0.57–0.82 s | 0.37–0.54 s | **0.80 s** |

- The prototypes' wasm was opt-level 3 and no `wasm-opt`; the payload is `opt-level = "z"`, `wasm-opt -Oz`, no simd128.
  Every preset with line meshes has a member the record's rule models whole (z ≲ 29 at x 0). **Per extra load: not
  reliably measured** (the bench subtracts separately timed runs: −0.24 to 5.2 ms); the right-hand side, ramp sweep,
  adjoint stations and Rust mesh generation are not built. An L2 factor is 2 MB, a whole z 29 56 MB, and wasm memory
  never shrinks. `extremes_cases.txt`'s "P2 F nan" is a swallowed out-of-memory error; L2 solves it to 0.04 % [rev].

### 6.4 The fast mode inside the crate [X, rev; code paths checked, ed]

- **The search never calls the bending rating.** It scores `trial_efficiency` (`train/shape.rs:2220`); sections are
  built once per (member, line mesh) in `shape::cut`, after the search, and scaled per case. If bending entered it, each
  of 1,967 evaluations (17/43) would be a new geometry: P2 100–125 s per search in wasm, P1 ≈ 35 ms against 22 ms today.
- **What multiplies the cost:** one solve per input change, two per case edit, two per menu hover (the dry run), ≈ 160
  load positions per section under LinearRamp. A solve takes 0.9–3.9 ms per preset today (22 ms to 2.5 s with the shift
  search on). P2 uncached, sector rule everywhere: spur ≈ 55–65 ms, layshaft ≈ 160–190 ms per solve, doubled on hover.
  A cache helps only load edits, and `docs/rationale.md` ("Inputs are the only state"; "No worker, no async, no loading
  states") rules out "cached live" as the crate stands.

## 7. Open issues

1. **What drives LWW's under-read low on the path (−28 to −31 %):** the load's own field past the Saint-Venant floor
   (the first reading) or the component weighting (the frame review's). The exact solver with the load replaced by its
   wrench at each height would separate them. Not run.
2. **Does a constant-free closed form exist?** Sharp-limit coefficients per component (N, V, M) on the rack corner at
   each α, computed once by the exact solver, then tested on the load-height, L3 and path trends; if they depend on the
   body, no closed form here is constant-free. Not built.
3. **P1 across the admissible space** (full-round tools, negative shifts on small pinions, apex-loaded pointed teeth
   with the direction from `load_at`; tip-loaded references read on the fillet only), restructured per station for
   load sweeps, and gated on sensitivity to shift and load height (d ln σ/dx against the exact), not only the median.
4. **P2 before any use:** a corner particular solution; one continuous body rule with its bias stated, for every z and
   helical z_v; the augmented system; right-hand side, load sweep and mesh generation in Rust, timed per preset.
5. **Outside this file, not edited here:** the record's ring body reads high by up to 0.55 %, past the 0.5 % gate
   `docs/state.md` records for every body check; `gear-cli fillet` prints a NaN or (±1, 0) direction on
   centreline-loaded pointed teeth, which `tools/fillet_bem.py` inherits (the crate's rating is unaffected);
   `notch-research.md` §9.4 quotes the withdrawn P2 headline ("3.5–11 ms within 1.44 %"). B's spread is not computed.

## 8. Findings for the owner

**What a mechanics-based fast model can achieve,** as measured. **Frame:** a closed form can be covariant and
continuous, taking its section family and load split from an exact local solution (LWW) rather than a convention (30°,
the parabola, the axial term); it cannot shed the tooth's axis, which defines its wedge, or the tool's α_n. Only a solve
on the body is frame-free (P2, D), and P2 keeps a half-plane frame at the load point. **Constants:** LWW has no fitted
constant but two chosen ones; the record happens to bound the choice (+10.1 %), an equally plain one is all under
(−17 %), and no single derived prefactor exists. **Accuracy:** a sixth of the published baselines' spread on the
record, with structured errors off it. **Cost:** µs per section at one load, ≈ 1 ms under the ramp as built.

| Model | Frame | Constants | Ordinary · tight · rings, median % | sd ln | Known off-record failures | Cost per section, wasm |
|---|---|---|---|---|---|---|
| DB (today) | parabola, split, axial term | H, L, M; photoelastic fit | −15.0 · −52.9 · −26.8 | 0.237 | not measured here | µs |
| B: DB without axial | parabola, split | as DB | −2.3 · −46.4 · −10.8 | not computed | not measured here | µs |
| C′: ISO, Y_S continued | 30° tangent, split | Hirt's fit, past its range | +6.4 · −3.7 · +27.7 | 0.126 | not measured here | µs |
| P1: LWW, κ = 1 | covariant; wedge from the axis | c = 2x, κ = 1 chosen; exponent from α_n | +9.6 · +12.3 · +5.8 | 0.036 | path −31; L3 −13; c/ρ < 2 −9.6; apex unrated | 9.4 µs; ≈ 1.5 ms ramp as built |
| P1: LWW, κ = 0.910 | as above | κ fitted | −0.1 · +3.6 · −4.1 (holdout) | 0.036 | as above, × 0.91 | as above |
| P2 at L2 | frame-free but at the load | none fitted; body chosen | within 0.63 % of the record | — | tip corner −40 %; sector body to −1.0 % | 32 ms sector; 0.8 s whole pinion |
| D: computed K_t | frame-free | none | ≈ 1 % (body ≤ 0.55 %) | — | — | 0.1–1 s (estimate); hp 0.4–0.8 s |

**The decision the owner faces** (not chosen here; Q10's existing options are not re-assessed):
1. **What "generic" must mean for the fast mode:** covariant with derived sections and splits, which LWW achieves, or
   free of every chosen constant, which so far only a body solve delivers.
2. **Whether a model conservative on the record's domain, with structured errors off it, can be a fast-mode figure**
   under rule 6, which wants each bias's size and sign.
3. **Where the next round goes,** if anywhere: the per-component coefficients (§7.2), which test whether a
   constant-free closed form exists; P1's known repairs (load direction, ramp, admissible-space validation); or neither.
4. **P2's class.** Live needs rule 3 amended, the solve on a worker with a loading state, and cache memory bounded;
   otherwise it is option D made cheaper (≈ 32 ms per sector member), still needing the corner, body and scale fixes.
5. **Whether bending will ever enter the shift search.** Today it does not, and that sets every cost budget above.
