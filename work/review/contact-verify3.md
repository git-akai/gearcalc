# contact-verify3: adversarial check of work/contact-model.md round two

Author: contact-verify3, 2026-09-29. Read-only; no cargo. Scripts in `~/.cache/gearcalc-work/contact-verify3/`:
- `v1_kernel.py` → `v1_kernel.txt`: own Carlson R_D, own Gauss-Legendre quadrature; G, G′, asymptote; the 2-D line limit from McEwen's subsurface stresses; the finite-segment approach by direct Boussinesq quadrature. No prototype code.
- `v2_found_iso.py` → `v2_found_iso.txt`: the J integral behind L*; M* from Flamant; Sainsot's fit at z 17 and z 4000; 2-D Hertz scaling of the edge; ε_α, Method B, ring M1/M2, and ISO arithmetic.
- `w1_proto_re.py` → `w1_proto_re.txt`: **the subject run as-is** (as v3b was last round, not used as a check). Coupled + edge rule, 20 N·m, C_a 0, β 0/0.1/0.5, with r_e 0.2 and 0.1 mm.

## 1. Coupling worsens the β→0 jump: CONFIRMED on the kernel and the physics; PARTLY on "worsens"

- **G matches.** Independent quadrature agrees with the prototype's G to <1e-12 at r 1e-4…50.
  - G′ quadrature against (4/3π)(1+r²)R_D(0,r²,1+r²): −2e-15.
  - The asymptote ln 4r + ½ + 1/(16r²) is 2.7e-14 off at r 1e3.
- **The 2-D line formula is exact.** McEwen's σ_z and σ_x, integrated as plane-strain ε_z over depth, give asinh(h/b) − ν/(1−ν)·h/(√(h²+b²)+h) to 1e-15 at b/h 0.05…3.3.
  - The quoted "3e-12" holds only at b/h 0.05.
  - The prototype's own kernel.txt shows +2.4e-8 at b/h 0.25 and +1.1e-3 at b/h 1. That error is its 4-point Gauss–Chebyshev across b in the depth term, not the formula.
  - Fix: more nodes when b/h is not small.
- **Ellipse convergence is O(N⁻²).** q₀ errors fall by 4.05× and then 3.99× per doubling of N.
  - p₀ and b are reconstructed through Hertz's own C(q), so part of the "Hertz reproduced" result holds by construction. Only q(y) along the line is a real test.
- **"Stiffer when short" is physical.** Take a uniform q with a Hertz cross-section, b 0.03, h 1.
  - The centre approach per unit q is 0.43 / 0.58 / 0.81 / 0.97 / 1.00 of the infinite line at ℓ/h 0.05 / 0.1 / 0.3 / 1 / 3.
  - The reason is 3-D decay: the field of a short patch never reaches the reference depth.
  - A plate-coupled tooth has the same sign. So the previous verifier's hypothesis is correctly refuted.
- **"Worsens" does not hold at every relief.** The coupled jump against the slices jump is +26.7 vs +23.7 % at C_a 0, **+21.4 vs +29.6 % at C_a 2 µm (coupling better)**, and +42.7 vs +27.5 % at C_a 5 µm. The honest statement is "coupling does not cure it."

## 2. The tip-edge rule as the cure: PARTLY. The geometry is right, but continuity fails once r_e is stated

- **The geometry is physical.**
  - The rigid gap along the wheel's tip edge from the conjugate line is ≈ κ_A(s sin β_b)²/2, which is ≪ 0.1 µm over the whole face at β 0.1°.
  - A real gear under a few µm of approach therefore does touch along that edge. This is the known load-extended or corner contact outside the path of action.
  - The prototype's own numbers show the mechanism. Without the rule, the loaded length L/(b/cos β_b) drops from 1.862 (β 0, tip contact extended by load) to 1.618 = ε_α (β 0.1°). With the rule, β 0.1° gives 1.862. The jump was lost contact, not a pressure artefact.
- **But continuity holds only under the flank-curvature convention (r_e unset).** Run with r_e stated (w1):

  | r_e | β 0 | β 0.1° | β 0.5° | Jump at 0.1° |
  |---|---|---|---|---|
  | 0.2 mm | 3198 | 5258 | 5309 | **+64 %** |
  | 0.1 mm | 3198 | 6691 | 6757 | **+109 %** |

  - The β 0 figure does not move with r_e. The same is true of the spur ring in ring_kA.txt: 12433 at all three r_e.
  - The cause is in `coupled.py:92-124`. 1/r_e is applied only to edge pieces (β > 0) and to zero-length tip segments. A spur line lying on the tip edge, including its load-extended part beyond ε_α, keeps the flank curvature.
  - So claims §2.1 ("continuous at every relief and load") and §2.2 ("r_e removes the special case") contradict each other. The jump has moved into how r_e is applied.
  - Fix: treat the tip edge as one geometric object at every β. Any station whose contact point lies on a member's tip edge takes that edge's curvature, whether the station is an edge piece, a spur tip line or an anchor.
  - Size: +64…+109 %, same sign as before. High confidence.
- **The r_e sensitivity matches 2-D Hertz.**
  - At equal load, p(0.1)/p(0.2) = √((10+κ₂)/(5+κ₂)) = 1.4175, with κ₂ ≈ −0.046/mm for the concave ring flank. The prototype gives 1.3976; the 1.4 % gap is load shed by the sharper edge.
  - Edge load is 79 N/mm, and b/r_e is 0.095, so Hertz is valid.
  - A singularity as r_e → 0 is correct elasticity (a wedge corner).
- **Wording error in the doc.** "1204 → 3818 → 5336 at the pinion's tip edge" is wrong for 1204. That maximum sits at r1 16.622, the SAP and face end. Only the r_e cases sit at r1 20.200, the tip edge.

## 3. Derived foundation: CONFIRMED (closed forms); stiffness numbers reproduced where independent

- **L\* checks.** J = ∫∫_{[−½,½]²} tτ ln|t−τ| = −0.0625000000 exactly (K(u) is a cubic; log moments are exact).
  - With σ = 12Mx/S³ and a work-conjugate rotation: L* = −288 J(1−ν²)/π = 18(1−ν²)/π = 5.2139.
- **M\* checks.** Flamant's tangential–normal coupling, with c = (1−2ν)(1+ν)/2E and uniform shear, gives θ = −2cF cos α/S.
  - Doubled by reciprocity, that is M* = 2(1−2ν)(1+ν) = 1.040.
  - This is identical to Weber–Banaschek/Cornell's 2(1−ν−2ν²).
- **L\* differs from Weber.** Weber's L is (1−ν²)·16.67/π = 4.83 (my recollection of the Cornell form, medium confidence). The derived 5.21 is 8 % higher; Weber did not use a work-conjugate rotation.
- **Sainsot at z 17** (h_f 1.4, θ_f 0.12–0.18): L* 6.87, M* 1.05–1.09.
  - The annulus's L* is 32 % above the half-plane's. That is consistent with the derived model being stiffer: 15.60 against 13.65.
  - This is a sign-known bias, not an error. A half-plane omits the finite body's rotation compliance.
- **Sainsot at the rack.** At θ_f ~ 4e-4 (z 4000) it gives L* −343, M* 3.8e3, P* −3.2e3. The fit clearly fails, confirming "2300×" in kind; I did not rerun the exact 44.65 µm figure.
- **ISO recomputed.** q′ 0.062376, c′_th 16.032, c′ 12.505, ε_α 1.6211, c_γ 18.330.
- **Not recomputed.** The 15.60 itself, which needs the full generated-tooth model.

## 4. ISO numbers: CONFIRMED, with one caveat

- **Arithmetic.** 1.0702 × 1522.6 = 1629.5 and 837.8 × 1.0157 = 851.0. Z_B = 1.0889 − 0.8238 × 0.0889 = 1.0157.
- **Ring 17/−43 m2 x0/−0.3.** Recomputed with signed radii: a −26.5579, ε_α 1.8301, M1 1.3447, M2 1.1739, σ_H,B 886.6.
- **Caveat (medium confidence).** ISO 6336-2 takes Z_D = 1 for internal gears. The "ISO at D 773.9" is the signed extension, not ISO.
  - Against ISO proper (659.3), the wheel reading 719.7 is +9.2 %, not −7.0 %.
  - Label the D column "signed-radius extension".

## 5. Matched ZN worm: CONFIRMED (structure); the comparison is not like-for-like

- **The three figures differ in three assumptions:**
  - **Sharing.** Crate: 1 point, full F_n = 2951 N. Involute field: 3.31 points. Matched: 2.05 lines of 8.1 mm.
  - **Location.** Crate: the pitch point. Both fields: the worm tip, r 4.500.
  - **Contact type.** Point contact in the crate and the involute field; line contact in the matched set.
- **What is common.** All three use the wheel torque 54.953 N·m and E* 70811; η uses the worm torque 2 N·m.
- **Both field maxima are unrelieved worm-tip-edge figures** under the flank convention, so §2's r_e caveat applies to them too.
  - The mid-line pressure of the matched set is lower still.
  - "−15 % / −47 %" compares an edge figure with a pitch-point Hertz figure.
- **"No new kind of solve but a second wheel geometry" is confirmed by the code.** `matched.py` uses a quadratic u(v), bracketed clips, and Δ closed form on g ≡ 0. The wheel is the worm's envelope, with no gap off the line.
- **The phase is still scanned** (7.2 M evaluations), although φ(u,v) is explicit (contact-verify2 claim 7).

## 6. Cost: PARTLY

- **The as-built arithmetic reproduces from the unit costs:**
  - spur 0.66 s and 2.06 s;
  - ring coupled 1.09 + 0.99 + 4.42 + LU ≈ 6.6 s.
- **The slices upper bound of 4.6 s is stale.** It is the ring with 25 pieces, which the doc itself attributes to the withdrawn 2300× tooth. With 6 pieces the slices ring is about 1–1.5 s.
- **The 35 ms fast route arithmetic is right** (250 × 141.6 µs), but the "kernel ≈ 60 µs" is not derived.
  - N² × 2 bodies × 200 ns of depth panels is 230 µs per line, which gives ≈ 80 ms.
  - 60 µs holds only if the depth matrix is Toeplitz (uniform panels, h constant along the line). That fails on helical lines crossing heights, where h varies.
  - So the true range is 35–80 ms, at the top of or above the ×2 band.
- **The verdict stands:** fit for the final solve, not for the search.
