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
1. **Patch to point force** ≤ 0.04 %. **Point force to wrench** (Saint-Venant) 2.3 % median, 4.0 % worst at HPSTC
   (*superseded*, §9.1: −1.0 % median, −3.6 … +0.3, with the moment at the tip centre), growing as the load nears the
   fillet (along 17/43's path the peak falls to 2.729 at r 8.17, then rises to 2.914 at r 8.00); unmeasured at the
   lowest point on z 1000 or ring 60 (§7).
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

## 9. Component model (components round, 2026-09-30; three reviews applied)

For the owner's direction of 2026-10-03, item 3 (`plan.md` §5): split the stress into its load components, give each
its own mechanics, and see whether a fast model built that way escapes item 2's structured errors.
- **Baseline:** the exact elastic fillet peak over root land and fillet, as `work/bending-options.md` names it. On the
  record that is `tools/fillet_bem.txt`. Off the record it is P2 at level F, or the record's own solver, as the options
  file says for each column.
- **Figures:** model / baseline − 1 in %, negative unconservative. A structure is a range in points.
- **The rule:** the options file's, one rule for every row. Substantial means ≥ 5 anywhere measured; *at the threshold*
  means 4–6, inside the reference's ±1 point.
- **Marks:** [X] this round's; [X, rev] a reviewer's run; [X, ed] the final editor's. No source was read, so the
  literature here is [C], except one piece of metadata [M].
- **Scripts:** `~/.cache/gearcalc-work/bending-comp/` (README there). The reviewers' and the editor's checks are in
  `bending-options/final/`. Nothing in the crate changed.

**Short answer.**
- **The split is exact in the exact field, and the components do need their own mechanics.** Superposition holds at
  the load point to 5e-8. At the exact peak no one-factor form gets the along force, the across force and the moment
  right together.
- **Neuber's deep hyperbolic notch works as a closed form, but not alone.** Solved here in closed form for all three
  components and fitted to each fillet point, it places the along force's action within 0.01–0.02 h of exact (§9.3).
  It cannot rate a tooth alone.
- **The model built on it is a hybrid, not a superposition.** "Neck components" (§9.4) takes N and M from the neck at
  a corner-mapped sharpness, and V from LWW's term about a chosen reference point. Its answer depends on that point
  and on a named pivot q0.
- **It misstates comparisons less than LWW,** with a p90 of 6.8 points against 8.4, and 30 reversals of 22,346 pairs
  against 66.
- **But by the one rule it still carries substantial structure on seven of nine variables,** two of them at the
  threshold.
  - It removes the ring structure and most of the structure at ordinary fillets.
  - It keeps or adds structure at tight tools and 14.5°: shift 8.1, notch 10.8, record groups x 10.6 and z 9.9.
  - Direction is worse than LWW at every span, reaching 11.7 at ±14°.
- **Its accuracy at the design load is partly cancellation.** V reads 5–18 % high and M 2–9 % low on externals.
- **The residual is in V, LWW's own term reused,** not in the neck (§9.4).
- **Cost:** ≈ 100 µs per section in wasm, 10.7× LWW, measured.
- No closed form here reaches "no substantial structure".

### 9.1 The exact reference, per component [X]

17 record teeth (12 external z 12–1000, 5 rings) were rerun on the record's BEM class and body (`refsubset.py`: one LU,
seven loads). The loads were unit N and V at the reference point (the tip centre), a unit couple M (an along pair on the
tip land), and unit along and across forces at the load point.
- L0's sampled peak against the record: −0.012 … +0.031 %. The rerun equals comp-fast's earlier run bit for bit.
- **Superposition at the load point is exact:** L0 = F_across·L1 + F_along·L2 at every station, to 1.4e-8 … 4.9e-8 of
  the peak. On all 228 it holds to ≤ 4.9e-8 [X, rev]. This is a property of the BEM (linear elasticity), not of any
  model below.
- **The wrench at the reference point is not exact.** V g_V + N g_N + M g_M reads −3.1 … +0.1 % of L0's peak (median
  −1.0): the load's own near field at HPSTC. On all 228 it reads −1.0 % (−3.6 … +0.3) [R, comp-fast; X, rev]. This
  supersedes §1's "2.3 % median, 4.0 % worst", which took its moment from a lower-flank load and carried that load's
  near field (now marked there).
- Every model below is therefore a model of the remote wrench, and it carries that floor low on the path: −0.4 … −6.7
  at LPSTC, down to −48 at the lowest point, relative to HPSTC [R, components round].

### 9.2 A closed form for each component: Neuber's neck, all three [X]

On z = i c sinh ζ, the strip |Im ζ| < θ maps onto the region between the branches x = ±c cosh ξ sin θ,
y = c sinh ξ cos θ. The waist half-width is c sin θ, the root radius c cos²θ / sin θ, and the asymptotes lie at ±θ
from the axis. Traction-free branches reduce Muskhelishvili's boundary condition to one equation with constant
coefficients, Ḡ(w + 2iθ) − Ḡ(w − 2iθ) − 2i sin 2θ G′(w) = const (φ = G(ζ)). Its solutions that carry a resultant are
φ = iαζ (tension), φ = Aζ (shear, zero moment at the waist) and φ = B cosh ζ (a pure couple).

On the branch, take D = sinh²ξ + cos²θ, N in tension, V the force the upper part puts on the lower, and M about the
waist centre with the +x side in tension:

    σ_N = N · 4 cosh ξ cos θ / [2 (2θ + sin 2θ) · c D]
    σ_V = V · 4 sinh ξ sin θ / [2 (2θ − sin 2θ) · c D]
    σ_M = M · 4 sin 2θ / [(sin 2θ − 2θ cos 2θ) · c² (cosh 2ξ + cos 2θ)]

- **Checks** (`neck.py` → `neck.txt`; rerun to the same output [X, rev]):
  - The traction residual is ≤ 1e-15 of the largest stress on both branches.
  - The three resultants, integrated across the waist, equal the closed forms to 1e-6.
  - At the waist, the tension and bending factors equal Neuber's deep-notch formulas to 7 digits (a/ρ 0.25 … 100). A
    reviewer confirmed this algebraically against the formulas as recalled [C].
  - The shear case is derived here; Neuber treated it too [C].
  - E and ν drop out, as for any traction problem on a simply connected body.
- **Fitting it to a fillet point.** Take a point at distance x from the axis, with tangent angle a and radius ρ. The
  neck with tan a = tanh(−ξ) tan θ, x = c cosh ξ sin θ and the same curvature is unique for every a in [0, 90°) and
  every x/ρ > 0, because x/ρ is monotone in θ over (a, 90°) (`match_check.txt`).
- **Its flat limit is LWW's wedge, exactly.** As ρ → ∞, θ → a, and the three responses become the Flamant and
  Carothers edge stresses LWW uses: k_N, k_V and k_M to 1e-6 (`flat_check.txt`).

### 9.3 What the fitted neck gets right, and why it cannot rate alone [X]

**The mix.** Each quantity is read at the exact peak station, against the exact per-station components (`extra.txt`).
x* is the along force's pole, η* the across force's, and h the chord. "Neck" is the fitted neck alone. "Built" is the
neck components model of §9.4. Its V is LWW's, so its η* is not the neck's.

| Class | x*/h: exact · neck · built · LWW | η*/h: exact · neck · built · LWW | Bending level (exact = 1): neck · built · LWW |
|---|---|---|---|
| ordinary | −0.248 · −0.231 · −0.228 · −0.189 | −0.318 · −0.283 · −0.359 · −0.326 | 1.046 · 0.978 · 1.062 |
| middle | −0.263 · −0.244 · −0.242 · −0.201 | −0.400 · −0.356 · −0.485 · −0.442 | 1.122 · 0.952 · 1.030 |
| tight | −0.268 · −0.249 · −0.248 · −0.211 | −0.436 · −0.423 · −0.566 · −0.555 | 1.238 · 0.912 · 0.948 |
| rings | −0.258 · −0.247 · −0.246 · −0.217 | −0.555 · −0.508 · −0.585 · −0.628 | 1.185 · 1.008 · 0.932 |

- **The along force.** The neck, and the model built from it, place its pole within 0.01–0.02 h of exact on every
  class. LWW's wedge is 0.04–0.06 h off.
- **The across force.** The neck is nearer than LWW on tight fillets and rings, about level on middle ones, and further
  on ordinary ones. The built model's pole comes from LWW's V over the neck's M. It is further from exact than LWW's on
  every external class, though nearer on rings [X, rev]. "Gets the mix right" is therefore true of the neck, not of the
  model built from it.
- **At those stations the fitted neck alone** reads L0 at ordinary −1.1, middle +6.1, tight +20.7 and rings +12.3.
  Its direction structure is 0.9 / 5.3 and its load-height structure 1.2 / 6.4. At the same stations LWW reads 1.4 /
  4.4 and 1.5 / 7.7 (`atpeak.txt`).

**The level and the location fail**, for two reasons of body:
- **The neck is a deep notch on both sides.** Its sharp limit is a double-edge crack (exponent ½); the tooth's is the
  rack corner (1 − λ = 0.40–0.43). Its bending level therefore climbs with sharpness (the table's last column). At the
  peak station its notch structure is 15 / 34.
- **Its curvature peaks in the wrong place.** The hyperbola's curvature is largest at its waist; the fillet's is largest
  near its root end.
  - A point low on the fillet (a → 90°) is fitted by a neck with θ → 90° and its waist just above: a crack tip.
  - There the neck's shear term becomes a mode-II crack field, with neck/wedge for V at 26–95 at a 89–90° (rack,
    ring 60).
  - The exact V is 2.8–3.1 and flat along the whole fillet, while M and N rise toward the root end in both.

Its own maximum therefore runs to the root end, and it reads +163 % at the median on the record
(`record_summary.txt`). **It is not a rating.**

### 9.4 The model built: neck components, a hybrid [X]

**The model.** At each fillet station, σ = N·s_N + V·s_V + M_Q·s_M. The load's wrench is taken about **Q, the
station's foot on the axis** (a chosen reference point), and the rating is the maximum over the root and fillet (the
stations LWW scans). The three terms:
- **Along force N and moment M.** The fitted neck's σ_N and σ_M (§9.2), evaluated at the corner-mapped sharpness
  q_eff = q0 (q/q0)^(2(1−λ)), with q = x/ρ and q0 = 1. The map turns the neck's sharp-limit exponent ½ into the rack
  corner's 1 − λ(α_n) and leaves the flat limit unchanged.
- **Across force V.** The tangent wedge's V response about Q, × W = (2x/ρ)^(1−λ). This is LWW's V term, unchanged.
  - About Q it is not a pure shear. It is Carothers' shear plus the moment V·x·cot a, since the wedge's apex lies
    x·cot a above Q [X, rev]. Part of the bending therefore travels through LWW's power law, not through the neck.
  - The earlier claim that "LWW's term tracks" the exact V (exact / W 0.77–1.09 across tangent angles, `collapse.py`)
    does not hold at the peak. There it reads 1.10 of exact at the median, with a 35-point spread (below).

**It is not superposition: the reference point matters** [X, rev].
- **Why.** In the exact field the response does not depend on where the wrench is taken, since s_V(Q′) = s_V(Q) +
  h s_M. The model's does, because its V and M factors come from different bodies. Moving the reference h up the axis
  changes σ by V·h·(s_M,neck − W·w_M).
- **How much.** At the exact peak station:

  | Reference | Median | Ring-minus-ordinary step |
  |---|---|---|
  | Q (as built) | +2.9 | +1.6 |
  | the neck's centre | +3.4 | +0.8 |
  | the wedge's apex | +7.3 | −6.3 |

  As a maximum over the fillet, the apex version diverges, since a → 0 at the fillet's top.
- **So** Q is a chosen constant of the model, like κ.

**Constants.** None is fitted. The model was nonetheless chosen from 13 structural variants scored on the record (§9.5).
- **Derived:** the three neck solutions and the fit to each point.
- **Chosen and named:**
  - the reference point Q;
  - the corner map, both its exponent 2(1 − λ) and its pivot q0 = 1;
  - the notch length: x for N and M, c = 2x for V;
  - κ = 1 for V.
- **The exponent is chosen, not derived:** it interpolates between the neck's flat limit and the sharp corner's
  exponent, and the record's teeth never reach that sharp regime [X, rev].
  - The exact log-slope −d ln σ/d ln ρ_fP from ρ_fP 0.01 to 0.03 is 0.08–0.09, against 1 − λ = 0.40–0.43.
  - The reason is that a sharp tool corner cuts a trochoid of finite curvature at finite z. The radius at the peak
    stops falling: z 12 reads 0.389 / 0.402 at ρ_fP 0.01 / 0.03, z 30 reads 0.148, and only z 1000 gets small
    (0.015).
  - The model's slope overshoots the exact one by +0.016 … +0.039 at 14.5°, +0.011 … +0.023 at 20° and +0.004 …
    +0.009 at 25°. Summed over the tool-radius steps, that is ≈ 9, 6 and 3 points. It is the notch structure reported
    in §9.5, and the reason its worst cases fall at 14.5°.
- **Sensitivity** [X, rev; two reviewers agree]:
  - **q0 moves the median, not the structure.** q0 = 0.25 / 0.5 / 1 / 2 / 4 reads −0.9 / +1.1 / +3.1 / +5.4 / +8.1,
    while the c/ρ bins stay at 5.5–6.1 and the notch sweep at 6.8–8.5. The +3.1 median is a choice as arbitrary as
    LWW's κ, not a gain of mechanics.
  - **V's c moves the structure.** c = x reads −10.4 with notch sweep 3.9, record ρ groups 7.9 and x groups 5.6.
    c = 4x reads +22.9 with notch 12.2.

**Per component, at the exact peak station** [X, rev; checked, X, ed]: model / exact, median (range).

| Component | All 228 | Ordinary | Middle | Tight | Rings |
|---|---|---|---|---|---|
| V (LWW's term, about Q) | 1.103 (0.953 … 1.307) | 1.08 | 1.17 | 1.18 | 1.05 |
| M (neck) | 0.973 (0.875 … 1.041) | 0.98 | 0.95 | 0.91 | 1.01 |
| N (neck) | 0.908 (0.814 … 1.016) | 0.91 | 0.88 | 0.84 | 0.95 |
| M, for comparison: LWW | 1.029 | 1.06 | 1.03 | 0.95 | 0.93 |

- **V varies with the design.** It rises with sharpness: 1.078 at ρ_fP 0.38, 1.134 at 0.1 and 1.200 at 0.01. It falls
  with α: 1.157 at 14.5°, 1.099 at 25°.
- **The errors, in points of the peak** (ordinary / middle / tight / rings):

  | | Ordinary | Middle | Tight | Rings |
  |---|---|---|---|---|
  | V's error | +4.0 | +8.4 | +9.9 | +3.1 |
  | M's error | −1.5 | −2.9 | −5.0 | +0.4 |
  | Sum of the absolute term errors | 6.5 | 12.7 | 17.8 | 5.0 |
  | Net error | 3.4 | 6.0 | 7.0 | 4.1 |

  The model's accuracy at the design load is V high cancelling M low.
- **The mix, as the along-minus-across error at the load point:** +0.4 on ordinary fillets, −7.6 on middle, −8.3 on
  tight. Over the record it spans −15.6 … +13.1, no narrower than LWW's −11.8 … +14.8.

**Which component carries the structure that remains** [X, rev]. Each component in turn is replaced by the exact
per-station response:

| Variant | ρ groups, max | x groups, max | c/ρ bins, low → high | Direction ±6°, max |
|---|---|---|---|---|
| As built | 12.1 | 10.6 | +0.1 … +6.2 | 4.5 |
| Exact V | **3.4** | 6.6 | +0.0 … −3.9 | **1.6** |
| Exact M | 13.8 | — | +0.8 … +11 | — |
| Exact N and M | 13.5 | — | — | — |

The residual is V's, and V is LWW's term. Correcting M or N alone makes the structure worse, because M's low error
partly cancels V's high one.

**Guard.** Points whose tangent turns past the axis normal (a ≥ 90°, on ring root lands only) have no symmetric neck
and are not rated. No maximum lies near one. The model searches the lower flank as well for turned and unit loads; that
changes none of the 228 [X, rev].

**Continuity.**
- The fit is smooth in every input, and flat or convex points take the flat limit continuously.
- The largest step between sweep neighbours is 0.14–0.44 points. The exception is 1.26, at the sharp end of the notch
  sweep, where the curve is smooth. The step across the rack is 0.0.
- The maximum is taken finely enough [X, rev]: 2001 stations per piece, at most 0.002 ρ apart. Refining it with a
  parabola moves it by ≤ 1e-4 %. No maximum sits at the end of a piece.
- The flat limits differ. As ρ → ∞, N and M tend to the wedge, but V tends to 0, since W → 0 (LWW shares this). That
  matters for c/ρ < 2, which is untested.

**Location.** The model's maximum lies 0.05 ρ from the exact peak at the median and 0.15 ρ at the worst (LWW: 0.08,
0.28).

**Cost** [X, ed; `bending-options/final/neckwasm`].
- **What was built.** The model in Rust, on the P1 crate's scan (16 samples, then a golden refine, per section). The θ
  solve uses gear-core's Brent on ln Q − ln q, to 1e-12 rad, at ≈ 11 evaluations per station.
- **Checked.** It reproduces the Python prototype to 1e-6 on all 228.
- **Measured, at one load:**

  | Build | Neck components, per section | LWW, same build | Ratio |
  |---|---|---|---|
  | wasm32 under node | 98–101 µs | 9.1–9.5 µs | 10.7× |
  | native | 65 µs | 6.1 µs | 10.7× |

  The prototype's 80 halvings cost 50×. The earlier estimate of 2–4× is withdrawn.
- **Not tried or built.** A better-started θ solve could cut the cost; it was not tried. The model is linear in the
  load, so a ramp could reuse each station's three responses; that is not built.

### 9.5 Against the baseline [X]

Sources: `record_summary.txt`, `sweeps.txt`, `path.txt`, `where.txt`, `bending-options/final/`.
- LWW, rerun through the same pipeline, reproduces its options-table row to 0.01 points.
- **In-sample.** The record figures are in-sample: 13 structural variants were scored on the full record (neck, C, D,
  Va, Vc, Ve, Vn, E1, E2, G, DW, DW088, QW) [X, ed; rev].
- **Out of sample.** The sweeps and the path (run after `models.py` was fixed) and the reviewers' new sweeps are out of
  sample. The worst figure in sample is 12.1 (record ρ groups); out of sample it is 10.8 (notch at 14.5°) and 8.1
  (shift at a tight tool, 14.5°).

| | Neck components | LWW, κ = 1 |
|---|---|---|
| All 228: median (min … max) · IQR width | +3.1 (−0.7 … +14.4) · 3.5 | +10.1 (+0.3 … +19.1) · 5.3 |
| Ordinary · middle · tight · rings · undercut | +2.0 · +5.4 · +5.5 · +3.5 · +1.5 | +9.6 · +13.0 · +12.3 · +5.8 · +8.7 |
| Load position LPSTC → HPSTC, 11 teeth | 1.5 / 5.0 (5.01, at the threshold) | 3.8 / 9.6 |
| Direction ±6° · ±10° · ±14° | 1.8 / 4.5 · 3.0 / 7.7 · 4.4 / 11.7 | 1.3 / 4.0 · 2.3 / 7.0 · 3.4 / 10.7 |
| Load height: across at L, near the tip, at the tip centre | 1.1 / 5.0 (5.05, at the threshold) | 2.2 / 10.9 |
| Shift sweeps: ρ_fP 0.25, 20° · 0.03, 20° · 0.03, 14.5° | 3.5 · 3.7 · **8.1** | 13.0 · 9.9 · 10.0 |
| Shift: record groups [≥ 5 at ρ_fP ≥ 0.1 · ≤ 0.03] | 0.7 / 10.6 [1/51 · 4/34] | 2.6 / 7.2 [10/51 · 4/34] |
| Teeth z 12–220 · record groups [≥ 5 at ρ_fP ≥ 0.1 · ≤ 0.03] | 3.4 · 3.2 / 9.9 [2/18 · 7/12] | 7.0 · 3.9 / 7.5 [8/18 · 3/12] |
| Mate z: ρ_fP 0.25 · 0.03 · ring | 0.9 · 2.7 · 0.8 | 3.5 · 2.1 · 1.5 |
| Pressure angle 14.5–25° · 25–31° · record groups | 1.2 · 3.3 · 2.8 / 8.1 | 0.6 · 4.9 · 3.0 / 8.1 |
| Notch sweeps at 14.5° · 20° · 25° · record groups | **10.8** · 7.6 · 3.8 · 5.1 / 12.1 | 7.0 · 3.7 · 1.8 · 4.9 / 8.7 |
| Median by c/ρ bin, < 4 … > 32 | +0.1 … +6.2 (range 6.1) | +5.6 … +13.8 (range 8.2) |
| Ring vs external: the step across the rack at ρ_fP 0.2 · 0.1 · 0.03 · class medians | 0.0 · +0.7 · +0.7 · +1.5 | −7.1 · −7.2 · −5.1 · −3.7 |
| All record pairs: misstatement median · p90 · max · reversed of 22,346 | 2.4 · 6.8 · 13.2 · 30 | 3.5 · 8.4 · 18.8 · 66 |
| Share of benefit credited: shift (ρ_fP 0.25) · notch (20°) | 87 % · 108 % | 54 % · 104 % |
| Outside the span: tip side − HPSTC · the lowest point | −3.4 … +2.4 · −34.1 … −1.4 | −0.3 … +9.9 · −31.1 … +0.5 |

An earlier row, "along the tooth", compared the most compressive stress under the along load, not the baseline's σ_tt
maximum. That extreme changes with the domain on 217 of 228 teeth, by up to 275 % [X, rev], so the row is withdrawn,
along with the claim built on it (−13.0 → −5.8).

**What the split does, by the one rule:**
- **Removes:**
  - ring against external: the step across the rack is 0.0 · 0.7 · 0.7, against LWW's 7.1 · 7.2 · 5.1;
  - most of the ordinary-fillet structure: record groups ≥ 5 at ρ_fP ≥ 0.1 are z 2/18 and x 1/51, against LWW's 8/18
    and 10/51;
  - the shift structure at ordinary fillets, 13.0 → 3.5.
- **Reduces, but leaves at the threshold:** load position 9.6 → 5.0 (5.01, at z 30, 25°, ρ_fP 0.25: an ordinary
  fillet, not a notch effect), and load height 10.9 → 5.0 (5.05).
- **Keeps or adds (substantial):**
  - **At tight tools:**
    - shift at ρ_fP 0.03 and 14.5°: 8.1, with 7.8 of it through undercut onset;
    - the notch sweeps: 10.8 · 7.6 · 3.8, which is 1.5–2.2× LWW at every angle;
    - the record's tight-fillet groups: z ≥ 5 in 7 of 12 against 3 of 12, worst z 9.9 against 5.3, worst x 10.6
      against 7.2.
  - **Direction,** worse than LWW at every span and on 160 of 228 teeth at ±6°.
  - **The worst groups.** In z, x and α they sit at ρ_fP 0.01, but not all at 14.5°. At 20° the z group (x 0,
    ρ_fP 0.01) reads 9.2 (+0.2 → +9.4 → +5.7 across z 12 → 60 → 1000; LWW 2.7). The x group (z 12, 20°,
    ρ_fP 0.01) reads 8.2 (LWW 7.2). LWW's worst α group is also at a tight tool (z 150, x 0, ρ_fP 0.01), so the
    tight-tool attribution applies to both models.
- **Not addressed:** the load's own near field. The lowest point reads −34 %, as it does for every wrench model.

The earlier conclusion, "removes shift, ring, load position and load height; leaves only the notch", overstated the
split. The correct statement: it is reduced at ordinary fillets and at 20–25°, and it stays substantial at 14.5° with a
tight tool, in the tight-fillet record groups and in direction.

**Variants measured and set aside** (their own maxima on the record; three of the thirteen):
- The neck for the mix, at LWW's level: load height 4.6 / 13.6.
- LWW with the neck's along-to-bending ratio (G): +7.2, with worst groups z 6.1, α 9.1, x 5.9 and ρ 8.9, and load height
  10.9. At ±6° its direction is 0.7 / 3.4 [X, rev].
- The neck's factors raised to 2(1 − λ): +1.0, with worst groups 10.8–12.1.

### 9.6 Open issues, for this model

1. **V's notch factor, first.** The remaining structure is V's (§9.4): it is 5–18 % high, rising with sharpness and
   falling with α. The next test is V's own notch factor against the exact per-station V. Its power law with the
   sharp-corner exponent over-steepens on teeth that never reach the sharp regime. **Do not fix M or N alone:** that
   makes the structure worse (the ablation). The earlier plan to compute per-α sharp-limit coefficients for N and M is
   dropped, because the record's teeth are not in that regime.
2. **The reference point.** A reference-free form is untested. It would carry V's lever moment through the neck's M
   factor and apply W to the pure shear only, about the neck's centre. Until then, Q is a named choice, and the spread
   over references (+2.9 … +7.3 at the median; the ring step +1.6 … −6.3) belongs beside every figure.
3. **The corner map.** Either replace it with a derived blunt-V-notch field, or keep it as a named interpolation with
   its q0.
   - Filippi, Lazzarin and Tovo 2002 (https://doi.org/10.1016/s0020-7683(02)00342-6) [M] give such fields. That their
     opening angle and root radius enter independently is recalled [C].
4. **c.** V's notch length governs the structure (c = x halves the notch structure and reads −10 %). Whether a derived
   c exists is open.
5. **Selection.** Choose among variants on half the record and score on the other half. Until then, the record figures
   are in-sample.
6. **The baseline's domain off L0.** The path column was read on P2's domain rule. Its agreement with root+fillet away
   from HPSTC is unchecked, as is every tooth whose flank maximum exceeds its fillet's there.
7. **Cost.** A better-started θ solve, and a ramp that reuses each station's three responses, are both untried.
8. **The near field** (the components round's D4), which every wrench model shares.
9. **Not tested:**
   - c/ρ < 2, where V's flat limit (0) and N's and M's (the wedge) part;
   - α above 31°, where the reference BEM failed at 32.5°;
   - pointed teeth loaded at the apex;
   - shaper-cut, asymmetric and thin-rim teeth;
   - the ring tip-side reference (F was verified near the corner on externals only).

## 10. Component model, round 2 (2026-09-30; three reviews applied)

For the owner's direction on §9: research the across force's (shear) closed form and the other extensions on this
route; avoid structured errors of substantial size; no hidden fitted constant (derived, or a named visible option
with its source); continuity in every input; common material properties only; the fast mode µs per tooth, with no
per-geometry numerical solve and no cache.
- **Baseline and rule:** as §9 and `work/bending-options.md`: the exact σ_tt peak over root land and fillet; a
  structure is a range in points; S ≥ 5 anywhere measured, *thr* 4–6, m 2–5, n < 2.
- **Marks:** [X] this round; [R] read from the record, `bending-options.md` or §9; [C] recalled. No source was read in
  this round beyond the research round's copy of Berto 2015 (Frattura ed Integrità Strutturale 34:11–26: Eqs. 10 and
  13 and r0 = ρ(π − 2α)/(2π − 2α) read [read]). The literature the round leans on (Neuber, Williams, Filippi–Lazzarin–
  Tovo, Lazzarin–Zappalorto–Berto, Dini–Hills) is [C] or abstract-only, as the research round recorded it.
- **Scripts:** `~/.cache/gearcalc-work/bending-comp2/` (README there). Nothing in the crate changed.
- **Reviews applied** [rev]: three reviews re-scored the holdouts, swept every input finely, and rebuilt the cost. Their
  scripts are in the session scratchpad (`rv/`, `nf.py`, `lim.py`, `w.py`, `sw/`, `w/`). What they changed is marked
  [rev] below; what they left open is §10.8. They read no source beyond search abstracts (Zappalorto–Lazzarin, below).

**Short answer.**
- **Protocol.** The record was split before any model was built: 153 development teeth, 75 record-holdout teeth (a
  fixed rule, §10.1). A fresh holdout of 70 teeth the record does not contain was solved on the record's own solver
  and body. Every choice below was made on the development set only.
- **A reference-free model exists, and it changes nothing.** R2d takes the across force's pure shear about
  the osculating neck's own centre (where Neuber's shear carries no moment) from the tangent wedge × W, and routes its
  lever through the neck's own M. No reference point is chosen; the answer is the same for every one. Its figures are
  QW's within a point on all three sets (worst external group 11.3 / 10.3 / 8.4 on development / record holdout /
  fresh, against QW's 12.1 / 11.2 / 8.4). **It is not "derived"** [rev]: the reference point's arbitrariness moved to
  the map's pivot q0, which moves R2d's median +0.3 … +7.9 (7.6 points, more than the 4.4 the reference point did);
  it rests on three named choices (the map's exponent 2(1 − λ), q0, c = 2x); and none of its three terms is an exact
  solution applied within its assumptions (§10.2).
- **No derived shear law was found, but the route most likely to give one was not tried** [rev]. Every construction
  tried on the development set (§10.3) was worse than QW. The structure is removed only by giving V's notch factor an
  α-free exponent below the corner's 1 − λ(α_n). §10.3 argued the shear route on mode II, which is not singular at the
  rack corner for any α in the sets; V's sharpness sensitivity must come through the inclined corner's mode I, and a
  two-term law (mode I at 1 − λ1, 0.427 · 0.414 · 0.391 at 14.5° · 20° · 28°, plus a bounded remainder) predicts an
  effective slope just below 1 − λ1, which 0.38 is. That decomposition, and the published mode II rounded-notch
  solutions, are untried (§10.8).
- **With that exponent as a named fitted constant (R2f, e_V = 0.38 from the development set):**
  - it holds out of sample (re-scored independently [rev], every figure reproduced): worst α, x and ρ groups 2.8 /
    3.6 / 5.6 on the record holdout and 2.6 / 3.4 / 2.3 on the fresh externals, against QW's 8.0 / 8.2 / 11.2 and
    8.4 / 5.8 / 3.7; e_V = 0.38 is also the best value on both of those holdouts, but not on the fresh rings (best
    ≥ 0.44, where the worst α group is still 9.0; the development rings regress at 0.401);
  - reversals of pairs ≥ 5 % apart: 0 of 9,931 (development), 0 of 2,402 (record holdout), **10 of 1,886** on the
    fresh set with its four duplicate 28° ring teeth counted once [rev] (20 of 2,111 as first reported, which counted
    each duplicate against every other tooth twice); QW 17, 5 and 14 of 1,886;
  - it is still substantial on **z at tight tools** (8.8 and 11.1 on the record's two sets, at ρ_fP 0.01 across
    z 12 → 1000; measured at α 14.5–20° only: at 28°, ρ_fP 0.01, z 20 / 100 / 1000 reads −0.5 / −2.3 / −3.6, a range
    of 3.1 [rev]), on **direction at ±14°** (8.4 / 7.6 / 5.6), on **α above about 26–27° on both kinds** [rev, below],
    and at the threshold on ρ (5.1 / 5.6) and the tight-tool shift sweep at 14.5° (5.6);
  - **its level is a signed bias** (rule 6): median −0.7 on every set, unconservative on 61–77 % of teeth; it moves
    −4.1 … +3.7 with the map's pivot q0 = 0.25 … 4 (§10.5) and about 1.5 points per 0.01 of e_V [rev].
- **α at the tool limit** [rev]. α structure of 5 points or more returns on rings and on externals once α passes
  about 26–27°, and on rings it begins inside the record's range: on ring z 50, shift 0.25, ρ_fP 0.05, mate 21, R2f
  reads −1.1 · −1.8 · −2.4 · −3.5 · −5.4 · −6.2 · −7.1 · −8.3 · −9.7 at α 22.5 · 24 · 25 · 26 · 27 · 27.25 · 27.5 ·
  27.75 · 28° (QW +4.7 → −7.1); on external z 20, shift 0.25, ρ_fP 0.05, +1.8 · +1.0 · −0.8 · −2.2 · −5.5 at 17.5 ·
  22.5 · 28 · 30 · 32° (range 7.3, 4.7 of it from 28° to 32°). Above about 27° the cutter's clamp shrinks the fillet
  (ring ρ_f 0.110 → 0.072; external 0.132 → 0.097) and the exact peak climbs; past about 28.1–28.5° a ring does not
  build at all. So the fresh 28° rings are the end of a smooth trend near the cutting limit, not a separate structure,
  and the options sweep "α 25 → 31°, 1.8" misses it because it is one tooth line. No model here follows it.
- **The load near the root** (fresh set; every wrench model): with the single-pair zone near the root (28°, shift
  0.75; rings) load-position ranges reach 15–17 on externals and 20–21 on rings, and the lowest point −35 … −48. On the
  rings part of R2's figure there is an artefact of the 90° guard (next bullet), so these are not yet clean.
- **The 90° guard** [rev]. Near a tangent angle of 90° (ring root lands, a ring's lowest loads, the space centre at
  large z) R2's per-station stress rises with unbounded slope to about LWW's value at 90°, and the maximum sits on that
  rise. Consequences: jumps of 7.3–7.6 % between load points 6e-5 of the flank apart (ring 120 / 22.5° / 0.6 / 0.15);
  a peak that depends on the station count (1.0–2.1 % between 2,001 and 200,001 stations at the fresh rings' lowest
  point); a rating that does not converge as z → ∞ (+1.6 % per decade of z at the space centre, external 28° / 0.75 /
  0.18 at the lowest load, where LWW converges); and 24 of 3,102 fresh tooth-loads (18 rings, 6 externals, 22 at the
  lowest point) rated on it, lifting the ring ratings 14–23 % above the best value below 88°. This fails the continuity
  rule; QW shares it, LWW does not. The L0 tables are unaffected (their maxima lie inside the fillet).
- **The near-field term is withdrawn** [rev]. Flamant's field was used outside its domain (the hot spot lies outside
  the flank's tangent half-plane at the lowest load on all 70 teeth, and every ring station at every load), it counts
  the load twice on top of the wrench response, and it leaves the fillet loaded. Its figures (lowest point −22.7 →
  −19.3, direction 8.4 → 12.1) say nothing either way; a self-equilibrated version is untested (§10.8).
- **Cost:** per section at one load R2 is QW's cost, 9.3–10.2× LWW (wasm32 125–136 µs; reproduced at 98–117 µs
  against LWW's 11–12.6 µs [rev]). **The crate rates a section at about 160 loads under LinearRamp**, which extrapolates
  to about 16–19 ms per section in wasm [rev, not measured], against LWW's ≈ 1.5 ms and today's 0.9–3.9 ms per preset
  solve: tens of milliseconds for a spur pair, doubled on a hover's dry run. That misses "µs per tooth" unless each
  station's response is reused across loads, which needs a fixed station set and so the 90° guard fixed first. Each
  station's θ is a numerical root find, which the owner may rule on against "no per-geometry numerical solve". Rust
  reproduces the Python to 8.4e-7 **at L0 only**; at the fresh rings' lowest point they differ 1.0–2.1 % (the guard).

### 10.1 Protocol [X]

- **The split, made before any model** (`split.py`): within each fillet class (ordinary, middle, tight, ring), the
  record's teeth sorted by (kind, z, α, x, −ρ_fP); every third (index mod 3 = 2) is the record holdout (28 · 22 · 9 ·
  16 = 75), the rest development (153).
- **What is not clean.** The research round that preceded this one regressed per-component factors on all 228 teeth,
  and its finding (an α-free V exponent near 0.37) suggested R2f's form. The record holdout is therefore clean for the
  value 0.38 (scanned on development only) but not for the idea. The fresh holdout is clean for both.
- **The fresh holdout** (`gen_fresh.py`): 70 teeth off every record level.
  - External z 20, 45, 300 × α 17.5°, 22.5°, 28° × shift −0.2, 0.25, 0.75 × ρ_fP 0.05, 0.18, mate 25 (54).
  - Ring z 50, 120 × α 22.5°, 28° × shift 0.25, 0.6 × ρ_fP 0.05, 0.15, mate 21 (16). At 28° the two ring tool radii
    cut the same fillet, so four pairs are identical teeth. A twin never pairs ≥ 5 % apart with its twin, but each
    duplicate counts again against every other tooth: the pair counts below are taken with each counted once (66
    distinct teeth), and the duplicates also padded the fresh ring ρ groups (first read 0 of 35) [rev].
  - Each tooth solved as the record is: `fillet_bem.solve_tooth`'s three passes (last-pass change median 0.010 %,
    max 0.052 %), body by the record's rule (18 whole gears, 52 sectors). Then one multi-load solve on the same class
    (`frame-critique/bemx`, pass-B mesh at that peak): the record run's seven loads, and the path: five points LPSTC →
    HPSTC and the lowest point (d = ε − 0.03), each along the involute normal at the crate's own load points (`optrs`).
    The sampled L0 peak is within 0.05 % of the three-pass peak on all 70.
- **The evaluator** (`ev.py`, `summ.py`): every model rated on the exact outline's root+fillet stations, **less those
  at tangent angle a ≥ 90°** (`ev.sigma`'s mask, an evaluation guard the baseline does not have [rev]); exact from the
  record or the fresh solve; turned loads by superposition of the unit loads at the load point. Through it, QW
  reproduces its options-table row exactly (median +3.1, 30 reversals of 22,346, groups 9.9 / 8.1 / 10.6 / 12.1, the
  sweeps 3.5 · 3.7 · 8.1 and 10.8 · 7.6 · 3.8).
- **Group definitions** below count both kinds (groups of teeth differing in one variable alone, ≥ 2 members, the five
  tip-corner teeth left out), so N differs from the options table's external-only brackets; its brackets for R2 are
  given in that file.

### 10.2 The model built: R2 [X]

At each fillet station, with the osculating neck at the corner-mapped sharpness (§9.4: q_eff = q0 (q/q0)^(2(1−λ)),
q = x/ρ, q0 = 1) of centre O, dy_c above the station:

    σ = F_V · V · s_V,pure^wedge(O) + N · t_N^neck + M_O · t_M^neck
    s_V,pure^wedge(O) = s_V^wedge(Q) + dy_c · s_M^wedge(Q)        (the tangent wedge's across response about O)
    F_V = (2x/ρ)^e_V

- **No reference point.** The load enters as (V, N, M_O), M_O its moment about O, and O is a property of the station's
  fitted neck (where Neuber's shear solution carries no moment, §9.2). Any other reference gives the same σ exactly:
  the lever between it and O is carried by the neck's own t_M, so the §9.4 dependence (+2.9 … +7.3 at the median) is
  gone — **traded for the pivot q0** [rev]: O's height dy_c comes from the *mapped* neck, so it depends on the map's
  exponent and on q0, and R2d's median moves +0.3 … +7.9 over q0 = 0.25 … 4, a larger spread than the reference point's
  4.4. This is the "neck-centre consistent hybrid" of the research round, built. (The algebra and the sign of dy_c
  were checked against `hyp.comps_their` [rev].)
- **What each term is** [rev]. None is an exact solution within its assumptions. N and M: Neuber's deep neck fitted at
  a curvature that is not the station's own (the corner map), an interpolation device. V: LWW's semi-empirical factor
  (2x/ρ)^e_V on Carothers' wedge, c = 2x named. **Their blunt limits disagree**: as ρ → ∞ the neck tends to the
  notch-free wedge (factor 1) but V's factor tends to 0, so the V term vanishes on a flat root land or a convex station.
  At every fresh hot spot 2x/ρ is 3.4–62, so the peaks are not affected; continuity is.
- **R2d:** e_V = 1 − λ(α_n), LWW's own exponent. Reference-free, with three named choices (the map's exponent, q0 = 1,
  c = 2x for V); not "derived".
- **R2f:** e_V = 0.38, α-free. **A fitted constant**, named here with its source: the value minimising the sum of the
  four worst record groups on the development set (`scan_ev.py`, 0.30 … 0.45 in steps of 0.01; 0.38 → 19.8, 0.37 →
  22.6, 0.39 → 21.3). The research round's at-peak regression on all 228 gave 0.37; on development alone, V about the
  neck's centre regresses at 0.369 (externals) and 0.401 (rings). **e_V and q0 are one fitted pair** (fitted at q0 = 1,
  c = 2x and the λ-map); the objective is a sum of maxima with a sharp minimum, so the value is noisy at ±0.01, and the
  level moves about 1.5 points per 0.01 [rev]. On the holdouts (sum of the four worst groups at e_V 0.35 / 0.38 / 0.41 /
  0.44): record 29.3 / **23.0** / 27.5 / 37.0, fresh externals 18.3 / **12.6** / 13.9 / 25.1, fresh rings 25.2 / 20.2 /
  17.4 / **15.8** [rev]: the value carries to externals, not to rings.
- **Continuity.** e_V is one number and the map is continuous in α. **Not continuous in load position or z near
  a = 90°** [rev]: the guard at a ≥ 90° (§9.4's) rates up to 90° and drops past it, and R2 rises with unbounded slope
  toward 90° (§10.4, the 90° guard). §9.5's "no maximum lies near one" is false on the fresh rings.
- **Implementation.** `r2rs/src/lib.rs` (`r2_at`, `r2_peak`): QW's scan with M_O and the pure-shear term; θ by
  gear-core's Brent or a safeguarded Newton (`neck_theta_fast`, equal to Brent within 9.3e-13; its comment says it
  starts from the flat limit, the code starts at the bracket's midpoint [rev]).

### 10.3 What was tried on the development set, and dropped [X]

Each scored on the 153 (worst groups z / α / x / ρ, median):
- **One α-free exponent for everything** (the wedge × (2x/ρ)^0.456, the square corner's 1 − λ, which is what the
  per-component slopes of M and N regress to): +20.9, ρ groups 23.3. The M and N regressions have non-zero intercepts
  at c = 2x (−0.12 and +0.11 in ln), so the exponent alone does not carry them.
- **V about the tangent wedge's apex** (Carothers' force at the apex, whose factor has the tightest law at the peak:
  slope 0.40, sd 0.021 in ln), its lever through the neck's M: diverges to +18,000 % at the top of the fillet, where
  the apex runs off to infinity and any difference between V's and M's factors is multiplied by x cot a.
- **The neck for all three components** (one body, reference-free): +78 %, maximum at the root end. At the exact peak
  its V has the right sharpness law (log-slope of neck/exact on ln(2x/ρ) +0.004; level 0.87 … 0.92 by class), but
  along the fillet neck/exact V grows with the station's place past Neuber's shear peak, s = sinh ξ / cos θ: ≈ 0.9 at
  s = −1, 1.4–2.1 at −4, 2.1–2.5 at −6 … −9, 5.8 at −33. There the osculating neck is a crack whose tip nearly
  reaches the station, and its mode-II field is singular where the rack corner's is not (λ2 > 1, research round).
- **A per-tooth V factor read at the intrinsic station s = −1:** −9.8, tight −17.8.
- **V exponents above LWW's** (the square corner's 0.456 or the crack's ½, on the pure shear about O): worse at every
  step (ρ groups 15.9 and 30.2).
- **The mode argued** [rev]. The "crack's mode II is singular where the rack corner's is not" reasoning above is
  right, but it was used to set aside the shear route as a whole, which it does not do: no Williams mode II root
  λ2 < 1 exists for void openings 104.5–118° (α 14.5–28°), so V's measured sharpness sensitivity (0.37–0.40) must
  come from V's projection onto the inclined corner's **mode I** (1 − λ1 = 0.427 · 0.414 · 0.391 at 14.5° · 20° · 28°).
  A two-term law w_I(α)(2x/ρ)^(1−λ1) plus a bounded remainder has an effective slope below 1 − λ1 that varies with
  geometry, which is what the fitted 0.38 looks like. Only single exponents and the neck were tried, so "no derived
  shear law exists" is not shown.
- **A lead not built.** Neuber's own deep-notch factors, as recalled [C], have transitional log-slopes in q = a/ρ over
  q 5 … 100 of 0.47–0.50 (tension), 0.41–0.49 (bending) and 0.23–0.42 (shear): the shear law is the shallower one, as
  the exact V is (0.37 against 0.45). It was not usable as V's factor because its nominal, V/(2a), makes the blunt
  limit diverge (K → 3/(2t)); a nominal that makes it tend to 1 was not found.

### 10.4 Results on the three sets [X]

Figures: median (min … max) of L0; worst record group per variable, in points, [groups ≥ 5 of N]; direction and
height as median / max; pairs = reversals of pairs ≥ 5 % apart and the misstatement's p90. "R2d" and "R2f" are without
the near-field term. The fresh rows' pair counts and ring ρ groups were taken with the four duplicate teeth counted
twice: corrected counts R2f 10, QW 14 of 1,886 [rev]; the others were not recounted. Ring ρ is not tested at all on the
record holdout (it has no ring ρ group) and on the fresh set only by the 22.5° teeth.

| Set | Model | L0 | Classes ord · mid · tight · ring | z | α | x | ρ | Direction ±6° · ±14° | Height | Pairs: reversed · p90 |
|---|---|---|---|---|---|---|---|---|---|---|
| Dev (153) | LWW | +10.0 (+1.9 … +18.6) | +9.3 · +13.0 · +11.7 · +6.2 | 7.5 [5/38] | 7.6 [6/60] | 7.1 [7/55] | 8.0 [13/51] | 1.3/4.0 · 3.3/10.7 | 2.3/10.9 | 16 of 9,931 · 8.0 |
| | QW | +3.1 (−0.7 … +14.4) | +2.1 · +5.3 · +5.6 · +3.4 | 9.9 [5/38] | 7.8 [6/60] | 9.0 [2/55] | 12.1 [17/51] | 1.8/4.5 · 4.5/11.7 | 1.1/4.6 | 17 · 6.9 |
| | R2d | +3.5 (+0.1 … +14.4) | +2.7 · +5.5 · +5.7 · +3.4 | 9.4 [5/38] | 7.8 [6/60] | 8.6 [2/55] | 11.3 [15/51] | 1.7/4.4 · 4.1/11.6 | 1.1/4.6 | 13 · 6.6 |
| | R2f | −0.7 (−5.9 … +3.3) | −0.7 · +0.4 · −2.6 · −0.8 | 8.8 [7/38] | 2.6 [0/60] | 3.3 [0/55] | 5.1 [2/51] | 0.8/3.1 · 2.2/8.4 | 1.1/4.8 | 0 · 3.9 |
| Record holdout (75) | LWW | +10.4 (+0.3 … +19.1) | +10.1 · +12.8 · +15.4 · +4.4 | 6.9 [3/24] | 6.9 [4/20] | 7.2 [1/18] | 8.5 [3/21] | 1.3/3.5 · 3.4/9.5 | 1.9/10.9 | 14 of 2,402 · 9.3 |
| | QW | +3.5 (−0.5 … +13.3) | +1.9 · +5.4 · +5.5 · +4.9 | 6.6 [1/24] | 8.0 [6/20] | 8.2 [1/18] | 11.2 [6/21] | 1.7/4.3 · 4.2/10.9 | 1.0/5.0 | 5 · 6.7 |
| | R2d | +3.9 (+0.1 … +13.3) | +2.6 · +5.5 · +5.6 · +4.8 | 6.5 [1/24] | 8.0 [6/20] | 7.7 [1/18] | 10.3 [5/21] | 1.6/4.2 · 3.7/10.9 | 1.0/5.0 | 5 · 6.3 |
| | R2f | −0.7 (−7.0 … +4.1) | −0.8 · +0.8 · −2.9 · −0.6 | **11.1** [2/24] | 2.8 [0/20] | 3.6 [0/18] | 5.6 [1/21] | 0.7/2.8 · 1.6/7.6 | 0.9/5.1 | 0 · 4.1 |
| Fresh (70) | LWW | +9.9 (−10.7 … +17.9) | +7.7 · +13.2 · +12.8 · +0.2 | 7.0 [5/26] | 19.2 [16/26]; ext 7.9 | 10.1 [10/26] | 3.1 [0/35] | 1.3/2.8 · 3.5/7.6 | 2.4/8.8 | 65 of 2,111 · 19.2 |
| | QW | +3.3 (−9.3 … +8.2) | +1.7 · +3.9 · +0.7 · +0.1 | 5.4 [3/26] | 14.7 [16/26]; ext 8.4 | 5.8 [1/26] | 3.7 [0/35] | 1.7/3.7 · 4.4/9.9 | 0.9/3.2 | 25 · 9.4 |
| | R2d | +3.5 (−9.3 … +8.3) | +2.2 · +4.3 · +0.9 · −0.0 | 5.3 [2/26] | 14.7 [16/26]; ext 8.4 | 5.4 [1/26] | 3.3 [0/35] | 1.6/3.7 · 4.1/9.8 | 0.9/3.1 | 24 · 9.9 |
| | R2f | −0.7 (−12.0 … +1.9) | −0.1 · −0.6 · −2.1 · −3.3 | 5.9 [2/26]; ext 4.3 | 11.6 [7/26]; ext 2.6 | 3.4 [0/26] | 2.3 [0/35] | 1.0/2.1 · 2.5/5.6 | 0.9/2.7 | 20 · 8.4 |

**The path, fresh set only** (range over the five single-pair points LPSTC → HPSTC, median / max; the lowest point
min / median / max):

| Model | Externals (54) | Rings (16) | Lowest point, externals | Lowest point, rings |
|---|---|---|---|---|
| LWW | 8.1 / 26.4 | 10.9 / 13.9 | −34.5 / −16.4 / +4.9 | −44.1 / −35.6 / −23.8 |
| QW | 2.6 / 15.6 | 11.4 / 20.4 | −40.7 / −17.9 / +2.6 | −45.2 / −36.6 / −27.1 |
| R2d | 2.8 / 16.4 | 11.5 / 20.4 | −40.9 / −18.0 / +2.4 | −45.2 / −36.6 / −27.1 |
| R2f | 4.1 / 17.4 | 12.7 / 21.1 | −43.2 / −25.0 / −1.2 | −47.6 / −39.9 / −33.9 |

The near-field rows first printed here are withdrawn [rev]: the term was Flamant's field outside its half-plane,
added on top of the same force's wrench response, and not traction-free on the fillet (§10.8). **The ring columns and
the lowest point include the 90° guard's rise** [rev]: R2f's maximum sits at a > 88° on 24 of 3,102 fresh tooth-loads
(18 rings, 22 at the lowest point), lifting the ring ratings 14–23 % above the best value below 88° (LWW's lift ≤ 1.7 %),
and the 16-sample load scan reads up to 6.5 % below the model's own dense maximum on ring 120 / 22.5° / 0.6 / 0.15.
These rows are to be re-scored once the guard is fixed.

- **Where the worst sit.**
  - R2f's z groups: tight tools (ρ_fP 0.01), shift 0.5, z 12 → 1000 (the record holdout's 11.1 at 14.5°, the
    development set's 8.8 at 20°). Its error runs about +3 at z 30 and −5 at z 1000: the exact peak's rise at large z
    with a sharp tool, which no local law here follows.
  - QW's and R2d's worst ρ, x and α groups: 14.5° with tight tools, as in §9.5.
  - On the fresh set every model's worst α and position groups are the 28° rings. Their exact peak rises 13–33 % from
    22.5° to 28° (z 50, shift 0.25, ρ_fP 0.05: 2.68 → 3.03) while every model's error drops to −7 … −12. The peak
    moves to tangent angle 70–78° (2x/ρ 22–62), near the root end, where the mapped neck's θ goes to 90°, the crack-like
    limit §10.3 finds wrong for V: a kernel used out of range. A continuous α series [rev] shows this is the end of a
    trend that starts by 26–27° on both kinds as the cutter's clamp shrinks the fillet (the short answer's series).
  - The externals' worst load-position ranges (15–17) are 28°, shift 0.75: the LPSTC sits 0.6–0.8 module from the
    fillet's top and the exact peak rises toward it; no wrench model follows that.
- **All pairs over the whole record** (for the options table's column): R2f 1 reversal of 22,346, p90 3.9; R2d 25,
  p90 6.5; QW 30, p90 6.8.
- **Fine sweeps that survive** [rev]: 69 sweeps (externals ρ_fP 0.002–0.45, x −0.6 … 1.0 through the undercut onset at
  z 12 and 17, α 10–35°; rings α 12–32°, ρ_fP, x) show no jump from the model at the crate's load (second-difference
  excess ≤ 0.25 %; 16-sample scan = 4,001-point scan within 1e-4 %). The kinks seen are the crate's geometry and every
  model shares them: the rack's tip width reaching zero at α 32.14° (+10 % in every model), the ring cutter's clamp at
  24.9° and 27.5°, external x ≈ 0.99 (≈ 0.2 %, cause not checked). Mate on the 28° ring (15 / 21 / 30 / 40) ranges 0.6;
  shift −0.5 at 17.5° reads +1.6 and +0.0. At the fillet–root-land junction every wrench model, LWW too, drops 58–77 %
  between adjacent stations (ρ finite → ∞); it is never the maximum in the three sets.

### 10.5 Sweeps, constants, and the pivot [X]

**The options table's sweeps** (out of sample for every choice in this round; range in points, and the share of the
exact change between the ends the model credits):

| Sweep | Exact | QW | R2d | R2f |
|---|---|---|---|---|
| Shift, ρ_fP 0.25, 20° | −21.5 % | 3.5 (87 %) | 4.0 (86 %) | 1.3 (95 %) |
| Shift, ρ_fP 0.03, 20° | +47.3 % | 3.7 (104 %) | 3.7 (104 %) | 4.4 (89 %) |
| Shift, ρ_fP 0.03, 14.5° | +14.2 % | 8.1 (142 %) | 7.9 (140 %) | 5.6 (69 %) |
| Teeth z 12 → 40, ρ_fP 0.25 (29 points) | −36.2 % | 2.6 (95 %) | 2.5 (96 %) | 0.2 (100 %) |
| Mate: ρ_fP 0.25 · 0.03 · ring | | 0.9 · 2.7 · 0.8 | 0.7 · 2.6 · 0.9 | 1.2 · 1.0 · 1.3 |
| α 14.5 → 25° · 25 → 31° | −20.9 % · +12.5 % | 1.2 · 3.3 | 1.3 · 3.7 | 0.8 · 1.8 |
| Notch at 14.5° · 20° · 25° | −46.2 · −46.5 · −41.3 % | 10.8 · 7.6 · 3.8 | 10.1 · 7.0 · 3.3 | 4.8 · 3.0 · 1.2 |
| Ring vs external across the rack, ρ_fP 0.2 · 0.1 · 0.03 | | +0.1 · +0.7 · +0.7 | −0.5 · +0.4 · +0.6 | −0.0 · +1.2 · +2.4 |

**Constants.**

| Constant | R2d | R2f | Status |
|---|---|---|---|
| Neuber's neck in N and M, fitted at the corner-mapped curvature | yes | yes | neck + named map (not the elastic solution for this geometry) [rev] |
| V's factor (2x/ρ)^e_V on Carothers' wedge | yes | yes | semi-empirical (LWW); tends to 0, not 1, as ρ → ∞ [rev] |
| The wedge's Flamant and Carothers kernels; λ (Williams) | yes | yes | derived |
| O, the reference for the load's moment | the mapped neck's centre | the same | follows from the map and q0 (the point where Neuber's shear carries no moment) |
| The corner map's exponent 2(1 − λ(α_n)) and pivot q0 = 1 | yes | yes | named choices (§9.4) |
| V's notch length c = 2x | yes | yes | named choice (LWW's) |
| V's exponent | 1 − λ(α_n) | **0.38** | LWW's · **fitted** on the development set (`scan_ev.py`), as one pair with q0 = 1 |
| The 90° guard (stations at a ≥ 90° dropped) | yes | yes | an unstated evaluation choice; R2 discontinuous at it [rev] |
| E, ν | — | — | none: a traction problem on a simply connected body |

**The pivot, R2f on the development set** (`q0dev.py`): q0 = 0.25 / 0.5 / 1 / 2 / 4 reads −4.1 / −2.5 / −0.7 / +1.5 /
+3.7 at the median, with worst ρ groups 6.8 / 5.9 / 5.1 / 5.9 / 6.6 and z 8.7–9.1. e_V was fitted at q0 = 1, so the
two are not independent: a different q0 would have fitted a different e_V. R2d: +0.3 / +3.5 / +7.9 at q0 = 0.25 / 1 / 4
(a 7.6-point swing, larger than the reference point's it replaced); **R2d's group structure against q0 was not
measured** (§10.8).

### 10.6 Calls, by the one rule

| | Position | Height | Direction | Shift | Teeth | Mate | α | Notch | Ring vs external |
|---|---|---|---|---|---|---|---|---|---|
| R2d | **S** (fresh 16.4; rings 20.4) | *thr* (5.0) | S at ±14° (11.6), m at ±6° | S (7.9 at 14.5°; groups 10.1) | S (groups 9.4) | m | S (groups 8.1; fresh rings 14.7) | S (10.1) | n |
| R2f | **S** (fresh 17.4; rings 21.1, guard-affected) | *thr* (5.1) | S at ±14° (8.4), m at ±6° (3.1) | *thr* (5.6 at 14.5°); groups m (3.7) | **S** (groups 11.1; ρ_fP 0.01 at α 14.5–20° only; 3.1 at 28°) | n (1.3) | **S** over α ≳ 26–27° on both kinds (ring 22.5 → 28°: 8.6; external 17.5 → 32°: 7.3), following the fillet radius's fall at the tool limit [rev]; m below 26° on the record groups (2.6–2.8) | *thr* (4.8 at 14.5°; groups 5.6) | m (2.4) |

Both rows also fail continuity in load position and z near tangent angle 90° (§10.4) [rev]. R2f takes the α and x
structure QW keeps at 14.5° with tight tools below the threshold, and the shift and notch structure there to it (5.6,
4.8). It keeps the z structure at tight tools (measured at 14.5–20°), direction at ±14°, α near the cutting limit on
both kinds, and the load near the root, which every wrench model here shares.

### 10.7 Cost [X]

Measured on the 228 in one build (`r2wasm/`, a copy of the editor's `neckwasm` with round 2 added), best of 7, at one
load; the machine was loaded by other builds, so compare ratios, not the editor's absolute figures:

| Build | LWW | QW | R2 (Brent) | R2 (safeguarded Newton) |
|---|---|---|---|---|
| wasm32 under node | 13.0–13.5 µs | 134 µs (10.0–10.4×) | 132–136 µs (10.1–10.2×) | 125–127 µs (9.3–9.8×) |
| native | 9.3–10.1 µs | 101–107 µs | 108–113 µs | 95–96 µs |

- Rust = Python to 8.4e-7 on all 228 (R2d and R2f) **at L0**; at the fresh rings' lowest-point loads Python (2,001
  stations), a 200,001-station scan and Rust differ by 1.0–2.1 % (e.g. ring 50 / 28° / 0.6 / 0.05: 1.6831 · 1.7150 ·
  1.7183), the 90° guard's rise [rev]. Newton = Brent to 9.3e-13.
- **Reproduced and re-based** [rev]: wasm32 at opt-level 3, "z" and "z" + `wasm-opt -Oz`: R2 98–117 µs per section
  against LWW's 11–12.6 µs (7.8–11.4×); native 80–92 µs against 8.3 µs. These are **per load**. The crate rates a
  section at about 160 loads under LinearRamp (§6.4): about 16–19 ms per section in wasm (extrapolated, not
  measured), against LWW's ≈ 1.5 ms and today's 0.9–3.9 ms per preset solve. That misses the brief's "µs per tooth".
- The θ solve dominates, and it is a numerical root find at every station — for the owner to rule on against "no
  per-geometry numerical solve". A start from a fixed table in (a, ln q), a closed form's inverse rather than a
  per-geometry cache, would cut it to one or two Newton steps; not built. Reusing each station's linear response across
  the ramp's loads would remove the ×160, but needs a fixed station set, on which the 90° guard makes the answer depend
  on the grid; the two fixes go together.

### 10.8 Open issues, for this route

1. **The 90° guard** [rev]. Derive the neck's limit as a → 90°, or rate up to a stated angle with a continuous blend;
   then make the peak search find the endpoint limit and every local maximum, add laws that the rating moves
   continuously along fine load sweeps (turns 0° and ±14°, rings included) and that the scan equals a dense maximum,
   test z → ∞ at low loads (converge to the rack, or state the bias's size and sign), and re-score the fresh path table,
   the ring α groups and the reversals, saying how much each moved.
2. **A derived V law.** R2f's α-free 0.38 is the only thing that removes the α, x and shift structure, and it is
   fitted (with q0, as one pair). Untried leads [rev]: the mixed-mode decomposition of V at the inclined rack corner
   (mode I singular at 1 − λ1, mode II non-singular for α > 12.6°: w_I(α)(2x/ρ)^(1−λ1) plus a bounded remainder); the
   published mode II rounded-notch fields (Zappalorto & Lazzarin 2011, Int J Fract, V-notches with end holes,
   doi 10.1007/s10704-010-9567-5, and GSIFs for rounded notches under in-plane shear, doi 10.1007/s10704-011-9613-y;
   Procedia Eng. 2011, S1877705811003729 — abstracts only, whose exponents combine Williams' mode I and II eigenvalues);
   Neuber's shear factor with a nominal whose blunt limit is 1 (§10.3).
3. **V's blunt limit** [rev]. V's factor tends to 0 as ρ → ∞ while N's and M's tend to 1; fix it to tend to 1 so the
   model is continuous onto a flat root land and convex stations.
4. **α at the cutting limit** [rev]. Structure ≥ 5 from α ≈ 26–27° on both kinds, following the clamped fillet radius;
   add a sweep continuous in α up to where the cutter stops producing a tooth, per kind, and find a term that follows
   the clamped fillet. On rings the best e_V is ≥ 0.44 at the fresh teeth, in line with the 0.401 regression.
5. **z at tight tools.** R2f's worst groups (8.8, 11.1): the exact peak rises from z 150 to 1000 with ρ_fP 0.01; no
   local law here follows it. Measured at α 14.5–20° only (3.1 at 28°). The research round saw the same in V's residual
   (−4 … +4 over z 12 → 1000).
6. **The root end.** Rings near the cutting limit and every load near the root put the exact peak low on the fillet
   (a 70–78°), where the mapped neck runs to its crack limit and the exact pole stays bounded (research round). A
   bounded body for those stations is the next piece of mechanics this model lacks.
7. **The near field** [rev]. The first term is withdrawn. If kept, rebuild it self-equilibrated (Flamant's field, or the
   wedge whose apex angle is the flank's local curvature, minus the response to its own resultant), applied only to
   stations inside its domain, then re-test the lowest point and direction. The load within a module of the fillet
   remains −35 … −48 for every wrench model; how to fix it is untested.
8. **The pivot.** q0 moves R2f's level ±4 points and R2d's 7.6; R2d's group structure against q0 was not measured. The
   derived pivot through Filippi–Lazzarin–Tovo's tip relation needs ω̃1 from the paper, which was not read.
9. **Cost.** Under the ramp R2 is ≈ 16–19 ms per section in wasm (extrapolated); measure it, and either build the
   linear reuse with item 1 or carry the ≈ 10× slower solve in the options table. The per-station θ root find awaits
   the owner's ruling.
10. **Direction at ±14°** stays substantial for every variant (5.6–12.1).
11. **Bias record.** R2f's median −0.7 and its 61–77 % unconservative share belong in `docs/state.md` under rule 6 if
   it is adopted.

## 11. Round 3: two prototypes, scored as round 2 (2026-10-01; three reviews applied)

For the owner's direction on §10: continue the component method where it was left; optimise it by conventional means,
with an internal benchmark of where its time goes; look for solutions outside notch mechanics. Three tracks ran
(`~/.cache/gearcalc-work/bending-r3/`: `method/r4/`, `profile/`, `other-fields/`, READMEs there), each with a
skeptic's review. This round took the two most promising candidates and built each:
- **A. R4+NF**, the component method as the method track left it, with the profile track's optimisations, in Rust.
- **B. A shipped table**, from the other-fields track: a tensor Chebyshev interpolant of the exact peak over the
  design box, built offline on the record's own solver, used as a correction on a mechanics model (a control variate).

Both are scored exactly as round 2, with `bending-comp2/summ.py` on the fixed split: dev 153, record holdout 75, fresh
holdout 70. The record teeth carry the path loads re-solved in `method/r4` (`recpath.jsonl`), so the record holdout
has position and lowest-point figures too. Scripts: `~/.cache/gearcalc-work/bending-r3/proto/` (README there). The
repository was not edited.
- **Reviews.** Three reviews followed (§11.4 has what each measured and where its scripts are). They confirm the
  scores below where the scoring sets sample. They refute four of this round's claims as first written: that the
  table removes every structured error, that direction is exact, that the rating is continuous, and that the z 27 → 30
  step is ≤ 1 %. Each claim below is corrected in place, marked [rev].
- **Marks:** [X] computed in this round; [R] read from the method, profile or other-fields tracks or their reviews;
  [C] recalled.
- **Sources:** none was read in this round. The tracks' sources are as they recorded them: Papkovich–Fadle roots
  computed, not read; Brent 1973 ch. 5 recalled; the Chebyshev and multifidelity literature read as abstracts
  (Barthelmann–Novak–Ritter 2000; Chkifa–Cohen–Schwab 2014; Hashemi–Trefethen 2017; Peherstorfer–Willcox–Gunzburger
  2018).

**Short answer.**
- **A, R4+NF, holds out as the method track reported, now in Rust, continuous, and fast.**
  - **Agreement and search.** Rust equals the Python model to 4.5e-5 at every one of 4,446 scoring loads. Its 16-station
    search equals a 4,001-station dense maximum to 1.6e-10 on all of them. On a 160-load ramp over 228 teeth, 2 of
    9,120 loads differ, by up to −2.4e-4; at 24 stations none does. The profile track found 1,091 misses of up to
    −7.5 %; with the guard gone and the 90° crossing made a candidate, they are gone.
  - **Cost.** wasm32: 4.4–4.9 µs per section at one load, against LWW's 8.3–9.0 µs in the same module, and
    214–222 µs per section under the ramp.
  - **What it keeps.** On the scoring sets it is still substantial in six places: the load's position (7.8–12.8), z at
    tight tools (8.6 / 10.9), α at the cutting limit (fresh 11.8), direction at ±14° (7.7–8.4), ρ at the threshold
    (5.2 / 5.4), and the lowest point's spread (−12 … +18). It reverses 20 of 2,111 fresh pairs.
  - **Off the scoring sets it is worse** [rev]. Ring α sweeps range 14.6 points at L0 and 27.7 at the lowest point; ρ
    ranges 7.3 at the lowest point (beyond the threshold, not at it); near the root end (t ≈ 0.07) over the table's
    nodes its error has p90 13 / 29 / 43 % and max 48 / 51 / 119 % (whole / sector / ring). This matters because it is
    the table's fallback.
- **B, the table on R4+NF (Tab(R4)), removes every structured error the scoring sets measure, and not every one
  there is** [rev]. It uses degree 4: 625 BEM solves per patch, three patches.
  - **Level.** L0 is −1.7 … +0.9 on every set, median −0.0.
  - **Groups.** Every z, α, x and ρ group is ≤ 2.2.
  - **The load.** Direction ≤ 2.7, height ≤ 3.6, position ≤ 5.0 (one dev ring, at the threshold) and the lowest
    point −4.1 … +5.5.
  - **Pairs.** No reversal in any set, with a worst misstatement of 2.5.
  - **Cost.** About 55 µs per geometry in wasm (unoptimised) plus 0.44 µs per load, on top of R4+NF.
  - **What it is.** A shipped 2.9 MB (f64) interpolant of the named baseline. It carries an empirical, not a
    certified, error bound. It holds no model constant, but it explains nothing, and it builds in the baseline's
    body, rack (dedendum 1.25, no thickness change), ring cutter and ν = 0.3 [rev].
  - **What the reviews found off the scoring sets** [rev]:
    - **Between direction nodes** the error reaches 17–33 % near the root end (t < 0.1) and 15–21 % mid-flank at
      ψ 45–53°: the exact peak is a maximum over fillet locations with kinks in ψ, which a degree-14 polynomial cannot
      follow. Substantial in direction and in position under the one rule.
    - **Near the root on rings** t is not converged (leave-one-out at the t nodes 26–63 %); an off-node ring α sweep
      ranges 5.3 at the lowest point, and one off-node ring reads −8.3 there.
    - **It is not continuous as a rating.** Leaving the box, or z 28–29 between the external patches, steps the
      rating by the table's whole correction: up to 13.7 points at L0 on a 28° ring, p90 8–20 on the faces.
  - **What carries the result.** The control variate does. The same table on the exact peak alone (Tab(ex)) is
    substantial on α, ρ and z (9–12), because the crate's clamp and undercut kinks sit inside the box. On LWW instead of
    R4+NF it is nearly as good (Tab(LWW): L0 −1.9 … +0.7, groups ≤ 2.2).

### 11.1 A: R4+NF in Rust [X]

**The model.** At each fillet station (half-width x, tangent angle a, radius of curvature ρ, q = x/ρ), as §10.2 with
the method track's round-4 changes (`proto/r4rs/src/lib.rs`):

    σ = F_V f_x (k_V/x − dy_c k_M/x²) + t_N f_y + t_M (p_x f_y − (p_y − y_O) f_x)
        − f_x (F_V/x) exp(−β |p_y − y| / x̄)                     (the near field; x̄ = (x + |p_x|)/2)
    F_V = max(1, (2x/ρ)^e_V)

- **N and M** come from Neuber's neck at the corner-mapped sharpness, with θ in closed form:
  tan²θ = tan²a + q_eff/cos³a. As a → 90° they take the exact limit (t_N, t_M, dy_c → 0). Past 90° they take the
  tangent wedge's Flamant and Carothers terms.
- **β = 3.748838**, the real part of the first antisymmetric Papkovich–Fadle root (sin 2k − 2k = 0), per half-width.
- **The search.** N stations on the fillet parameter, both ends included (the root-side end is the fillet's closing
  point). Every local maximum of the scan is refined by Brent's maximiser, seeded with its neighbours.
- **The 90° crossing.** The neck's limit at a = 90° is a √ cusp, a maximum narrower than any scan. Where the tangent
  angle crosses 90° between two stations, the crossing is a candidate in its own right. It is found once per tooth by
  bisection on cos a, a geometric root load-independent. Without it one ring's lowest-point load read −1.7 % low at
  16 stations.

**Agreement** (`r4/score.py`, `bench2.txt`):
- **Rust = Python.** R4 and R4+NF in Rust equal the Python model (`method/r4/final2.pkl`; 2,001 stations per piece plus
  the closing point) within 4.5e-5 at every load.
- **Root land.** The root land is not searched in Rust. No load peaks there (§10, item 3 of the method track), and the
  agreement confirms it.
- **Search against the dense maximum** (4,001 stations, every local maximum refined to 1e-9·h, the crossing included):

  | Loads | 16 stations, 1e-4·h | 16 stations, 1e-2·h | 8 stations, 1e-2·h | 24 stations, 1e-4·h |
  |---|---|---|---|---|
  | The three sets' 4,446 scoring loads | within 1.6e-10 | 80 loads beyond 1e-6, worst −2.0e-6 | within 1.7e-5 | within 5.4e-11 |
  | Every 4th ramp load, 9,120 | 2 loads beyond 1e-6, worst −2.4e-4 | 184 loads, worst −2.4e-4 | 2,868 loads, worst −2.6e-3 | none beyond 6.1e-11 |

**Scores** (the Rust values; figures as §10.4):

| Set | e_V | L0 | Classes ord · mid · tight · ring | z | α | x | ρ | Dir ±6 · ±14 | Height | Position · lowest point | Pairs |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Dev | 0.38 | −0.3 (−5.6 … +3.7) | −0.4 · +0.6 · −2.3 · −0.4 | 8.6 [7/38] | 2.8 | 3.2 | 5.2 [1/51] | 3.1 · 8.4 | 4.3 | 1.2 / 12.8 · −12.3 / −2.9 / +9.4 | 0 of 9,931 · p90 3.8 |
| Record holdout | 0.38 | −0.4 (−6.5 … +4.4) | −0.5 · +0.8 · −2.5 · −0.2 | 10.9 [2/24] | 3.1 | 3.6 | 5.4 [1/21] | 2.8 · 7.7 | 4.3 | 1.2 / 7.8 · −12.2 / −1.3 / +18.1 | 0 of 2,402 · 3.9 |
| Fresh | 0.38 | −0.5 (−11.8 … +2.1) | −0.0 · −0.5 · −1.9 · −3.0 | 5.8 [2/26] | **11.8** [7/26] | 3.4 | 2.3 | 2.1 · 5.6 | 2.4 | 2.1 / 10.5 · −9.5 / −2.6 / +14.2 | 20 of 2,111 · 8.3 |
| Record holdout | 0.39 | +1.2 (−3.8 … +6.2) | +0.4 · +2.4 · −0.2 · +1.7 | 10.1 | 2.9 | 4.8 | 6.3 | 3.3 · 8.8 | 3.9 | 1.2 / 6.6 · −9.7 / +0.6 / +23.0 | 0 · 3.8 |
| Fresh | 0.39 | +1.0 (−9.2 … +3.4) | +0.9 · +1.3 · +0.4 · −1.1 | 5.3 | 11.0 | 2.7 | 1.9 | 2.5 · 6.6 | 2.0 | 1.7 / 9.7 · −6.8 / +0.2 / +18.3 | 20 · 7.5 |

- **Every figure equals the method track's Python scoring (`method/r4/final2.log`) at print precision**, except one
  that moves by 0.1 (the fresh lowest-point minimum at 0.39, −6.9 → −6.8). The Rust model is that model.
- **The whole record (228 teeth, five tip-corner teeth out), at e_V 0.38:**
  - 1 reversal of 22,346 pairs 5 % or more apart;
  - misstatement median 1.3, p90 3.9, p99 6.7, max 11.6.
- **Pair counting.** The fresh pair counts take each duplicate 28° ring twice, as §10.4 first did.
- **Bias** (rule 6): at e_V 0.38, median −0.3 … −0.5, and unconservative on 56–67 % of teeth.

**Continuity** (`r4rs/examples/sweep.rs`, `r4/sweep.txt`) [X]:
- **The sweeps.** Each has 1,201 steps, at a unit involute-normal load near the root end, mid-flank and near the tip
  (t = 0.03, 0.5, 0.97 of the flank's roll bracket):
  - ring z 50 and 120, α 14.5 → 28.2° (through the cutter's clamp at 24.9° and 27.5°, and to where a ring stops
    building);
  - ring ρ_fP 0.005 → 0.45;
  - ring x −0.3 → 0.9;
  - external α 14.5 → 32°;
  - external ρ_fP 0.003 → 0.5;
  - external x −0.6 → 1.0 at z 17 (through undercut onset);
  - external z 12 → 400 and ring z 40 → 600, each tooth count.
- **The result.** The largest step that stands out from its neighbours' is 0.005 %, on z at ρ_fP 0.01. The largest
  plain steps are the smooth slope over one sweep step: 0.2–0.4 % in α at the ring's limit, and 0.7–1.3 % per tooth at
  z 12–25.
- **The fast search equals the dense maximum on every step of every sweep.**
- **What this clears.** R4+NF passes the continuity rule in every input swept, including through 90° crossings.
  §10's 7.3–7.6 % jumps are gone; this is the geometry sweep the method track's review asked for.

**Cost** (`r4/bench2.txt`, `bench_table.txt`; the 228-tooth set and 160-load ramp of `profile/r2wasm`). The machine
was shared with other builds, so read the ratios to LWW in the same run [X].

| Build | LWW (16 + golden) | R4+NF, 16 stations, 1e-4·h | 16, 1e-2·h | 8, 1e-2·h | 24, 1e-4·h | Ramp, 160 loads, 16 stations, 1e-4·h |
|---|---|---|---|---|---|---|
| native | 6.1–7.1 µs | 3.5–3.6 µs | 3.2 µs | 2.5 µs | 4.3 µs | 163–179 µs (1.0–1.1 µs per load) |
| wasm32 (node), three runs | 8.3–9.0 µs | 4.4–4.9 µs | 4.0–4.6 µs | 3.0–3.4 µs | 5.4–5.9 µs | 214–222 µs (1.34–1.39 µs per load); 1e-2·h 156–160 µs |

- **Per section.** These are per section at one load, everything from scratch.
- **Under the ramp.** The stations, the 90° crossing and their load-independent parts are computed once per tooth.
  Each load then pays 16 dot products and 16 exponentials for the near field, plus the refinements.
- **The crossing's cost.** Before it was hoisted out of the load loop, the ramp took 390–405 µs.
- **Where the time goes** [R, profile track]. A station costs about 87 ns native: atan2, atan, sin/cos, one ln, two exp
  and about four square roots. The near field adds one exp per station per load. No per-geometry numerical solve
  remains. The 90° crossing is a geometric bisection on the outline's tangent, as the crate's own section searches
  are.
- **Against LWW.** LWW here still uses its unoptimised search, 16 stations plus golden section. R4+NF's ratio to LWW is
  not like for like in that sense; the profile track estimated LWW would gain about as much from the same changes.

**Constants and choices.**

| Item | Status |
|---|---|
| e_V = 0.38 | **fitted** on the dev split (§10.2). 0.39 is the method track's rack calibration, off every scoring set; it trades +1.3 points of level for 1–2 points of x, ρ and direction structure |
| β = 3.748838 | derived: the Papkovich–Fadle antisymmetric root, computed. Taking the antisymmetric mode for a one-sided load is an argument, not a derivation [R, review] |
| Near-field amplitude V/x × F_V; x̄ = mean half-width; the floor max(1, ·) at 2x = ρ (LWW's pivot) | **values informed by a fit**: chosen after the dev fit gave amplitude 1.018 and exponent 0.381 [R]. Rounding a fitted value is still choosing it on the data [rev] |
| `qe.max(1e-9)` | a numerical floor; the result is unchanged to 1e-6 between 1e-6 and 1e-9 [rev] |
| A non-finite `neck_closed` | the station silently takes N = M = 0 (`r4rs/src/lib.rs:72`). The floor keeps it from firing, but if it fired it would under-rate silently; it should be an error [rev] |
| Corner map 2(1 − λ), pivot q0 = 1; V's c = 2x | named choices, as §10 |
| Stations 16, tolerance 1e-4·h | numerical choices, shown above not to move any figure |

**Load sweeps** [rev] (`r3rev/r4c/examples/sweep_load.rs`, `sweep_abs.rs`). Seven teeth (three rings, four externals,
including z 13 at 28°, x 0.6 and the 50-tooth ring at 27.5° with a 0.006 tool), t 0 → 1 and direction ±40° from the
involute normal at t = 0.02, 0.1, 0.5, 0.95, 4,001 steps each:
- no isolated step above 0.0001 %; the fast search equals the dense one to ≤ 4.6e-10;
- under steeply turned loads the peak can pass through zero (z 13, 28°, x 0.6: −0.075 … 4.4), continuously, which a
  table holding ln(·) cannot represent;
- a ring takes 27–29 evaluations per load, an external 6–18; the bench's 228-tooth mean understates a ring-heavy
  train.

**A ruling the documents need** [rev]. R4+NF bisects for the 90° crossing once per tooth and runs Brent's maximiser
at every load. Whether a geometric bisection and a peak search are "a per-geometry numerical solve at run time" is the
owner's call. The crate's own section searches are of the same kind.

**What it keeps** (by the one rule):
- **S, the load's position:** dev 12.8, record holdout 7.8, fresh 10.5. These are the rings and the single-pair zone
  near the root.
- **S, z at tight tools:** 8.6 and 10.9, at ρ_fP 0.01.
- **S, α at the cutting limit:** fresh 11.8, the 28° rings.
- **S, direction at ±14°:** 7.7 and 8.4.
- **At the threshold, ρ:** 5.2 and 5.4.
- **The lowest point:** spread −12 … +18, with an unconservative tail.
- **Off the scoring sets** [rev] (`fresh2`/`fresh3`, 139 off-node teeth, `ev2.txt`, `ev3.txt`; `real2.py`):
  - L0 down to −13.5 (ring, 28°); lowest point up to +19.6; ring sp4 −15.5 (28°);
  - ring α sweep (z 90, x 0.45, ρ 0.1): range 14.6 at L0, 27.7 at the lowest point;
  - external ρ sweep at the lowest point: range 7.3, so ρ is **S**, not at the threshold;
  - over the table's nodes at realistic directions:

    | Body | t ≈ 0.07, p90 | t ≈ 0.07, max | t ≈ 0.25–0.37, p90 |
    |---|---|---|---|
    | Whole gears | 13 % | 48 % | 6–7 % |
    | Sectors | 29 % | 51 % | 8–11 % |
    | Rings | 43 % | 119 % | 15 % |

    On the scoring loads the same figure is p90 9.1 % at t < 0.1. Over all directions the ψ = −10° and 55° faces
    reach a factor of 10.
- **Diagnosed, not fixed** [R]. The method track traced each to a body error: N and M's corner map at extreme
  sharpness; N's tooth-to-tooth spread; the neck's crack-like limit near a = 90°, which wants a body that turns
  through 90° with finite curvature.

### 11.2 B: the shipped table (Tab) [X]

**What it is** (`proto/table/`). Per patch, a tensor Chebyshev–Lobatto interpolant of ln(exact / control) over six
coordinates:
- **w = 1/z.**
- **α**, 14.5–28°.
- **x.**
- **s = ln(ρ_fP / ρ_clamp(α)).** ρ_clamp is the crate's largest tool radius at α. Beyond it the tooth does not change
  (rule 5's clamp). It depends on α alone: for externals it is 0.95 × the full-round rack tip, and for rings the shaper
  clamp, which falls from 0.47 at 14.5° to 0.0073 at 28°. It is found here by bisection on the crate's printed ρ_f; a
  shipped table would read it from the crate's own cutter limit.
- **The load's position t** on the flank's radial span, 0 at its root end and 1 at the tip.
- **The load's direction ψ** = atan2(N, V), −10 … 55°.

The rating is control × exp(table). The control is R4+NF (Tab(R4)) or LWW (Tab(LWW)). Tab(ex) interpolates ln(exact)
per unit load, with no control.

**The patches.** They follow the baseline's own discontinuities:

| Patch | Range | Body | x |
|---|---|---|---|
| ext-whole | z 12–27 | the whole gear on its shaft | −0.25 … 0.8 |
| ext-sector | z 30–1000 | a sector | −0.25 … 0.8 |
| ring | z 40–1000 | a sector | −0.1 … 0.7 |

- **z 28–29** falls between the external patches, and no scoring tooth sits there. As first written this said a crate
  would pick the patch by the record's r_f rule. In fact [rev]:
  - `evaluate.py`'s `patch_of` picks by z ≤ 27, not by r_f; neither rule was exercised.
  - The record's body rule (r_f − 10 ≥ 0.25 r_f, i.e. r_f ≥ 13.33) falls at x ≈ 0.08 for z 29 and x ≈ 0.58 for z 28,
    so the switch is a step in x, α and ρ as well as z.
  - At the same tooth the two extrapolated patches disagree by median 0.6 %, p90 5.1 %, max 14.0 % (z 28) and 15.5 %
    (z 29). The correction steps z 27 → 30 by p90 6.2 %, max 19 %, against p90 1.0 / 0.6 % for the neighbouring steps
    (`r3rev/patch.py`).
  - Under the stated fallback, z 28 at α 23.6 reads −3.4 at the lowest point under R4+NF against −0.3 from the table.
- **The tip centre.** Its across load (the height column's TV) has its own 4D table.

**The nodes.**
- **Geometry.** Five Chebyshev–Lobatto nodes per geometric axis, which nests the degree-2 subgrid. z is an integer,
  so the w nodes are the nearest integers' 1/z, and the interpolant is solved on the actual nodes.
- **Load grid.** 13 nodes in t and 15 in ψ. Every node is one solve on the record's own solver and body (`fillet_bem`
  through `bemx`):
  - pass A (ρ_ref/16, growth 0.07) for the hot spot under the mid-flank across load;
  - pass B (ρ_ref/32, 0.05) with 27 loads: a unit across and a unit along load at each of the 13 flank points, and the
    tip centre's across load.
- **Direction at the node.** It is exact at the 15 ψ nodes only, read by superposition, the peak being the sampled
  maximum over root land and fillet. **Between them it is not** [rev]; see the error bound below.
- **The node solve reproduces the record.** At five record teeth, read at their L0 load through the node's own t
  interpolation, it is within +0.02 … +0.23 % of the record's sampled L0.
- **The build.** 1,875 solves, with a median of 2.5–2.6 s on sectors and rings and 5.9 s on whole gears. About 2.5 h
  wall at a pool of 2, on a shared machine.
- **Twenty failed nodes.** All are ext-whole nodes at the pointed-tip corner (z 12–13, α ≥ 26°, x ≥ 0.65), where the
  crate's tooth has a tip of length 0. The BEM returns NaN on 13 and runs over 240 s on 7. They are filled by the
  polynomial along x through that line's other nodes, a stated extrapolation, and then used as interpolation data:
  there the table is not an interpolant of the baseline, and its error is unmeasured [rev]. The fresh z 20, 28°,
  x 0.75 teeth draw on that corner. The rectangular box takes in teeth the crate cannot build.
- **Undefined lines** [rev]. 13 of 3,120 ψ-lines give NaN where the peak nears zero, so ln(exact/R4) is undefined there.

**Scores** (every scoring tooth is out of sample; nothing was chosen on any set, and the box, degree, coordinates and
load grid were fixed before the build):

| Set | Option | L0 | Classes ord · mid · tight · ring | z · α · x · ρ (worst group) | Dir ±6 · ±14 | Height | Position · lowest point | Pairs: reversed · max misstatement |
|---|---|---|---|---|---|---|---|---|
| Dev | Tab(R4) | +0.0 (−1.6 … +0.9) | +0.0 · +0.0 · −0.0 · −0.8 | 1.1 · 2.2 · 1.2 · 1.0 | 2.7 · 2.7 | 3.6 | 0.4 / 5.0 · −3.5 / +0.2 / +5.5 | 0 · 2.5 |
| | Tab(LWW) | −0.0 (−1.9 … +0.7) | −0.1 · +0.0 · +0.1 · −0.9 | 1.1 · 2.2 · 1.3 · 1.9 | 2.4 · 2.7 | 3.6 | 0.3 / 4.7 · −3.4 / +0.1 / +5.6 | 0 · 2.6 |
| | Tab(ex) | +0.3 (−3.4 … +11.1) | +1.1 · +0.5 · +0.3 · −1.1 | 9.3 · **11.9** [22/60] · 3.5 · 8.2 | 2.4 · 2.7 | 3.5 | 0.3 / 4.9 · −2.5 / +1.5 / +12.8 | 57 · 15.0 |
| | Tab(R4), degree 2 | +0.1 (−2.1 … +4.5) | | 4.0 · **6.3** [2/60] · 3.2 · 1.8 | 1.9 · 2.5 | 3.6 | 0.4 / 4.4 · −9.0 / +0.8 / +7.8 | 1 · 6.4 |
| Record holdout | Tab(R4) | −0.0 (−1.6 … +0.9) | +0.0 · −0.1 · −0.0 · −1.2 | 1.4 · 1.8 · 1.4 · 0.8 | 2.1 · 2.1 | 3.6 | 0.5 / 4.1 · −4.1 / +0.2 / +3.8 | 0 · 2.5 |
| | Tab(LWW) | −0.0 (−1.9 … +0.5) | | 1.4 · 1.8 · 1.4 · 2.2 | 1.9 · 2.4 | 3.5 | 0.4 / 3.5 · −5.5 / +0.1 / +4.1 | 0 · 2.4 |
| | Tab(ex) | +0.3 (−1.6 … +11.5) | | 9.1 · **12.0** · 3.0 · 9.7 | 1.9 · 2.1 | 3.5 | 0.4 / 3.5 · −3.0 / +0.9 / +12.4 | 20 · 13.3 |
| Fresh | Tab(R4) | −0.0 (−1.7 … +0.4) | +0.0 · +0.0 · −0.0 · −0.5 | 0.6 · 2.0 · 0.7 · 0.6 | 1.4 · 2.0 | 2.8 | 0.4 / 4.5 · −1.3 / −0.1 / +4.4 | 0 of 2,111 · 2.2 |
| | Tab(LWW) | −0.1 (−1.7 … +0.3) | | 1.0 · 2.0 · 1.0 · 0.6 | 1.2 · 1.9 | 2.8 | 0.5 / 4.0 · −2.1 / −0.0 / +4.0 | 0 · 2.0 |
| | Tab(ex) | −0.0 (−1.9 … +6.2) | | 4.0 · 6.9 [7/26] · 3.0 · 1.9 | 1.2 · 1.9 | 2.8 | 0.4 / 4.0 · −3.3 / +0.2 / +7.5 | 1 · 8.2 |

- **The whole record, Tab(R4)** (`table/pairs_all.log`): 0 reversals of 22,346; misstatement median 0.4, p90 1.3,
  p99 1.9, max 2.5. Tab(LWW): 0, max 2.6. Tab(ex): 172, max 15.4.
- **Per load against the baseline, over all 4,446 scoring loads** (Tab(R4), degree 4):
  - L0: max 1.74 %;
  - path: 3.41 %;
  - turned: 2.27 %;
  - across and tip loads: 1.98 %;
  - lowest point: 5.50 %;
  - overall p99 2.48 %.
- **Where the worst sit.**
  - The lowest-point loads on rings (t ≈ 0.09–0.11, +4 … +5.5).
  - The rings' L0 level, −0.9 at the median. Five record teeth show this is the interpolant's and not the node solve's.
  - The ring patch spans z 40–1000 with the record's rings at its steep end.
- **Bias** (rule 6): median −0.0, unconservative on 50–56 %. The rings read −0.5 … −1.2 at the median, unconservative.
- **The scoring sets sit on the nodes** [rev] (`onnode.py`). The table was not fitted to them, but 59 % of the 298
  scoring teeth have a coordinate on a Chebyshev node (α 14.5, 28; z 12, 17, 40, 1000). Teeth with none score worse:
  worst-load error p90 3.1, median 1.4 (122 teeth), against 2.1, 0.9 with two coordinates on nodes (52 teeth). Only 135
  of the 4,446 scoring loads have t < 0.1.
- **An off-node holdout** [rev] (`fresh2`, `fresh3`: 139 BEM solves on the record's solver, every tooth off the nodes;
  fine α sweeps of externals at z 45 and 33 and rings at z 90 and 45, sweeps of z 10–400 across the patch seam, x
  −0.25 … 1.0, ρ 0.003–0.38 and the mate 12–400, and 25 seeded interior teeth). On the 124 inside the box, Tab(R4):
  all loads max |e| 8.3, p99 2.3; L0 −1.7 … +1.0; lowest point −8.3 … +5.5. Two exceptions to the scores above:
  - the ring α sweep (z 90, x 0.45, ρ 0.1) at the lowest point reads +0.4, +1.0, +3.5, +5.5, +4.6, +2.1, +0.7, +1.8,
    +1.6, +0.2: a range of **5.3** across one variable, peaking between α nodes (interpolation ripple, not mechanics);
  - ring (600, 16.87°, x 0.037, ρ 0.273) reads **−8.3** at the lowest point (R4+NF −8.8), outside −4.1 … +5.5.
  - Degree 4 is not always better than degree 2: the external α sweep at ρ 0.25 ranges 4.7 at the lowest point at
    degree 4 against 2.2 at degree 2, as interpolating a maximum with kinks would.

**The error bound.** The a-posteriori estimate is |degree 4 − nested degree 2|:
- **Over the sup** it is conservative: 16.2 % at most against a true 5.5 %. On L0 alone, 5.7 % against 1.7 %.
- **Pointwise** it bounds the degree-4 error at only 61 % of loads.
- **It is blind to the load grid** [rev]. It differences degree 4 and degree 2 over the four geometric axes only; the
  t (13 nodes) and ψ (15 nodes) interpolation is never estimated. At node geometries, where the geometric part is
  exact, the exact peak between ψ nodes (by superposition) shows:
  - at ψ midpoints, 120 nodes, 21,840 loads (`psi.py`): up to 16.9 % (whole), 21.6 % (sector), 26.2 % (ring), worst at
    t < 0.1 and ψ 0–40°, p90 there 1.8 / 3.8 / 6.5 %; at t ≥ 0.5 at most 2 %;
  - on 261 directions at 240 nodes (`r3rev/psi_between2.py`), the range across ψ −7 … 40° is median 13.7 and max 39.9
    at t = 0 (191 of 239 lines ≥ 5), median 7.3 and max 24.2 at t = 0.017 (138 of 239), 7.4–11.0 at most at t
    0.07–0.37 (6–21 lines per node), 2.4 at t = 0.5; pointwise up to 32.8 % at t = 0, and 15–21 % mid-flank at ψ
    45–53°, where the exact peak's location jumps.
- **Nor along the geometric axes** [rev]. Leave-one-out at degree 3 (`loo.py`, mid-flank, ψ −7 … 48°): maxima 13–19 %
  along w, 16 % along α for whole gears, 12 % along α for rings (p99 6.8 %). No holdout tooth sits there. In t, for
  rings: the Chebyshev tail (Σ|a₁₀..₁₂|, ψ 15–36°) has p90 12.7 %, max 21 %, and leave-one-out at the t nodes gives
  26–63 % (`loo_t.py`, `cheb_t.py`).
- So it is neither a certificate nor a reliable sup estimate. A rigorous bound would need an analyticity
  (Bernstein-ellipse) bound the BEM data cannot give, and data that are a maximum with kinks do not have one.
- Measured on the scoring sets the error is ≤ 1.7 % at L0 and ≤ 5.5 % at any scoring load; on the off-node holdout
  ≤ 8.3 %. That fails the other-fields track's own kill criterion (holdout max > 1 %). The owner's rule (no range ≥ 5)
  holds on the scoring sets and **fails** off them: direction and position near the root between nodes, and the ring
  lowest point across α (5.3).

**Cost** [X] (`r4rs/src/table.rs`, `bench_table.txt`; contraction timed on a table of the right shape):
- **Per geometry**, the 5⁴ × 195 contraction to a 13 × 15 slice: 20.8 µs native, 52–55 µs wasm32. This is unoptimised:
  an f32 table or blocking would cut it.
- **Per load:** 0.23 µs native, 0.44 µs wasm.
- **One load per section in wasm:** about 4.6 µs (R4+NF) + 54 µs, roughly 7× LWW.
- **The 160-load ramp in wasm:** about 218 + 124 µs per section.
- **Size:** 3 × 121,875 values, plus 1,875 for the tip table. It does not compress [rev] (`r3rev/size.py`): f64
  2.94 MB raw / 2.82 MB gzip, f32 1.38 MB gzip, f16 0.68 MB gzip (≤ 0.10 % rating change). The shipped
  `gear_wasm_bg.wasm` is 1.79 MB raw / 0.67 MB gzip today, so f64 makes the download about 5× and f16 about 2×.
  Low rank does not help enough: rank 40 errs 1.6–4.5 %, and 0.2 % needs rank 80 (`lowrank.py`).
- **Scaling** [rev]. Bending is rated per (member, line mesh) in every `cut`, so every solve, dry run and "one more
  tooth" re-solve pays the per-geometry contraction: ≈ 7× LWW and ≈ 13× R4+NF per section at one load.
- **Build:** 1,875 solves, about 2.5 h on two workers. The other-fields track's Rust BEM would cut it about 100×. Any
  change to the crate's tooth form invalidates every solve, and nothing fingerprints the table to the geometry it was
  built on, as `tools/fillet_bem.txt` is [rev].

**Continuity.** Not met as a rating [rev].
- **Within a patch** the table is C∞ in all six coordinates; the clamp face is continuous (u = min(ρ/ρ_clamp, 1)), t
  and ψ are continuous, and the control is continuous (11.1).
- **At the box's faces the rating steps** by the table's whole correction, exp(T) − 1, because outside the box it
  falls back to R4+NF (`face.py`, `faces*.py`, `r3rev/faces.py`):
  - measured across the α = 28° face: ring z 45, x 0.65, ρ 0.03, L0 13.7 points (Tab +0.2, R4+NF −13.5); ring z 90,
    L0 6.5; external z 45, ρ 0.25, lowest point 7.4;
  - from node values on the faces (t ≥ 0.06, ψ 15–40°): p90 8–12 points on external faces, 18–20 on ring faces; over
    all faces and loads p90 7–160 %, and single t = 0 nodes 17× (largest stored ln 2.86);
  - on the 4,446 scoring loads, the correction the fallback would drop: median 1.3 %, p90 4.6 %, p99 13.2 %, max
    26.8 % (L0: median 0.8 %, max 13.6 %);
  - mid-flank at ψ 22.5° the face correction is at most 5 % on externals, but 27 % on rings at z 40, 28°, x 0.7
    (`mid.py`).
- **Inputs the crate accepts land outside the box:** external α above 28° (the crate admits α to 90°, `input.rs`), z
  above 1000 (face step p90 12.9) or below 12, x outside −0.25 … 0.8 (rings −0.1 … 0.7), ρ/ρ_clamp below 0.012, and
  every addendum, dedendum, thickness_mod, cutter or rim other than the record's.
- **z 27 → 30 is not ≤ 1 %** as first written: see the patches above (step at the z = 27 face p90 7.5, max 52 against
  the fallback).
- **Extrapolating instead of falling back is not safe either:** at z = 10 the lowest-point error is −17.4.

**Against the owner's principles.**
- **Structured errors:** none substantial on the scoring sets (position at the threshold, 5.0). Off them, substantial
  in direction and position near the root between nodes, and 5.3 across α at a ring's lowest point [rev].
- **Grounded mechanics:** by proxy only. The table is the exact elastic solution interpolated, and the control model
  carries the mechanics. Tab(LWW) does almost as well as Tab(R4), so the table, not the component model, removes the
  structure.
- **No hidden fitted constant:** no model constant is fitted. What it has instead:
  - 366k tabulated numbers, regenerable by `table/plan.py`, `run.sh` and `nodemodels.py` from `tools/fillet_bem.py`;
  - named choices: the box, the patches, degree 4, the coordinates, the t and ψ grids, the pass A/B meshes;
  - one stated extrapolation, the 20 corner nodes.

  Whether that counts as a hidden fitted constant is the owner's call. It is visible, sourced and regenerable, but it
  is not a formula.
- **Constants of the baseline, built in** [rev]. ln(exact/R4) absorbs everything the BEM fixes: the body (rim depth
  `RIM`, `BORE`, the held cuts, the whole/sector switch), the rack's dedendum 1.25, no tooth-thickness change, the
  ring cutter, ν = 0.3. The crate takes `addendum`, `dedendum`, `thickness_mod` and `root_radius` as inputs
  (`params.rs`); `gear-cli fillet` cannot vary them, so neither prototype was tested on them, and for any non-standard
  rack the table applies a correction it has no node for, silently. Unless each is an axis or a refusal, these are
  hidden constants in the owner's sense.
- **Common material properties only:** ν = 0.3 plane strain is baked in. The body has fixed-displacement boundaries
  (`fillet_bem.py:286`), so the peak depends on ν, probably weakly; two solves per node at ν 0.25 and 0.35 would show
  whether it needs an axis. Not run [rev].
- **Fast mode** (µs–ms, no per-geometry solve, no cache): met. The live mode evaluates a polynomial; the offline build
  is the solve. The owner's condition on a shipped approximation, a **certified** error bound, is **not** met [rev].
- **Outside the box:** every crate input the table does not carry must be refused, or rated by R4+NF with a note
  (rule 5). That covers z < 12, α > 28°, h_fP, protuberance, the shaper cutter's own z0 and addendum, rim, ν, and loads
  off the flank except the tip centre's across load. A fallback with a note does not repair the step at the face
  [rev]; only a refusal, a domain covering every admissible input, or a stated blend does.

### 11.3 What the two prototypes say (they choose nothing)

- **Structured error.** No fast option measured is free of substantial structure everywhere.
  - Tab(R4) and Tab(LWW) have none on the scoring sets (groups ≤ 2.2, direction ≤ 2.7, position ≤ 5.0, 0 reversals),
    but off them they are substantial in direction and position near the root, and 5.3 across α at a ring's lowest
    point [rev].
  - Its cost: 2.9 MB f64 (0.68 MB gzip at f16, against a 0.67 MB gzip wasm today), ≈ 54 µs per geometry, a 2.5 h
    offline build per change of the baseline or the box, and a bound that is neither certified nor reliable as an
    estimate.
  - As built it is not continuous: the box faces and the z 28–29 gap step by up to 13.7 points.
- **R4+NF** is the fastest mechanics option measured (about half LWW's cost as prototyped) and continuous in every input
  and load swept. It keeps five substantial structures on the scoring sets and more across the box (ring α 14.6 / 27.7,
  ρ 7.3, near-root errors to 119 % on rings), each traced to a body the model lacks.
- **The two are not alternatives.** R4+NF is the table's control and, as built, its fallback, so the fallback's errors
  are what the face steps measure.
- **Open:** §11.4.

### 11.4 Reviews applied, and open issues [rev]

**The three reviews.** All re-ran the round's artefacts with the record's own solver; none used the web or edited the
repository. Scripts: `/tmp/claude-1000/-home-user-gearcalc/97717eac-5178-43f8-9259-847b3e7646bd/scratchpad/` (`gen2.py`, `gen3.py`, `fresh2.jsonl`, `fresh3.jsonl`, `ev2.py`, `ev2.txt`, `ev3.txt`,
`sweepsum.py`, `faces*.py`, `hole.py`, `loo_t.py`, `cheb_t.py`, `onnode.py`, `psi.py`, `loo.py`, `face.py`, `mid.py`,
`real.py`, `real2.py`, `box.py`, `neck.py`) and its `r3rev/` (`faces.py`, `patch.py`, `psi_between2.py`, `size.py`,
`lowrank.py`, `r4c/examples/sweep_load.rs`, `sweep_abs.rs`).
- **Off-node holdout** (139 teeth): confirms the scores in the box's interior; finds the ring α ripple (5.3) and a −8.3
  lowest point; the face steps; the z 28–29 gap; R4+NF worse across the box; the baseline's constants in the table.
- **Error bound and box**: the estimator is blind to t and ψ; errors between ψ nodes to 26 %; leave-one-out to 19 %;
  the fallback a jump; the 20 corner nodes extrapolated; R4+NF's near-root errors over the box; fitted-then-rounded
  choices; the silent non-finite branch.
- **Cost and continuity**: R4+NF continuous under load sweeps; the bisection and Brent ruling; Tab's face and patch
  steps, ψ between nodes (to 32.8 %), the size against today's download.
- The reviews' figures for the error between ψ nodes differ (16.9–26.2 % at ψ midpoints; 32.8 % on 261 directions)
  because they sampled differently; both are kept.

**Open issues.**
1. **Continuity of the table as a rating.** Cover the crate's whole admissible domain, refuse what is outside it, or
   blend the correction to zero across a stated margin. Map w to 0 so z → ∞ is inside. Join the external patches
   (overlap, or the body switch as a coordinate, or drop the record's fixed body). Shape the box to the buildable-tooth
   limit and solve or drop the 20 extrapolated corner nodes.
2. **Direction and position.** Store influence functions, not peaks: ln(S_V/control) and ln(S_N/control) at fixed
   fillet stations, maximised at run time, which makes direction exact everywhere and removes the ψ kink. Cluster t
   nodes at the root end (or use a coordinate that removes the near field), then re-measure off the grid.
3. **Honest scoring.** A validation set spread over the whole box in all six coordinates, off every node, including
   t < 0.15, the face corners and z 40 rings at 28°, reported with how it was sampled, the off-node subset separately.
   Stop calling the estimate a bound. If the owner's rule needs a certified bound, the table does not meet it.
4. **The baseline's built-in constants.** Name body, rack form, ν and thickness as visible options, or extend the BEM
   harness to sweep addendum, dedendum and thickness_mod and test them; run the ν = 0.25 / 0.35 check.
5. **R4+NF's own figures** across the box next to the scoring figures, since it is the fallback; its non-finite
   `neck_closed` branch made an error; the amplitude and floor labelled as fit-informed (done above).
6. **The ruling** on a per-tooth geometric bisection and a per-load Brent search under "no per-geometry numerical
   solve".
7. **Shipping detail** if the table goes on: f16 or f32, a CI fingerprint of the geometry, Table 5's sweeps (not run),
   the f32 or blocked contraction, a Rust BEM for the build.
