# Adversarial verification of work/spike-contact.md

Author: spike-verify, 2026-09-26. Read-only on the repo, no cargo. My own scripts, which share no code
with spike-contact, are in `~/.cache/gearcalc-work/spike-verify/`:
- `rigid.py`: an exact rigid mesh of two face-bounded involute helicoids on skew axes. **Both** bodies'
  flank surfaces are sampled, so an edge of either gear on the other's flank is caught, with tip and
  face edges included. It gives the freedom of relative advance at each phase. Play is the **minimum
  over phase** of that freedom, and a₀ is the root of play(a).
  - Validated against ISO 21771's backlash for spur gears to 1e-12, and against inv α_w (a_par) at
    Σ = 0. With the contact in the face it gives exactly the linear law, 2 sin αₙ·Δa.
- `loa.py`: the crossed line of action from the common-rack construction. It is tangent to both base
  cylinders to 1e-15. It gives z* and the tip zone.
- `closed.py`: my own derivations of the involute-helicoid curvature (tangent developable,
  κ = cos β_b/ρ_t), a Hertz solver (Johnson), the Winkler k, efficiency and pressure.
- `kweights.py`: the per-pair k_j at small Σ.
- The other `t*.py` and `run_a0.py` produce the tables below. The a₀ table is saved as `a0.txt`.

## Claim 1: the distance closed form (≤ 2.3 µm) and play (~1 %). PARTLY.

**The form is derived, not fitted.** If the lengthwise gap is a parabola of vertex z* ∝ Δ/Σ, then
a₀ = a_lin − D₀(1 − b/2z*)². The residual comes from the gap not being a parabola.

**Against my exact geometry, on the spike's own rows:**

| row | model − exact |
|---|---|
| 17/23 x .5/.5, 10 mm, Σ 1° / 5° / 10° | +0.55 / **+2.46** / +1.14 µm |
| same, 30 mm, Σ 1° / 5° | +0.62 / +1.75 µm |
| 17/43 x .8/.2, Σ 3° / 12° | +0.51 / +1.53 µm |
| 17/23 x −.3/−.3, Σ 1° / 3° | **−1.45 / −2.13 µm** |

On the negative-shift rows the model's a₀ is too low, so it overstates play. It is not "slightly
high" everywhere.

**On rows the spike did not test:**

| row | model − exact | share of D₀ |
|---|---|---|
| x 1/1, 10 mm, Σ 2° | +3.3 µm | 0.9 % |
| x 1/1, 10 mm, Σ 6° | **+6.4 µm** | 1.8 % |
| 17/43 β10, Σ 4° | +0.9 µm | 1.0 % |
| asymmetric faces 6/20 and 20/6 | ≤ 0.44 µm | |

So the bound that holds is **≤ 2 % of D₀**, not an absolute 2.3 µm.

**The spike's "exact" tool (geom.py) is not exact to the µm it quotes.**
- It samples gear 1's surface only.
- It sums the per-side minima, each taken at its own phase.
- It differs from mine by up to 2.9 µm. At its own a₀ for −.3/−.3 at Σ 3°, a dense brute-force grid
  finds 0.9 µm interpenetration per side, and a min-over-phase play of −1.4 µm.
- It needs fixing before it becomes `tools/skew_gap.py` with a 5 µm gate.

**Play.**
- My exact values agree with the spike: 42.42 µm at Σ 0, 43.02 at 0.01° and 72.0 at 0.5°. The
  crate-style projections (−41.2, −40.6, −15.0 µm) are also reproduced.
- The *reported* jump is 42 µm (from 42.4 to the floor at 0). The 83 µm is the unfloored difference.
- The slope formula is a linear interpolation in u, not a derivation. Its error is ≤ 0.6 % on the
  spike's points but **−2.35 % at Σ 10°, 10 mm** (exact 0.812, formula 0.793).

## Claim 2: the Σ = 0 reductions. CONFIRMED, but they are identities.

- ε_α = 1.49188, ε_β = 1.08868, ε_γ = 2.58056.
- 593.5/√ε_α = 485.9 MPa.
- The first-order efficiency is 98.777 % from the parallel formula, which I derived myself.
- Mean loaded length = ε_α b/cos β_b and mean line count = ε_γ are properties of any lines that fill
  the rectangular field. With κ_L = 0 and a common δ, the share ∝ L is ISO's assumption.
- 2.583 is 124/48, a staircase from the phase quadrature.

## Claim 3: the worm. CONFIRMED for the geometry.

- My independent curvatures are 0.081615 and 0.173242 /mm. The ellipse is 2a 1.8448, 2b 1.1184, p0
  3940.3 at F_n 4256.7 N.
- The Winkler parabola reproduces the Hertz line load exactly.
- The spike's ellipse is on the worm torque with no friction. The golden rates on the wheel torque
  (1.6328 × 0.9898 mm, 3487 MPa), so the spike compared its ellipse to itself, not to the golden.
- The pitch-point efficiency and the BS 721 thresholds are unchanged by construction.
- The reported worm efficiency moves by about −1 point, which the spike admits. It comes from the
  tip-edge "spill" patches: 2.28 loaded pairs against ε 1.83. Those are Hertz patches on what are
  tip-edge contacts. Not verified independently.

## Claim 4: continuity, and whether k is a tuning constant. PARTLY.

**k is not a free constant.**
- k = (π/2)E*e²/(K(e) − E(e)) = 3πE*/(2R_D((b/a)², 0, 1)), where the argument is the aspect ratio
  b/a, not the curvature ratio.
- It is load-free and exact for the untruncated ellipse.

**But it is extrapolated.**
- k → 0 logarithmically as Σ → 0. k/E* is 0.81 at B/A 10 and 0.15 at 1e8.
- The prototype therefore **branches**: `kf ≤ 1e-14 → k = 1`. So "not a branch" is false in code.
- When truncated, k is set by the untruncated a, not by the line length. The pair weights then differ
  by about 5 % at 0.001° and 7 % at 0.1°, which moves the pitch pressure by 0.4–0.6 %. It converges
  to ∝ L only as 1/ln(1/Σ).

**Continuity.**
- ε_path and the truncation law are continuous; the law is C¹ at r = 1, which I checked analytically.
- The "ε = loaded-pair count" is a phase-quadrature staircase unless it is integrated in closed form.

## Claim 5: against the audit. CONFIRMED.

- z* = 148 mm at 1° for x .5/.5, from my own line of action.
- On the fixture of T07.2's first gate (17/43, Σ 0.5°, +0.02 mm, 10 mm faces), the rigid pair has
  play 13.68 µm at every phase. The binding point sits inside the face (z 2.6–4.9 mm), because the
  gap along the face is flat to well under 0.1 µm. T07.2's "ε 0" and T07.3's refusal describe a
  near-parallel helical pair as not meshing.
- T07.4's premise, "loses contact for Σ ≲ 2°", is false.

## Claim 6. CONFIRMED with caveats.

- ε_path → ε_α/cos²β_b: 1.66373 against 1.66373.
- The 48 % is √(31.68/14.47). **The clipped line is 14.47 mm = g_α/sin β_b, not the "15.5 mm" the
  spike's text states.**
- It is not an unconservative error. A single line carrying all the load never occurs at ε_β > 1, and
  the crate's 342.6 MPa is above the physical shared 278 MPa.

## Overall judgement

The model is reasoned, not curve-fitted:
- the parabolic-gap distance law;
- the Hertz-calibrated Winkler model, with no free constant;
- the exact truncation algebra.

It meets every closed form where one applies.

**Cost.**
- The distance and play laws are closed form.
- The field needs about 48 phases × pairs × (an aspect root + 64-point quadrature) plus a δ root per
  phase, which is about 10⁴ evaluations against 1 today.
- The Σ = 0 closed form survives only as a separate fast route.

**Where it is worse than today.**
- The worm efficiency changes by about −1 point, driven by the tip-edge spill.
- Face-edge contacts at small Σ on shifted pairs are rated as smooth Winkler patches with no edge
  stress. The lengthwise mismatch is about 5 µm across a 10 mm face at 1°.
- The model's a₀ is too low, by ≤ 2 µm, on negative-shift pairs.
