# contact-verify2: adversarial verification of work/contact-model.md (phase-exact contact prototype)

Author: contact-verify2, 2026-09-29. Read-only on the repo, no cargo. Scripts (my own, sharing no code
with contact-proto; the prototype is only *run* once, as the subject, in v3b) in
`~/.cache/gearcalc-work/contact-verify2/`:

| script | checks |
|---|---|
| `v1_stiff.py` | ISO 6336-1 Method B (q' polynomial, C_M, C_B), and an independent idealised-tooth PE stiffness (Tian/Liang geometry + Sainsot) |
| `v2_iso.py`, `v2b_ring_B.txt` | ISO 6336-2 Z_H, Z_E, Z_eps, M1/Z_B, sigma_H0, and exact single-pair Hertz at C and B |
| `v3_field.py` | my own minimal Sigma=0 slice field (plane of action, relief as gap, Winkler) at 2, 20, 60 N m |
| `v3b_proto_load.py` -> `.txt` | the prototype itself at 20 N m (10x the load of its relief table) |
| `v4_bpp.py`, `v4b_eps.py` | own Carlson R_F/R_D vs direct numerical Boussinesq integration of B''; line limit; eps and corner spacings |
| `v5_worm.py` | worm efficiency table, locking thresholds, Hertz ellipse from own elliptic integrals |
| `v6_backlash.py` | ISO 21771 j_n from my own thickness formulas (the prototype's "ISO" check reuses the oracle's own `v()`) |
| `v7_ring.py`, `v7_check.py`, `v7_ring.txt` | ring-tip corner path against the pure involute AND against the rack-generated pinion (point-in-swept-tool test) |

## Claim 1 -- tooth compliance. PARTLY (numbers right; the "~14/20" gate is ISO Method C, and the value is +-15 % model-dependent)

- **Model**: potential-energy beam integrals (Yang-Sun/Tian: bending, shear k=1.2, compression, plane-strain E')
  over the *rack-generated* profile (involute + ISO 53 A trochoid), plus the Sainsot-Velex-Duverger 2004 fillet-foundation
  term (**fitted** polynomial coefficients, cited; table matches the published one), plus a derived Weber/McEwen
  depth-referenced Hertz term. Derived from the generated tooth except the foundation, which is 44 % of the compliance
  and carries one free input, h_f = r_f/r_int = 1.4 (the bore; 1.2..2.0 moves c' 13.94..12.72). Not fitted to ISO.
- **ISO 6336-1 Method B, recomputed** (17/43 x0, E 206 GPa): q' 0.062376, **c'_th 16.032**, **c' = c'_th C_M C_R C_B = 12.505**
  (C_M .8, C_R 1, C_B .975), **c_gamma = c'(0.75 eps_a + .25) = 18.330** (eps_a 1.6211), from c'_th 23.500. All match stiff.txt.
- Model c' 13.65 is +9.2 % over Method B's c' and -14.9 % under c'_th; c_gamma 19.79 is +8.0 % over Method B.
  "~14 / ~20" are ISO **Method C**'s constants, not a Method B result -- say so.
- **Independent PE** (idealised tooth: straight constant-thickness section below r_b, no generated fillet; my code):
  **11.6** (plane strain) / 11.5 (plane stress), h_f 2.0: 10.9. The 15 % gap to 13.65 is the fillet geometry
  entering Sainsot's theta_f (0.118 idealised vs 0.177 generated). The generated fillet is the better input, but
  the spread shows "c' within ISO" is a statement about a model family, not a measurement.

## Claim 2 -- ISO 6336-2. PARTLY: arithmetic confirmed; the "point B 0.0 %" is mislabelled and both spur agreements are identities

- Recomputed FZG-C (as the prototype defines it): a 91.5, alpha_wt 22.4388, eps_a 1.4624, Z_H 2.3419, Z_E 189.81,
  **Z_eps 0.9197, sigma_H0 1522.6**, M1 1.0702. Exact single-pair Hertz: **C 1655.6, B 1771.8 at r_1 35.403** -- matches.
- **ISO's sigma_H at B is Z_B sigma_H0 = 1.0702 x 1522.6 = 1629.5 MPa** (sigma_H0 contains Z_eps). The script's
  "point B Z_B*" is Z_B x sigma_H0 / Z_eps. So the field max is **+8.7 % over ISO at B**, not 0.0 %. Both "exact"
  spur matches are identities: in the single-pair zone any model puts all the load on one pair, so they test the
  Hertz arithmetic, not load sharing.
- "FZG-C" uses untrimmed tips (d_a 82.64/118.54); the real FZG C gear has 82.46/118.36 (eps_a 1.4375, M1 1.0748,
  B 1779.4). Label it "FZG-C-like".
- Helical sigma_H0 recomputed: 17/43 **961.3** (Z_eps .8187, eps_b 1.0887, Z_B 1); 20/60 **837.8** (Z_eps .8116). The
  +1.0 % / +3.6 % stand. But 20/60 has eps_b 0.824 < 1, so ISO's own Z_B = M1 - eps_b(M1-1) = 1.0157, and ISO sigma_H = 851.0.
  That makes the pitch +2.0 % and the field max +42.6 %, not +45 %.
- The ISO comparator's M1 is **wrong for rings**: 1794.8/775.2 = 2.315 for 17/-43. The signed single-pair ratio
  sqrt(k_B/k_C) is **1.345** (v2b). Not in the headline claims, but the script prints it.

## Claim 3 -- field max at the unrelieved SAP, the beta 0 -> 0.1 jump, and the relief cure. PARTLY; the cure is REFUTED as structural

- **Mechanism reproduced independently** (v3_field, constant c'): a spur line enters whole, and Delta drops to the
  two-pair level at once. A helical line enters as a vanishing corner segment while Delta is still at the single-pair level.
  The Winkler slice's q = c'(Delta - g) is local and independent of segment length, so the max over phase jumps by
  sqrt(Delta_1/Delta_2): mine 1140 -> 1609 (+41 %), the prototype's 1028 -> 1287/1337 (+30 %).
- **Not a clipping singularity**: q is bounded, and there is no end singularity in a Winkler clip. It is a sup over a
  vanishing phase window of a quantity the slice model makes length-independent. That is the uncoupled-slice
  assumption, applied below the scales where it holds (segment << tooth height and << the Hertz width). A coupled
  model would give a continuous limit in beta: plate influence coefficients (Conry-Seireg/LDP-type) and a half-space
  that couples the slices.
- **What is physical**: unrelieved helical gears do load the entering corner. Corner/edge contact at the start of
  engagement, extended by deflection, is the textbook reason for tip relief (LDP/Houser-type load distribution
  studies; ISO/TR 21771 C_a, L_a). In real elasticity a sharp tip edge is singular, so no finite "field max at an
  unrelieved SAP" is well-posed. The +45-54 % is a model output, not a property of the gear.
- **The relief cure holds only at the tested load.** The relief table runs at 2 N m = 23.5 N/mm, where the approach is
  about 1.8 um < C_a = 2 um: the tips are simply unloaded. **The prototype itself at 20 N m** (v3b):
  - C_a 2 um: **2817.9 -> 3660.1 MPa (+30 %)**;
  - C_a 5 um: **2313.7 -> 2903.2 (+25 %)**;
  - C_a 0: 3237.7 -> 4023.3 (+24 %).

  Mine agrees: the jump persists whenever Delta_single > C_a, and C_a 15 um cures it at 235 N/mm but not at 706 N/mm.
  Continuity in beta is therefore conditional on relief sized for load, not a property of "relief as a gap". The
  owner's call in §6 must be framed that way.

## Claim 4 -- Sigma -> 0, B'', eps, anchoring. CONFIRMED (math), PARTLY ("no branch")

- **B''** (all four displacement formulas) agrees with direct numerical integration of Boussinesq over the Hertz
  ellipse to about 1e-6 at 4 (a,b,h) sets. u0 - ua = R_D(beta^2,0,1)/3 to every digit. The line limit
  asinh(h/b) - nu/(1-nu) h/(sqrt(h^2+b^2)+h) is approached as **-0.286 h/a**; the doc says -0.29 h/a. So Sigma = 0 is a
  value.
- **"No branch" is not yet true in code.**
  - The prototype switches to the line formula at kappa_L == 0 and skips pairs with kappa_A <= 0.
  - `cost.txt` records a **ZeroDivisionError at kA = 0** (field.py:208) on the ring case. The ring then reran in cost2.
  - In Rust, a series inside the special function below a threshold is acceptable, but the kappa_A <= 0 skip is a
    model branch, and it is unresolved.
- **eps** (own closed form, 17/43 beta20 b10): eps_a 1.4918785, eps_b 1.08868, eps_gamma 2.580562. The corner
  fractions 0, .08868, .49188, .58056 give exactly the spacings .0887/.4032/.0887/.4194 claimed.
- **Anchoring**: 1.36421 is exactly ISO eps_a of **17/43** x.5/.5 beta20 at a_par. The quoted "parallel 1.23" is
  **17/23** x.5/.5 beta20 at **a_lin** (1.2336; at a_par it is 1.3121). The paragraph compares two pairs and two
  distances. The cure's number is right; the 1.13 (spike code) was not re-run.

## Claim 5 -- worm. CONFIRMED (closed forms); sharing figures PARTLY

- Recomputed with my own code:
  - **efficiency table**: 86.882/76.751/68.691/56.677 and 84.993/70.078/55.254/25.874;
  - **locking**: 6.5104 = cos a_n / tan gamma, 0.1356 = cos a_n tan gamma (gamma 8.2132°);
  - **F_n** 2951.2;
  - **Hertz**: 1.6328 x 0.9898 mm and 3487.4 MPa, from kappa 0.081615/0.173242 and E* 70811.

  All match `tools/golden/worm_1_40_7_90.txt`. They are the same closed forms, so reproducing them is necessary, not
  evidence for the field.
- **Sharing**: the drop is 0.65-2.20 pt, which checks. The peak change is **-5.0 / -10.0 / -37.7 / -39.6 %**, so the
  range is -5...-40 %, not -38. The case is 3487 MPa on C360 brass (far past yield), at deflections that bring 3.3 pairs
  into contact, so the sharing percentages are load-specific. Not recomputed independently.

## Claim 6 -- oracle. CONFIRMED; the ring interference is REAL and larger than the oracle reports

- **ISO 21771 j_n from my own thickness formulas**:
  - 17/23: **34.5219**;
  - 17/43 beta20 x.5/.2: **38.6366**;
  - ring 17/-43 beta15 x.3/-.3: **33.7489**;
  - ring 20/-60 beta20: **37.7174 um**.

  All four match. Note that `t_oracle1.iso_jn` reuses the oracle's own `v()`, so the prototype's "ISO" was not independent.
- **Interference, 17/-43 spur x0**: the standard condition needs |r_a2| >= sqrt(r_b2^2 + (a sin a_w)^2) = **20.687**
  against 20.5, so the interference is real.
  - Against the pure involute (above r_b1) the ring's tip corner penetrates **1.32 um**. The oracle says 1.63; same
    order, but the measures were not reconciled.
  - **Against the rack-generated pinion** (ISO 53 A; point-in-swept-tool test, sanity-checked to 0.000 um on the
    involute) the corner goes **24.5 um into material at rho 7.635 < r_b1 7.987**: fillet/undercut interference.
  - The oracle cannot see it because its flank patch starts at max(r_b, r_f).
  - x2 = -0.3 clears by 103 um on both measures.
  - So `flank_interference` must be checked against the generated root, not the involute alone.

## Claim 7 -- worm types. PARTLY

- **"No branch for flank type"** holds only for **ruled** helicoids (ZA, ZN, ZI). ZK (cone-milled) and ZC (concave)
  are not ruled and are outside the one-surface family. mu_ZN 19.79° checks: asin(sin 20° cos gamma).
- **"Matched set needs an extra solve because the lines are not linear in phase" is overstated.** For any
  screw-invariant worm on fixed axes, the fixed-space surface at phase phi is S_0 + p phi z, and v_12 is affine in
  position. So the meshing equation is n.v_12 = f(u,v) + p phi g(u,v): **exactly linear in phase at every surface
  point**, and phi(u,v) = -f/(p g) is explicit. The contact lines are level sets of a closed-form function. Only the clip
  at the *wheel's* rotating boundaries needs a 1-D bracketed root, the same class of root the involute-wheel field
  already spends 95 % of its time on (anchor search).
- The prototype's test (the fixed-frame point moves non-linearly) is true but does not imply the conclusion. Also, hobbed
  wheels are normally cut oversize, so "line contact" is the ideal matched case, not practice.

## Claim 8 -- cost. PARTLY

- The evaluation totals are **extrapolated**: one state's count x the number of Gauss states (and `t_cost.py` prints
  "gap evals 0" through a dead expression). Whole-mesh time / one-state time gives 312, 455, 3580 and 337
  state-equivalents against 168, 280, 1400 and 224 reported. Breakpoint bisection and the nominal solve are uncounted,
  so the true counts are about **1.5-2.6x higher**.
- At 80 ns/eval "as built" is 49 ms (spur) to **1.0 s (ring)** and 0.4-0.56 s (worm), before that factor. The
  doc's "50-110 ms per mesh" omits the ring and worm rows.
- The "~5000 evaluations, 0.5 ms" fast route has no derivation. Even 1 anchor eval + 16 slices per pair per state
  is about 8k evaluations for spur and about 67k for the ring's 1400 states, and a Delta root per state comes on top.
  Expect 1-10 ms.
- **The cheaper exact route exists and is stronger than stated.**
  - At Sigma = 0, the anchors (the plane of action), clips (linear) and breakpoints (the four corners, shown above)
    are all closed form. Only Delta per state is a root, so microseconds to tens of microseconds per mesh are
    plausible, and even the search could afford it.
  - At Sigma != 0, the interior anchor lies on the closed-form crossed line of action.

## New findings (not in the doc)

1. The relief cure of the beta jump is load-conditional (v3b, the prototype's own numbers).
2. The ISO point-B "0.0 %" omits Z_eps: ISO's rating at B is 1629.5, and the field is +8.7 % over it.
3. Ring tip / pinion fillet interference is 24.5 um, which the oracle cannot see below r_b.
4. The ring M1 comparator is invalid.
5. The kappa_A = 0 ZeroDivisionError.
6. The cost counts are extrapolated, and the Rust ranges understate the ring and worm.
7. The matched worm set's phase is explicit (linear meshing equation).
