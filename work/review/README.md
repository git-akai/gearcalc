# notch-proto — the notch/edge prototype (P1, P2 of work/notch-research.md §6)

Pure Python 3 (no numpy). Every process caps its address space at 1.5 GB (`common.cap`), every loop is
bounded. Reads contact-proto/, contact-verify2/4/5 and the crate's `gear-cli` read-only; writes only here.
Results are written up in `/home/user/gearcalc/work/notch-research.md` §9 "Prototype results".

## Modules
- `common.py` — memory cap, Brent root, golden section, Gauss–Legendre, the inner-solution constants
  (sigma* = 0.83356, c = 0.58168).
- `across.py` — **the across contact in closed form for a rounded OR a sharp edge**: piecewise-quadratic
  gap, contact [a, b] from two elementary theta-integrals of g' (I0 = 0 for a smooth end, b = x_E for a
  sharp one), the edge intensity K = -E* sqrt(pi c) I0, the load, the pressure (logarithms), and the
  **approach** delta = (1/pi) int g dtheta + (2P/(pi E*)) ln(2/c) (exact for any single-interval 2-D
  contact). `edge_section` builds the tip-form section (flank, round r_e, land; or the truncation).
  Closed forms: inner law, inner extent d, r_lim = c^3 K^2 E*/S^3.
- `mesh2d.py` — whole-mesh 2-D spur model (17/43 m1 b10, both tips formed): the verifier's geometry and
  tooth compliance (v1_spur2d, v1_stiff), every contact through `across.py`, torque balance with each
  contact's arm, friction loss from the exact sliding speed. `model='weber'` = the verifier's pointwise
  compliance, for contrast.
- `melan.py` — first yield and Melan shakedown (moving load, residual sigma_x^r(z)) with traction mu p, for
  Hertz (check) and the rounded-edge inner distribution.
- `bem.py` — 2-D plane-strain direct BEM (Kelvin, quadratic elements, rigid-body diagonal, recursive
  near-singular subdivision, Somigliana interior stress, Gauss-point boundary stress + least-squares peak).
- `tooth.py` — the crate's external spur tooth, fillet, Lewis-parabola and 30-deg sections, DB K_f and ISO
  Y_S, re-implemented so rho_fP can vary; matches `gear-cli iso` to 2e-12.
- `fillet.py` — BEM models: 3 generated teeth on a 3m rim (inner arc fixed), and the rack tooth (z = inf)
  whose fillet can be made sharp.

## Runs (script -> output), in the order of the write-up
| script | output | what | wall time |
|---|---|---|---|
| `python3 t_across.py` | `t_across.txt` | P1.1: closed forms (truncated Hertz, tilt law, Hertz) to 4e-13; vs the verifier's BEM and contact2d; approach vs BEM; inner law at a fixed outer problem | 2 min |
| `python3 t_geom.py` | `t_geom.txt` | mesh2d's sections vs the exact involute/round geometry | 10 s |
| `python3 t_mesh2d.py 1000` | `t_mesh2d.txt`, `.json` | P1.2: integrated outputs vs r_e (0, 0.2 ... 0.001) at 20 and 60 N m, exact vs Weber compliance; peaks | 10 min on 9 cores |
| `python3 t_asym.py` | `t_asym.txt` | P1.3: p_max vs inner law and Hertz on the round at the phase of the max; slopes vs the verifier | 1 min |
| `python3 melan.py` | `melan.txt` | first-yield and shakedown coefficients (Hertz check: 4.00k / 3.57k) | 5 min |
| `python3 t_limits.py 0.5279 0.2797` | `t_limits.txt` | P1.4: limit radii, closed form vs root of the field, regime, continuity | 10 min on 10 cores |
| `python3 t_proto.py 48` | `t_proto.txt` | P1.2 on the prototype's own TField (spur, beta 0.5/2, ring) | ~20 min on 10 cores |
| `python3 t_bem.py` | `t_bem.txt` | P2.1 canaries: patch test, Kirsch (K_t 3), Somigliana interior, ellipses (K_t 7, 21) | 2 min |
| `python3 t_tooth.py` | `t_tooth.txt` | tooth.py vs `gear-cli iso` on 8 pairs | 10 s |
| `python3 t_fillet.py grid 10` | `t_fillet_grid.txt`, `.json` | P2.2: BEM peak vs DB and ISO over z x alpha x x x rho_fP (180 teeth) + point stress at L0/2 | ~40 min on 10 cores |
| `python3 t_fillet_summary.py` | `t_fillet_summary.txt` | DB / ISO vs BEM by regime; the limit-law slope at the peak's own radius | 1 min |
| `python3 t_sens.py` | `t_sens.txt` | the tooth model's sensitivity to rim depth, tooth count, load patch, far-field size | 2 min |
| `python3 t_rack.py` | `t_rack.txt` | P2.2: the notch limit law on a rack; sharp-corner divergence; point method finite | 10 min |
| `python3 t_support.py` | `t_support.txt` | P2.3: support C1 vs ISO / Peterson / FKM on the library's 4340s | 1 s |
| `python3 t_measure.py` | `t_measure.txt` | P2.4: eta, r_lim (tip radius needed), continuity, finiteness as rho_fP -> 0 | 1 min |

## Status (2026-09-29)
All runs above completed; outputs are in this directory. Defects found and fixed during the build (recorded in
the write-up): v1_spur2d's atan2 branch cut in the pinion-tip distance and its 0.05 mm pair cut-off (both
inherited, fixed in mesh2d.py only); the BEM load patch's mesh-dependent resultant (normalised) and the C0
derivative staircase (least-squares peak).  Not run: P1 step 5 (the wedge multiplier FE).  Four heavily
undercut 12-tooth 14.5 deg teeth do not solve in the BEM (singular system; not investigated).
