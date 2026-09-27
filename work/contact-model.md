# Unified contact model: phase-exact prototype — IN PROGRESS

Author: contact-proto, 2026-09-26. **Status: paused at the session limit after the reading phase.
No prototype code has been written and no validation has been run by this author.** Nothing below
is a result of mine; the figures quoted are the spike's or the verifier's and are marked so.

Inputs read: `work/plan.md` §5 (owner's ruling), `work/spike-contact.md`,
`work/review/spike-verify.md`, and the verifier's scripts in `~/.cache/gearcalc-work/spike-verify/`.
Scripts will live in `~/.cache/gearcalc-work/contact-proto/` (README there; empty for now).

## 1. Equations taken as settled (derived by the spike, checked by the verifier)

- Winkler slice stiffness from the pair's own ellipse: k = 3πE*/(2 R_D(β², 0, 1)), β = b/a the
  aspect ratio (not the curvature ratio). Load-free; reproduces the untruncated Hertz line load
  q₀(1 − y²/a²) exactly (verifier: worm check q₀ = kκ_L a²/2 to the digit).
- Face/tip truncation of a symmetric patch, r = L/2a ≤ 1: q_max = (P/L)(1 + r³/2); C¹ at r = 1.
- Crossed distance law a₀ = a_lin − D₀u², u = max_i(1 − b_i/2|z*_i|)₊: derived from a parabolic
  lengthwise gap; residual ≤ 2 % of D₀ (verifier), sign varies with shift.
- Verified identities at Σ = 0 (17/43 β20, 10 mm): ε_α 1.49188, ε_β 1.08868, ε_γ 2.58056;
  shipped worm curvatures 0.081615/0.173242 1/mm, ellipse 1.8448 × 1.1184 on the worm torque
  without friction; the golden rates on the wheel torque (1.6328 × 0.9898 mm, 3487 MPa).

## 2. Defects to fix before the new model is built on them

1. The spike's k branches (`kf ≤ 1e-14 → k = 1`), and k → 0 like 1/ln(1/Σ) — the Σ → 0 limit must
   go through R_D's logarithmic asymptote (task item 5), and in series with tooth compliance the
   branch disappears because the series stiffness tends to the tooth's.
2. The play slope is an interpolation in u (−2.35 % at Σ 10°, 10 mm): derive it from the gap
   parabola (task item 7).
3. ε and loaded-pair counts are phase-quadrature staircases (2.583 = 124/48): replace by
   breakpoint integration (task item 3).
4. The oracle (`rigid.py`) is itself sampled: an NR×NZ grid plus pattern search per phase, and a
   12-phase scan with golden refinement for the minimum over phase. Fixes planned: bracket every
   flank-point minimum along the known contact line (the analytic line of action gives the
   starting point), take the phase minimum from the freedom's breakpoints, and add a concave
   (internal) body — it has none today, so the ring validation needs an extension.
5. `loa.py` finds base-cylinder tangency by ternary search; it is a quadratic in s, closed form.

## 3. Remaining work (in order)

1. Oracle fixes above, re-run the verifier's `run_a0.py` table as a regression.
2. Contact lines per phase in closed form; clip by face planes and tip cylinders (quadratics).
3. Tooth compliance from the generated form (potential-energy method: Yang & Sun 1985, Tian 2004,
   with Sainsot et al. 2004's foundation term), in series with the Winkler Hertz slice; gate
   against ISO 6336-1's c′ ≈ 14, c_γ ≈ 20 N/(mm·µm) at the standard example.
4. Common approach δ per phase piece: the load is piecewise polynomial in δ (linear clipped
   segments, parabolic gaps), so each piece is a cubic/quadratic solved by Cardano; pieces from
   sorted breakpoints.
5. Means and maxima over phase from breakpoints and stationary points; ε exact.
6. Validations: oracle (distance, play, contact positions), ISO 21771 ε, ISO 6336-2 σ_H with Z_ε,
   the shipped worm on the wheel torque with friction, spur, ring pair, and the continuity sweeps
   (Σ 0→90°, β 0→45°, x, face through ε_β = 1).
7. Cost: evaluations per mesh, Python wall time, Rust estimate, hot-path verdict.
8. Rust migration plan for Stage 3.

## 4. Open issues already visible

- Tip-edge contact as a Hertz patch is the source of the worm's ≈ −1 point efficiency move
  (spike); it is not independently verified and the "tips relieved" option decides it.
- Face-edge contacts on shifted near-parallel pairs have an edge stress the Winkler slice omits.
