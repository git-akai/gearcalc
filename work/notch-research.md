# Singular stresses at sharp corners and notches (research track)

Track opened in `plan.md` §5 on 2026-10-01; parallel unless it can replace a model outright. **Round 3
(2026-09-29):** three adversarial reviews of the prototype (§5) are applied, with the editor's checks; open
issues §6, verdict §8. Read-only on the repo; pure Python, ≤ 1.5 GB; the search budget ran out before round 3.
- **Scripts:** `~/.cache/gearcalc-work/notch-research/` (`review3/` holds the reviewers' `net.py`, `edge.py`
  and `r2/`); the prototype `~/.cache/gearcalc-work/notch-proto/`, whose `README.md` lists every output.
- **Marks:** [R] full text read · [A] abstract or snippet · [M] metadata only · [C] recalled, not verified ·
  [X] computed in this track. **Rule:** inputs are common properties only (E, ν, σ_y, σ_u, hardness,
  density, a tabulated endurance limit); measured special factors are the *validation set*, never inputs.

## 1. State of the art: why the stress diverges and what bounds it

**The divergence is the model's, and exact.** A corner of zero radius has an elastic field σ ~ K·r^(λ−1),
λ a bracketed root of a determinant (Williams 1952 for a free notch; a two-wedge determinant for an edge
pressed on a body). FE or BEM reads it at element size h, so the peak grows as h^(λ−1): the solver reports
an infinity the model contains. Three independent solvers agree to three digits [X]:

| Corner | Angle | 1 − λ |
|---|---|---|
| Root corner a rack cuts (opening 90° + α_n) | α_n 14.5° / 20° / 25° | 0.427 / 0.414 / 0.400 |
| Tip edge (φ = 90° + α_a) on a flank, frictionless, like materials: 43-tooth wheel / 17-tooth pinion | 116.1° / 122.8° | 0.426 / 0.450 |
| same, μ = ±0.06 / stuck | wheel; pinion | 0.41–0.44, 0.485; 0.44–0.46, 0.490 |

No singularity below φ = 77.45°; λ is ν-free for like materials; a kink *inside* a contact is logarithmic
[X]. **Everything except the peak converges:** 1 − λ < 1, so forces, energy, shares, approach and efficiency do.

**What bounds it physically**, from the largest scale down:
1. **The real radius.** No manufactured corner is sharp; a generated fillet is a trochoid of finite
   curvature at finite z (`strength.rs:2766`: q_s only 1.62 → 2.37 as ρ_fP falls 0.38 → 0.005 m).
2. **Plasticity, in the body that carries it.** A flat punch on a half-space is capped at Prandtl's mean
   (2+π)k ≈ hardness. A loaded **corner** of angle φ > 90° collapses sooner, at p_c ≤ 2k(1 + φ − π/2)
   (Hill's wedge; φ − π/2 = α_a at a tip): 2.91k / 3.14k on wheel / pinion, 0.57–0.61 of Prandtl [C;
   derived X, `review3/edge.py`]. A cyclic range above 2σ_y cannot shake down (Melan). **Shakedown bounds
   ratcheting, not pitting:** a surface loaded every mesh is still rated on its contact-fatigue limit.
3. **Crack mechanics.** A notch's fatigue limit is whether a small crack keeps growing (ΔK against ΔK_th):
   hence K_f < K_t, saturating at sharp notches, with a length L = (1/π)(ΔK_th/Δσ₀)².

**The literature** reads the field with a hidden length (K/σ)² (critical distances, TCD; strain-energy
density, SED; fictitious rounding, IIW r_f = 1 mm; finite fracture mechanics, FFM) or reports the intensity
(N-SIF; the contact-edge K). **The dimensional fact that decides this track:** E, ν, σ_y, σ_u, HV and σ_e
hold no length, so **no method driven by them alone can predict a notch-size effect** (K_f < K_t). They
supply (a) the geometry's length (ρ_F, r_e); (b) a **load-derived** length, where the elastic peak meets a limit; (c) for steels,
ΔK_eff,th ≈ 1.3·10⁻⁵·E·√m (2–3 MPa√m, R-free; Tanaka 2024 [R], Chapetti 2023 [R]), so L₀ =
(1/π)(ΔK_eff,th/Δσ₀)². But Chapetti uses that constant only as a fill-in and prefers grain size [R]; Tanaka
adopts 3 MPa√m (+21 %, L₀ ×1.47) [R]; and L₀ (1–16 µm) lies below root roughness (Rz 2–20 µm) and grain size
(10–30 µm) [C]. (c) is a **class constant for steels**, like Peterson's a; a field read at L₀ is a regulariser.

## 2. Candidate paths, ranked by the owner's focus

| # | Method | Mesh-indep. | Length from | Closed form | Contact edge | Fillet | Inputs | Meets focus | Sources |
|---|---|---|---|---|---|---|---|---|---|
| 1 | Pointwise elastic peak at a sharp corner | **no** (∝ h^(λ−1)) | the mesh | — | diverges | n/a | E, ν | the failure mode | — |
| 2 | Elastic peak at the **real** radius | yes | geometry | fillet: DB/ISO fits; edge: across solution | yes | **the crate's model** | E, ν | **yes** | DB 1942 [C]; ISO 6336-3 |
| 3 | Williams / N-SIF (Gross–Mendelson; Lazzarin–Tovo/Filippi) | yes | none | λ yes; K^N per geometry | characterises | ρ^−0.41, a **law test on the BEM** | none | yes (no rating alone) | Williams [C,X]; ESIS 2006 [R] |
| 4 | Contact-edge K plus the rounded-edge inner law | yes | r_e | yes | r_e → 0 asymptote and bracket only | — | E* | yes | Sackfield 2003 [A]; Moore & Hills 2022 [R]; Eames 2026 [R] |
| 5 | Wedge-corrected elastic edge | yes | r_e | λ yes; multiplier unmeasured | size unknown | — | E, ν | yes | Fleury–Hills–Barber 2016 [A] |
| 6 | **Plasticity bounds on the mate** (first yield, Melan) and the **limit radius** | yes | load | root of the solved field | ultimate case | overload | σ_y, E, ν | **yes, gated** (yield-measured metals; σ_u/σ_y ≥ 1.2 for shakedown) | Johnson 1985 [C]; Melan [X] |
| 7 | Geometric shakedown (run-in to r_sd) | yes | load | yes | **dropped** (§4) | no | σ_y, E | — | Kapoor–Johnson 1992 [M]; Kumagai 2022 [R] |
| 8 | Neuber local strain, elastic-perfectly-plastic | yes | none | yes | — | overload strain | E, σ_y | yes | Neuber [C] |
| 9 | **TCD at the intrinsic L₀** | yes | E, Δσ₀ + constant | on closed fields | research | finite at a sharp root | E, σ_e, σ_u + class constant | **partial** (steels; ≈ ×2 loose) | Chapetti 2023 [R]; Tanaka 2024 [R]; Taylor [A] |
| 10 | TCD with measured L; SED over R₀; FFM (Leguillon) | yes | ΔK_th, K_IC | V-notch yes | fretting practice / open | yes | **ΔK_th, K_IC ✗** | no | Taylor 1999/2007 [A]; Araújo 2007 [A]; Lazzarin–Zambardi [A]; ESIS 2006 [R]; Leguillon 2002 [A] |
| 11 | Fictitious rounding / ISO Y_δrelT (ρ′) / IIW r_f; Peterson a(σ_u), FKM n_σ | yes | ρ′, ρ* tables; hidden mm | yes | dropped (§6) | yes | tables, σ_u + class constants | rule of thumb | M56 [R]; Radaj 2013 [A]; Braun 2020 [R]; Peterson [A]; FKM [C] |
| 12 | PSM (Meneghetti–Lazzarin) | **no**, calibrated | element size | no | — | — | per-code constants | no | PSM preprint [R] |
| 13 | **Profile smoothness; the relief that unloads the edge** | removes it at source | the form | yes (approach) | **the remedy** | flank–fillet joint | E, ν, geometry, load | **yes** | Johnson [C]; Komori–Kubo 2004 [A] |
| 14 | **Limit load of the tip corner** (Hill wedge) | yes | none | yes | tip body; land width | — | σ_y | **yes, gated** | Hill 1950 [C]; [X] |
| 15 | Cyclic yield from σ_u, HB (Manson rule; UML) | — | none | yes | shakedown gate | overload gate | σ_u, σ_y, HB | partial (unscored) | Manson [C]; Bäumel–Seeger [C] |
| 16 | Surface factor (Marin, FKM K_R, ISO Y_RrelT) | — | Rz (drawing) | yes | — | fatigue | σ_u, Rz + class constants | rule of thumb | Marin [C] |

## 3. Scored against the validation set

**3.1 Support alone, generic fillet** [X, `synth/support_score.py`]. The validation values are **formulae**, not
measured K_f: V1 ISO 6336-3 Method B, n = 1 + √(0.2ρ′(1+2q_s)) (IACS M56 [R]); V2 Peterson, a = 0.0254·
(2070/σ_u)^1.8 mm [A]; V3 FKM [C]. Over case-hardened, through-hardened and nitrided steel n − 1 is 5–41 % (V1),
1–23 % (V2), 6–29 % (V3), disagreeing as much as the effect; C1, the point method at L₀, recovers 0.1–8 %.

**3.2 Support on the library's teeth** (`t_support.txt`; 176 m1 BEM teeth; n − 1 min / median / max):

| | C1 at L₀/2 (BEM field) | V1 ISO ρ′ | V2 Peterson | V3 FKM [C] |
|---|---|---|---|---|
| 4340 Hardened (σ_u 1500, σ_e 750; ρ′ 0.0014; L₀ 1.94 µm) | 0.36 / 0.95 / 17.5 % | 3.2 / 5.1 / 20.4 % | 2.0 / 9.6 / 175 % | 12.2 / 15.5 / 31 % |
| 4340 annealed (σ_u 690, σ_e 330; ρ′ 0.0281; L₀ 9.74 µm) | 2.1 / 4.5 / 79 % | 14 / 23 / 91 % | 6.8 / 34 / 438 % | 24 / 31 / 62 % |

C1 recovers ≈ 18–20 % of ISO's support (median). **It is not a bound:** it exceeds FKM by up to 10.9 % on
six annealed z 1000, ρ_fP 0.01 teeth (FKM's gradient out of range) and ISO once by 0.5 %.

**3.3 Net of the K_t error: the crate's default rating** [X, `review3/net.py` on `t_fillet_grid.json`].
Rated stress over effective stress (BEM peak / n) − 1; ordinary fillets (ρ_f/s_Fn ≥ 0.1, 81 teeth); median
(min … max). **Negative is unconservative.**

| rated | support believed | 4340 Hardened | 4340 annealed |
|---|---|---|---|
| DB (crate default), n = 1 | none | −14.7 (−29 … +8) % | −14.7 % |
| same | V1 ISO | **−10.8 (−26 … +12) %** | +1.4 % |
| same | V2 Peterson / V3 FKM | −9.1 / −2.2 % (worst −23 / −19 %) | +3.9 / +9.6 % |
| ISO Y_F·Y_S, n = 1 | V1 ISO | +12.0 (−2 … +31) % | +27.4 % |

- **Round 2's rule-6 record had the wrong sign for the default.** "No support is conservative by 3–20 %" holds
  for the support alone; net of DB's K_t error the hardened steel is rated **unconservatively** by 2–11 %
  median (89 % of teeth under, with ISO's support), worst −19 … −26 %, and tight fillets (ρ_f/s_Fn < 0.02) by
  47 % (ISO past its clamp: −18 %); the annealed steel is net conservative by 1–10 %. Surface finish (3.4)
  moves both further the wrong way.
- The BEM's shoulder canary was never run (§6). Without any BEM, the crate's own ISO/DB ratio on these teeth
  is 1.25 median (0.94–1.73): one fit is off by about a quarter, and the BEM sides with ISO.

**3.4 Two biases not yet counted** (rule-6 items, with sign):
- **Surface finish.** σ_e is a coupon figure (`fatigue_specimen = "coupon"`) against a hobbed or ground root,
  and the crate applies no surface factor. Marin at σ_u 1500: ≈ 0.85 ground, 0.65 machined [C], i.e. 15–35 %
  unconservative, more than the support. A factor needs Rz (a drawing input) and class constants (#16).
- **Cyclic softening.** Melan's 2σ_y and the edge's shakedown use monotonic σ_y. 4340 Hardened's σ_u/σ_y = 1.10
  is the softening class by Manson's rule (< 1.2 softens, > 1.4 hardens) [C]; SAE 4340 at 409 HB cycles to a
  yield ≈ 0.53× monotonic (K′ 1655, n′ 0.131) [C, validation value]: both shakedown limits up to ≈ 1.9× high,
  r_sd 3.6–6.6× too small. σ_u/σ_y is a common-property gate.

**3.5 Fillet, static.** The crate's `Ultimate` bending check is first yield at the fillet. Round 2's
collapse reading (M_p = σ_y·b·s_Fn²/4 · 2/√3) is **not established** [X]: the tooth is stubby (h_Fe/s_Fn
0.38–1.45, median 0.45) and a plane-strain M–V interaction cuts collapse to 0.53–0.90 of M_p/h, while the
taper adds some back (sign open); a compressive axial force lowers M_p (M/M_p + (N/N_p)² ≤ 1), where
`t_measure.py` subtracted it and understated η_col ×1.16 median; 2/√3 holds only where b ≫ s_Fn, and made
consistent with a uniaxial first yield the canary's collapse/first-yield ratio is 2.75–2.83, not 3.18.
Collapse needs ductility: withheld where `ultimate_measure = "break"` (PA6 GF30, PA GF50, PA GF70).

**3.6 Edge.** No gear source rates an edge by intensity or shakedown, so there is no measured set.
- **Closed forms re-derived** [X, `edge-skeptic/`]: truncated-Hertz K, P and the tilt law K = √(2E*γP) match
  numerics to every digit; c = 0.58168; peak coefficient 0.94223 against Eames's 0.943. Johnson's shakedown
  reproduced by Melan [X, `synth/melan.py`]: 4.002k at μ 0, 3.565k at 0.1.
- **Against the crate's own flank allowable**, the rating a surface swept every mesh needs [X,
  `t_mesh2d.txt`]: at the edge's peak phase the round at r_e 0.2 mm runs **3.07× the flank's pressure** at
  20 N·m (4691 against 1527 MPa), 2.98× at 60 and ≈ 2.2× at 0.45 mm. To meet 4340 Hardened's
  `contact_estimate` (845.7 MPa, MQ) it needs ≈ 4.7 mm at 20 N·m, 13.9 mm at 60 (Hertz, mate flat, which
  understates), and ≥ ≈ 1.5 mm at any load where the flank itself meets it; the tip is 0.67 mm thick. **A
  loaded edge cannot be rounded into passing the flank's own criterion; relief is the remedy** (cf. Kubo [A]).

## 4. Recommended path for this crate

**Plainly: the best common-property method bounds the stress and says where a feature fails; it does not
compute a notch factor.** K_f < K_t needs a length that common properties lack. They do give, mesh-free, the
elastic peak of the real geometry rated on the crate's own allowables, plasticity in the body that carries
it, the relief that takes an edge out of load, and the radius a fillet needs.

**Ranked by how completely common properties drive them:** (1) **#2 + #6 + #14**, the peak at the real
radius rated on the crate's allowables and bounded by plasticity: E, ν, σ_e, the flank's contact allowable,
σ_y where `ultimate_measure = "yield"`, shakedown only where σ_u/σ_y ≥ 1.2; (2) **#13**, the relief that
unloads the edge: E, ν, geometry, load, no strength figure; (3) **#9**, the point method at L₀, a labelled
option for steels; (4) **#11, #16**, rules of thumb. **Not meeting the focus:** #10, #12, K′/n′.
**Hardness is a σ_u-class input.** Tabor's H/3 is a flow stress at ≈ 8 % strain: 1497 MPa for 4340 Hardened
(σ_y 1365, σ_u 1500), 752 for annealed (470, 690). As σ_y it errs **+10 … +60 %, unconservative**, and the
bias is recorded wherever σ_y comes from HV (ISO 18265 converts HV to σ_u).

```text
EDGE: the crate's (Rating::Contact, CaseKind) table; inputs E*, ν, μ, σ_y, the flank's contact allowable
  relief    the tip relief at which the edge's load → 0 at every phase, from across.py's closed approach
            δ = (1/π)∫g dθ + (2P/πE*)·ln(2/c): the actionable output          [not yet prototyped]
  r_e given p_max(r_e) from the across solution (never the round's pointwise Hertz), rated:
    fatigue   p_max ≤ contact_fatigue_allowable (else contact_estimate), the flank's own figure
    ultimate  mate (moving load): C_fy·p_max ≤ σ_y once; C_sd·p_max ≤ k when repeated (the Duty's count);
              C on the solved pressure at each r_e, continuous in μ; traction sign from the kinematics
              tip (stationary, a wedge): mean q/w ≤ p_c = 2k(1 + α_a + γ), γ the tilt; replaces (2+π)k
  r_e unset q_edge and the integrated outputs, "elastic, before run-in"; the land b = q_edge/p_c and the
            outputs' sensitivity to it. No K as a physical output.
  r_lim     smallest r_e ≤ r_max passing every rating, r_max where the tip land vanishes (per gear), with
            the continuous companion p_max(r_max)/S_lim; "none" → report the relief
FILLET: the crate's section and K_t (DB or ISO), with the net bias of §3.3–3.4 recorded; n = 1
  S_lim  fatigue σ_e (surface factor a visible option) · overload range 2σ_y only where σ_u/σ_y ≥ 1.2 ·
         ultimate first yield σ_y; collapse withheld until an M–V–N lower bound exists
  r_lim  smallest ρ_fP with η ≤ 1 for every larger radius to the full round (scan, then bracket), only
         from a fit whose slope in ρ passed the BEM's law test
  option (#9)  max σ₁ on the curve offset L₀/2 into the material; steels; "finite, loose up to ≈ 2×"
```

**In words.** Both edge bodies are rated; the lower governs. Peak for peak, the tip's crush ((4/π)·p_c =
3.70–4.53k) sits above the mate's first yield (3.11k) and at or above its shakedown (3.73k) [X, rough], so the
mate governs a round in the Hertz regime and the tip's crush the sharp state; "runs in to ≈ r_sd" and the
(2+π)k ceiling are dropped. The fillet's default K_t is the owner's decision: ISO Y_S continued past its
q_s = 8 clamp by the Williams law (K_t ∝ q_s^(1−λ₁), C⁰, no new constant) once tested on the BEM, or DB
with its net bias recorded as the unconservative figure it is.

**Worked example**, 17/43 m1 b10 spur, 20 N·m, μ 0.06 [X, §5.1]. **4340 Hardened:** the edge fails fatigue at
every radius the tip can carry; first yield and shakedown have no root below 0.45 mm (p_max(0.45) = 3526,
above 2931 MPa): the output is the relief. **σ_y 2000:** first yield 0.431 mm, shakedown 0.256 mm (ultimate only).
**HV 700 via H/3:** 0.291 / 0.177 mm, biased small. **Fillet**, 4340 Hardened, 21.8 N·m: r_lim 0.329 mm by DB,
where the BEM peak is ≈ 5 % above σ_e (exact r_lim > 0.38 mm); ISO's 0.423 mm is close. **In the crate** the
edge is one more `Rating::Contact` in `allowable()`; wedge and shakedown readings are additions.

## 5. Prototype results (P1, P2)

Run 2026-09-29 in `notch-proto/`; contact-proto, the verifiers' scripts and `gear-cli` read-only; all figures
[X]. The pre-registered gate: *if P1 passes steps 1–4 the refusal gives way; a step-2 failure keeps it.*

### 5.1 P1: the edge

**Step 1: one closed-form across solver** (`across.py`, `t_across.txt`). Contact and edge intensity come from
two θ-integrals of g′ over a piecewise-quadratic gap; the approach is **δ = (1/π)∫g dθ + (2P/πE*)·ln(2/c)**,
with no pointwise curvature. Closed forms agree to 4·10⁻¹³ (26 cases); rounded sections match the verifier's
BEM to 6·10⁻⁴ and `contact2d` to 2·10⁻⁹. At a fixed outer problem the inner law's error is ≈ +0.5·r_e/w: a 3 %
band needed d/w ≈ 0.014, not round 2's 0.2; the approach converges as O(r_e).

**Step 2: whole-mesh convergence** (`mesh2d.py`, `t_mesh2d.txt`). 17/43 m1 b10 spur, μ 0.06, 1000 phases per
pitch; the verifier's geometry and tooth compliance; every contact `across.py`'s; r_e = 0 the truncated edge.
Fixed on the way, both also in v1_spur2d: an atan2 branch cut in the pinion-tip distance, and a 0.05 mm
pair-gap cut-off that dropped a 50 µm edge pair at 60 N·m.

| relative change from r_e = 0 | 20 N·m: 0.02 mm | 0.005 | 0.002 | 0.001 | 60 N·m: 0.02 mm | 0.005 | 0.002 | 0.001 |
|---|---|---|---|---|---|---|---|---|
| friction loss (1.197 % / 1.305 % at r_e 0) | −7.6e-3 | −1.5e-3 | −4.7e-4 | −1.6e-4 | −2.9e-3 | −3.0e-4 | +3.9e-5 | +9.8e-5 |
| mean approach (TE) | +5.3e-3 | +1.1e-3 | +3.2e-4 | +1.1e-4 | +2.2e-3 | +2.5e-4 | −1.2e-5 | −6.2e-5 |
| TE peak-to-peak | +8.6e-4 | +1.7e-4 | +8.8e-5 | 0 | +1.9e-2 | +3.3e-3 | +7.5e-4 | +9.1e-5 |
| largest pair-share difference at any phase (/W) | 3.4e-2 | 8.3e-3 | 3.2e-3 | 1.5e-3 | 8.3e-3 | 1.4e-3 | 3.8e-4 | 2.9e-4 |

- **Integrated outputs pass** (≤ 0.26 % at 0.002 mm, linear in r_e) on any phase grid (loss at 0.002: −4.69e-4
  / −4.67e-4 / −4.67e-4 on 613 / 1000 / 1499 phases). **Sampled maxima do not:** the edge's peak share at 0.001
  reads +3.4e-5 / +7.6e-4 / +1.85e-3 and the label-based mean share fails 0.5 % on 613; both are dropped until
  refined in phase. v1's pointwise Hertz compliance on the round (Weber) drifts ≈ 3 %/decade.
- **Converged, but to an elastic state that cannot occur.** The sharp contact's mean pressure is 11.4 GPa at
  20 N·m (wedge indentation E*·tanγ/2: 11.9) and 17.2 at 60, 2.8–4.2× 4340 Hardened's Prandtl pressure. Its
  crush land b = q_edge/p_c is 25–30 µm at 20 N·m, 74–90 µm at 60; an equivalent r_e ≈ 1.4b moves the loss ≈ 1–2 %
  at 20 N·m and TE peak-to-peak ≈ +12 % at 60 [X, estimate]. Round 2's "runs in to r_sd = 0.2 mm" would move
  the loss −8.7 / −5.5 % and a pair's share by up to 30 % of W.
- **Peaks at 20 N·m**, refined in phase: p_max 4691, 6087, 8011, 11537, 15073, 19505, 27092, 34505 MPa at r_e
  0.2, 0.1, 0.05, 0.02, 0.01, 0.005, 0.002, 0.001 mm (sharp: K_max 1398 MPa·√mm; 2973 at 60 N·m), within 0.9 %
  of the verifier's exact-circle BEM and its slopes within 0.007: its "r_e^−0.37" is a transition toward −1/3.
  The tilt at a loaded edge reaches 0.21–0.27 rad. **Only phases 0–0.4 were searched:** the pinion-tip exit
  peak (0.63–0.69) reaches 87 % of the maximum; nothing here moves, but a rating must search the whole pitch.

**Step 3: the asymptotes at the phase of the maximum** (`t_asym.txt`; d/r_e from `review3/edge.py`):

| r_e (mm), 20 N·m | 0.05 | 0.02 | 0.01 | 0.005 | 0.002 | 0.001 |
|---|---|---|---|---|---|---|
| d/w · d/r_e (rad the round turns inside the contact) | 0.48 · 0.17 | 0.52 · 0.28 | 0.42 · 0.37 | 0.31 · 0.47 | 0.19 · 0.64 | 0.12 · 0.81 |
| p_max / inner law · / Hertz on the round | 1.063 · 0.967 | 0.942 · 0.930 | 0.943 · 0.886 | 0.956 · 0.830 | 0.973 · 0.745 | 0.982 · 0.678 |

The inner law is within 3 % where d/w ≤ 0.2, **but only where the half-plane's small-slope assumption fails**:
the round turns 0.64–0.81 rad inside the contact (1.34 at 60 N·m, 0.001 mm, beyond the corner's 1.115). d/r_e
≤ 0.2 needs r_e ≥ 0.067 mm (0.305 at 60 N·m), where d/w ≈ 0.5: **no radius here meets both**, so the inner law
is an asymptote and a bracket, never a rating. Hertz on the round is approached from below (−1.1 / −1.5 %).

**Melan and first yield with traction** (`melan.txt`; solved-pressure rows `review3/r2/melan_actual.json`).
Limit σ_y/C for first yield, k/C for shakedown:

| distribution | μ | first yield C | Melan C | p_sd |
|---|---|---|---|---|
| Hertz | 0 / 0.06 / 0.10 | 0.5575 / 0.5604 / 0.5654 | 0.2500 / 0.2681 / 0.2805 | 4.001k / 3.729k / 3.565k |
| rounded-edge inner field, traction toward / away | ±0.06 | 0.5188 / 0.5279 | 0.2390 / 0.2797 | 4.185k / 3.576k |
| **solved pressure at the roots** (r_e 0.196–0.366 mm) / at 0.0015 mm | +0.06 | 0.5572–0.5582 / 0.5361 | 0.2688–0.2691 / 0.2767 | 3.72k / 3.61k |

Every real-material root lies at d/w 3.6–22, where the pressure is Hertz-like: round 2's inner constants
(0.5279 / 0.2797) put the first-yield radius 18 % too small (unconservative) and shakedown's 11 % too large.

**Step 4: limit radii at 20 N·m** (`t_limits.txt`, corrected by `review3/r2/roots_fixed.py`): roots of the whole
mesh's p_max(r_e) = S_lim in log r_e on [10⁻⁴, 0.45] mm (round 2's values in brackets):

| material | fatigue (flank allowable) | first yield (mate) | shakedown (mate) | tip crush land, sharp |
|---|---|---|---|---|
| 4340 Hardened (σ_y 1365; `contact_estimate` 845.7) | none: needs ≈ 4.7 mm | none ≤ 0.45 (S 2451) | none ≤ 0.45 (S 2931) | 25–30 µm |
| σ_y 2000 stated | none | **0.431** (0.366) | **0.256** (0.286) | 17–21 µm |
| HV 700, H/3 = 2288 (a σ_u-class figure) | none | 0.291 (0.249) | 0.177 (0.196) | 15–18 µm |

- At 60 N·m nothing roots below 0.45 mm. The closed form c³K²E*/S³ over-states the root ×1.6–2.2 at every
  material limit (8 % even at d/w 0.18): r_lim is the field's root.
- **Continuity on the field root** (brent 10⁻¹²): T(1 ± 10⁻⁹) moves r_lim by −1.0e-7 / −1.2e-7, phase-maximisation
  noise; the ≤ 10⁻⁶ law passes with a floor of ≈ 10⁻⁷ (round 2's 1.4·10⁻⁹ measured the closed form). r_lim ∝
  T^1.37 over 19–21 N·m, no jumps; E, ν, μ and geometry were never perturbed. **The bracket is not derived:** the
  tip land vanishes at ≈ 0.62 / 0.61 mm (pinion / wheel), and r_lim jumps to "none" at 0.45.

**P1 on the prototype itself** (`t_proto.txt`; TField, μ 0.06, 48 phases; spur 17/43, β 0.5°, β 2° at 20 N·m;
ring 17/−43 m2 β15 at 60 N·m). The rounded sequence converges on all four (≤ 0.04 % from 0.002 to 0.001 mm),
**but the unset state sits 7.2–7.8 % (ring 4.4 %) off in mean approach and 11–12 % (6.5 %) in loss.** Round 2
blamed the flank convention, yet mesh2d's flank-convention state is only −0.57 % / +0.77 % from the truncated
edge, opposite in approach: the gap's main cause is unfound, and `t_proto.py` has no truncated state at all.

**P1 against its gate:** step 1 **pass**; step 2 **pass** in mesh2d (spur only), **fail** in the prototype
against its own unset state; step 3 inner law only outside its assumption, Hertz **not met**; step 4 closed
form **fail**, continuity **pass** on the root (floor 10⁻⁷, torque only); step 5 not run. **Decision: the
refusal stays.** A step-2 failure keeps it by the pre-registered rule; round 2's "the refusal can go" relaxed
its own gate and is withdrawn. Established: integrated outputs converge (mesh2d); edge compliance comes from
the across solution, never the round's 1/r_e; r_lim is a field root; on a loaded edge "none" is usual (§3.6).

### 5.2 P2: the fillet

**The instrument** (`bem.py`, `t_bem.txt`, `t_tooth.txt`): plane-strain BEM, independent of `strength.rs`. Kirsch
K_t 3.00000, interior ≤ 2.4·10⁻⁷; ellipses a/b 3 and 10 within 10⁻³; `tooth.py` = `gear-cli iso` to 2·10⁻¹² on 8
pairs. Three teeth on a 3m rim, a cos² patch at HPSTC, the peak a least-squares parabola over ±0.15ρ; rim 5m
−0.30 %, five teeth +0.59 %; mesh convergence median 0.09 %, worst 0.96 %. **The pre-registered Peterson-shoulder
canary was never run.** The four unsolved teeth are **ε < 1** (0.914–0.971), not singular systems: `tooth.py`
puts the load 0.05–0.15 mm beyond the tip, where the crate clamps (`highest_single_pair`, (ε − 1).max(0)).

**DB and ISO against the BEM** (`t_fillet_grid.txt`): z {12, 17, 30, 60, 150, 1000} × α_n {14.5, 20, 25} ×
x {0, 0.5} × ρ_fP {0.38, 0.25, 0.1, 0.03, 0.01}, mate 43; fit/BEM − 1, min / median / max; negative under.

| set | n | crate DB (Y_F − axial)·K_f | DB without the axial term | ISO Y_F·Y_S |
|---|---|---|---|---|
| canary 17/43, pinion / wheel | 2 | −4.7 / −11.5 % | +6.0 / +8.8 % | +1.3 / +2.9 % |
| all | 176 | −70 / −23.5 / +7.7 % | −64 / −9.5 / +16 % | −56 / +4.7 / +27 % |
| ρ_f/s_Fn ≥ 0.1 (ordinary) | 81 | −29 / **−14.7** / +7.7 % | −13 / −2.2 / +16 % | −5.9 / **+7.9** / +25 % |
| ρ_f/s_Fn < 0.02 | 29 | −70 / −52 / −33 % | | −56 / −27 / −5 % |
| ISO band 1 ≤ q_s < 8 / q_s ≥ 8 (clamped) | 139 / 37 | | | −22 / +6.4 / +27 % · −56 / **−23** / +14 % |

ISO is the closer fit in its band and 23 % under past its clamp, the error `stress_correction`'s comment warns
of. DB's product is −15 % median on ordinary fillets, almost all the axial term (median 13 % of Y_F); dropping
that term was decided after the fact, against "suspect the BEM first": a question (§6), not a finding. DB's
slope in ρ (−0.11 … −0.21, BEM −0.3 … −0.41) and ISO's clamp fail a Williams law test by construction: **the law
test belongs on the BEM** (within 0.035 of −(1 − λ₁) on the sharpest teeth); DB and ISO need validity ranges.

**The rack (z = ∞), the fillet made sharp** (`t_rack.txt`; F_n 1 N/mm normal to the flank, 0.5m above the pitch
line, α 20°). The BEM peak runs 2.680, 4.390, 8.375, 14.80, 28.76, 38.30 at ρ 0.38, 0.1, 0.02, 0.005, 0.001,
0.0005 mm, local slope −0.4134 at the last (Williams 0.4137; the law holds to 0.001 at 14.5°, 20°, 25°); sharp,
it grows 15.7 → 92.6 as h_min falls 3·10⁻³ → 3·10⁻⁵, as h^−0.40 at first, then −0.35 and −0.27. σ₁ at L₀/2
(4340 Hardened) read on the **bisector / arc midpoint** stays finite: 2.412, 4.174, 7.546, 10.53, 11.05, 10.84,
and 10.531 sharp.
- **The reading convention moves the sharp value 21 %.** The largest σ₁ on the circle r = L₀/2 round the sharp
  root is **12.70**, at 162° (73° off the bisector), ×1.206 (Williams mode I alone ×1.18–1.20). At ρ 0.38 the
  arc midpoint reads ≈ 2.41 against 2.663 on the offset curve, a ≈ 10 % location effect mixed into "support";
  the tooth grid (§3.2) read at the surface peak's normal, which matches the offset maximum there. The sharp
  value is continuous from ρ > 0 and stable to 0.1 % (annealed, L₀ 9.74 µm: 5.417 on the bisector).
- **Its level is uncertain by about ×2, one-sided.** Pairing ΔK_eff,th with the R = 0 Goodman range, the R = −1
  range or its tensile half gives L₀ 1.94 / 0.86 / 3.45 µm and the sharp stress ×1.00 / ×1.40 / ×0.79; 3 MPa√m
  gives ×0.85; and at a truly sharp notch arrest follows the long-crack ΔK_th, 2–3× ΔK_eff,th [C], so the
  reading is high by (L/L₀)^0.41 ≈ 1.8–2.5×. A **finite, mesh-free, conservative regulariser**, not the
  physical sharp-root value; on a generated tooth ρ never reaches 0.

**The measure** (`t_measure.txt`). σ_F stays finite as ρ_fP → 0 at every finite z (17/43 20°: DB 2.839 → 3.735,
ISO 3.019 → 4.019 from ρ_fP 0.38 to 10⁻⁶; ρ_f floors at 0.160 mm). r_lim is "none" at 1.03–1.25× the torques
tried; continuity ≤ 9.5·10⁻⁹. **σ_F(ρ_fP) is not monotone** (DB +0.14 % near the full round at z ≥ 150; ISO
≈ 3·10⁻⁴ on its plateau), so an endpoint-sign root finder can say "none" while a smaller radius passes (§4).

## 6. Open issues, and what is still missing

**Unresolved.**
- **The prototype's unset-state gap** (4–12 %) has a cause beyond the flank convention (< 1 %), not found.
- **DB's −15 %** rests on a BEM without its shoulder canary; whether DB's photoelastic K_f used the
  bending-only nominal (the 1942 bulletin, unread) decides the axial term.
- **The tip's limit** is an upper bound derived in session; the wedge-to-half-plane transition is unmodelled,
  and §4's peak-for-peak comparison is rough. Round 2's "5–15 %" elastic wedge estimate is withdrawn.
- **Melan coefficients on the solved pressure** exist at μ 0.06 only; the tip's traction sign has not been read
  off the kinematics. **Collapse** needs an M–V–N lower bound on the tapered tooth; its sign is open.
- **Coverage:** σ_y readings fit 3 of 8 library materials without argument (the 4340s, brass); the PA GF
  entries (break) have none; shakedown of the viscoelastic POM and PA6 is not grounded.
- **Carried over:** a fictitious edge radius stays dropped (contact is nonlinear: it depends on load); ISO's
  Y_δrelT ∝ √q_s against the true q_s^0.29–0.42 lies outside the crate's q_s < 8 band; EHL, the 3-D face-end
  vertex and dissimilar materials are outside the model; ISO 6336-3 read only through IACS M56.

**Angles not searched** (a fresh search budget is needed):
1. **Measured notched-specimen data across materials**, the owner's validation set proper (Frost; Siebel–
   Stieler; Taylor's tabulated L; Atzori–Lazzarin–Meneghetti; Murakami) and gear pulsator tests with varied
   fillet radius. Everything so far is scored against *formulae* on one alloy in two tempers.
2. **Cyclic properties from common ones:** the Uniform Material Law (Bäumel–Seeger 1990) and the hardness
   method (Roessle–Fatemi 2000), scored against measured K′/n′: the derivation the shakedown gate needs [C].
3. **Limit analysis of notched, tapered cantilevers** (Hill; Green; Ewing [C]): σ_y-only, length-free.
4. **Tip-relief practice** (Sigg; Niemann [C]) and **measured run-in land widths** (Kubo, full text).
5. **Gradient plasticity** ℓ ≈ b(μ/σ_y)² (Nix–Gao [C]) has L₀'s form b(E/σ)²; **surface-notch models**
   (Arola–Williams; FKM K_R) with Rz a drawing input.

**Claims unverified:** Hill's wedge and tilt term; the cyclic 0.53; Marin's 0.85 / 0.65; roughness and grain
ranges; ΔK_eff,th scaled to brass; DB's accuracy per Wilcox & Coleman; Kubo's pitting [A]. **Not prototyped**
(beyond §8's list): the land's effect beyond an estimate; ring members in the BEM; helical fillets; polymers.

## 7. Sources

- **[R] read in full.** Atzori, Lazzarin, Meneghetti, ESIS CP2006, https://www.gruppofrattura.it/ocs/index.php/esis/CP2006/paper/download/9433/6086 · Meneghetti et al., PSM preprint, https://air.unipr.it/retrieve/e177fbc5-f0ec-50b0-e053-d805fe0adaee/PrePrint.pdf · Braun et al. 2020, https://arxiv.org/pdf/2006.10151 · Moore & Hills 2022, https://arxiv.org/abs/2203.07095 · Eames 2026 DPhil (Oxford), ch. 3 and 7, https://ora.ox.ac.uk/objects/uuid:7ba57fe7-1f37-467d-bf5b-18edc79c0f35
- Kumagai, Liu, Kurokawa 2022, https://doi.org/10.2474/trol.17.44 · Paggi & Carpinteri 2008, https://staff.polito.it/alberto.carpinteri/papers/CARPINTERI_2008_N.477_AMR.pdf · Chapetti et al. 2023 (1.3·10⁻⁵E as a fill-in; grain size preferred), https://pmc.ncbi.nlm.nih.gov/articles/PMC10488808/ · Tanaka, Akiniwa, Gubeljak, Chapetti 2024 (3 MPa√m for steels), https://pmc.ncbi.nlm.nih.gov/articles/PMC11433514/
- Pippan & Hohenwarter 2017, https://pmc.ncbi.nlm.nih.gov/articles/PMC5445565/ · TCD formulas, https://pmc.ncbi.nlm.nih.gov/articles/PMC10181184/ · IACS UR M56 Rev.4 §M56.3.11, reproducing ISO 6336-3:2019 Method B, https://iacs.org.uk/resolutions/unified-requirements/ (copy `notch-research/iacs_m56.txt`) · contact-verify5 §2–3; `contact-proto/edge_round.txt`; `strength.rs`; `material.rs`; `materials_default.toml`; `train/mod.rs` `allowable()`.
- **[A] abstract or snippet.** Lazzarin & Tovo 1996, https://link.springer.com/article/10.1007/BF00018497 · Lazzarin & Filippi 2006, https://www.sciencedirect.com/science/article/pii/S0020768305001186 · Lazzarin & Zambardi 2001, https://link.springer.com/article/10.1023/A:1013595930617 · Lazzarin & Berto 2005, https://link.springer.com/article/10.1007/s10704-005-3943-6 · Taylor 1999, https://www.sciencedirect.com/science/article/abs/pii/S0142112399000079
- El Haddad et al. 1979, https://www.sciencedirect.com/science/article/abs/pii/001379447990081X · Araújo et al. 2007, https://www.sciencedirect.com/science/article/abs/pii/S0142112306000776 · Radaj, Lazzarin, Berto 2013, https://www.sciencedirect.com/science/article/abs/pii/S0142112313000194 · Leguillon 2002, https://www.sciencedirect.com/science/article/abs/pii/S0997753801011846 · Sackfield et al. 2003, https://doi.org/10.1016/S0022-5096(03)00020-6
- Giannakopoulos, Lindley, Suresh 1998, https://www.sciencedirect.com/science/article/abs/pii/S1359645498000111 · Ciavarella, Hills, Monno 1998, https://journals.sagepub.com/doi/10.1243/0954406981521259 · Fleury, Hills, Barber 2016, https://doi.org/10.1016/j.ijsolstr.2015.11.031 · Comninou 1976, https://link.springer.com/article/10.1007/BF01594906 · Gdoutos & Theocaris 1975, https://doi.org/10.1115/1.3423663
- Komori, Kubo et al. 2004, https://www.jstage.jst.go.jp/article/kikaic1979/70/700/70_700_3572/_article/-char/en and https://www.jstage.jst.go.jp/article/kikaic1979/70/700/70_700_3581/_article/-char/en · Peterson's constant, https://www.researchgate.net/publication/248524916_On_fatigue_limit_in_the_presence_of_notches_Classical_vs._recent_unified_formulations · Jackson & Green 2005, https://itzhak.green.gatech.edu/rotordynamics/A_Finite_Element_Study_of_Elasto_Plastic_Hemispherical_Contact_Against_a_Rigid_Flat.pdf
- **[M] metadata only.** Kapoor & Johnson 1992, https://doi.org/10.1016/0020-7403(92)90073-p · Dundurs & Lee 1972, https://link.springer.com/article/10.1007/BF00046059 · Mugadu et al. 2004, https://doi.org/10.1016/j.ijsolstr.2003.09.038 · Hills et al. 2012, https://doi.org/10.1016/j.ijfatigue.2012.02.006 · Askes et al. 2013, https://onlinelibrary.wiley.com/doi/10.1111/j.1460-2695.2012.01687.x
- **[C] recalled, not verified.** Williams 1952 (reproduced [X]); Gross & Mendelson 1972; Johnson 1985 (Prandtl; 4k reproduced [X]); Hill 1950 (the wedge, derived [X]); Tabor 1951; Neuber 1958; FKM a_G, b_G; Murakami; Hertzberg's E√b; Dolan & Broghamer 1942 (constants in `strength.rs`); Manson's σ_u/σ_y rule and SAE 4340 cyclic data; Marin; Bäumel–Seeger 1990; Roessle–Fatemi 2000; Nix–Gao 1998; Pavlina & Van Tyne 2008, https://doi.org/10.1007/s11665-008-9225-5 (paywalled).

## 8. Verdict and next steps

**Can this path replace the existing models?**
- **The contact edge refusal: yes in principle, not yet.** Its replacement is a report driven wholly by common
  properties: whether the edge carries load; the relief that unloads it (E, ν, geometry, load); for a stated
  r_e, the flank's own fatigue allowable, first yield, shakedown and the tip's crush; for r_e unset, the
  integrated outputs "before run-in" with the crush land. On a loaded edge the usual answer is "relieve the
  tip, by this much", which the refusal cannot say. Until a pre-registered gate passes, the refusal stays.
- **The bending notch factors: no; the path stays parallel.** No common-property method supplies support
  without a length; the only candidate (L₀) is a steel class constant, ≈ 2× loose at a sharp root; the BEM is an
  instrument, as `verify.rs` is for the cut. The largest finding is about the existing model: **the crate's
  default DB rating is net unconservative on the hardened steel (−2 … −11 % median, worst −26 %) before surface
  finish.** That belongs in `docs/state.md` now; DB or ISO as default is the owner's call once the canary runs.

**The next round (P3)**, each gate fixed before it runs:
1. **BEM shoulder canary** within 0.5 %; clamp the load point for ε < 1 and re-run the four teeth; then settle
   the axial question and write §3.3–3.4's net records for `docs/state.md`.
2. **ISO past its clamp by the Williams law** on the 37 clamped teeth. Pass: median within ±5 % of the BEM, no
   tooth below −10 %. Then **fillet r_lim** by scan-then-bracket, continuity in every input.
3. **Find the prototype's unset-state gap**, then build `edge_outer` into it. Pass: unset within 0.5 % of the
   r_e 0.001 mm values on spur, β 0.5°, β 2° and the ring. A failure keeps the refusal.
4. **The unloading relief** in closed form. Pass: the whole-mesh edge load falls to 0 at it within 1 %, all
   four cases; then compare it with published tip-relief practice.
5. **Edge limits** from the solved pressure, C continuous in μ over 0–0.3, the whole pitch searched, the
   bracket from the tip land, the companion ratio reported, continuity in E, ν, μ, geometry and T (floor 10⁻⁷).
6. **The tip's limit load** checked independently of Hill's field (P1 step 5, rigid- or elastic-plastic),
   across the wedge-to-half-plane transition. **Collapse** as a lower-bound limit analysis in M, V and N,
   gated on `ultimate_measure = "yield"`.
7. **Cyclic gate** (Manson's rule, the UML) scored against measured K′/n′; then, with a new search budget, the
   measured sets of §6, scored as the validation they are.
