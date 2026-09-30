# The field's differential oracle, version 2 (contact-proto-6; corrected by oracle-fix, 2026-09-30)

What the Rust port of the unified contact field (`work/contact-model.md`, "Stage 3: the Rust port") must reproduce. Every file here is written
by one script (version 1, which `review/contact-verify7.md` checked, is kept in `../oracle_v1/`; `index.json` carries
`version` and the list of `changes`):

    python3 oracle/make_oracle.py [module ...] [--pool N]     # modules: form kernel across compliance gap state phase matched

The unit-level modules take seconds; `gap`, `state` and `matched` a few minutes; `phase` (two whole analyses) about
half an hour. Every worker caps its address space at 1.8 GB. `index.json` records when it ran, the md5 of every
Python source it read (the port matches THOSE sources), the record ids per module, and any record that failed.

## Files and records

Each `<module>.json` is `{module, records: [{id, fn, inputs, outputs, tol}]}`:
- `fn` names the Python functions the record exercises (the port's step in the porting plan's table of steps replaces exactly those);
- `inputs` are plain numbers, enough to build the case without Python (mesh specs below);
- `outputs` are the Python's values; `tol` gives each output's tolerance and its basis.

| File | Step | Records |
|---|---|---|
| `form.json` | P1 | 5 gears (17 β20, 43 β−20, 17 spur, ring −43 β15 m2, the 1-start worm) × 4 forms (r_e 0.1 m; relief 5 µm·m over 0.4 m with r_e 0.1 m; the same with a sharp edge; r_e 0.45 m): the corner angle (sth, cth), the round's tangency and end, and (F, F′, κ) and `on_edge` at 25 σ |
| `kernel.json` | P2 | G(r) (table and quadrature) and G′(r) at r = 10^(k/2), k −12 … 12; `panel` on 34 cases (one with h = ∞; v2 adds separated panels with surface/total to −7e3, h < b, both sides of RSEP, a panel ending at the point), each with its route (`sep`: `panel_sep`; `G`) and the `scale` of its two terms; `line_limit`; `shape_C` and `aspect`; Carlson R_F, R_D |
| `across.json` | P3 | 43 strips (Hertz, 40 random sections with one or two rounds and tip relief, the verifier's two-round strip): `contact2d` (c, m, p, where) and `strip_report` (p_max, where, p_flank, p_edge, the edge share); one record of 30 panels for `panel_slide` (Coulomb) and `carter_slide` (creep), ten of them a spur line's (\|B\| 1e-16 … 1e-12 \|A\|) |
| `compliance.json` | P4 | 6 gears × 12 radii down to the form circle: the form radius, ct and h_d (the derived foundation at the rim default), ct with Sainsot's foundation, the interpolant's order and measured error; 5 cases of the half-plane foundation's L*, M*, P*, Q*. **The ring record is the rack tooth** (stiff.py's z 4000, ISO 6336-1's z_n2 = ∞), not the ring's shaper-cut tooth |
| `gap.json` | P5 | 5 meshes (h20, s0, c10, ring, worm) at one phase: each member's circles and form constants, the section direction and spacing, and per pair the anchor, clip ends with tags, y0, the trace nodes (sections and gaps) and 7 valley points (point, gap, across direction, d1, d2, σ, form-foot radius, k0, kz, kl, C, the curvature steps) |
| `state.json` | P6, P7 | 22 states: the whole report (D, n, L, Lp, torques, loss, F, F_edge, p_max, p_flank, p_edge, where, the end tags) and per line the end weights (lam), the panel edges and stations (ye, ys, lp), the loads q, gaps g, tooth compliances; span iterations; counters |
| `phase.json` | P8 | s0 and h20 whole analyses: breakpoints, means, maxima and where |
| `matched.json` | P10 | the matched ZI wheel's state at one phase |

## Mesh specs (inputs of `gap`, `state`, `phase`, `matched`)

`mesh = {members: [m1, m2], Sigma_deg, a, mu}`, each member `{z (signed: < 0 a ring), beta_deg (this member's own
helix), x, b, mn, an_deg, E, nu, ha, hf, rho_f, form: {Ca, La, re}, rim, end}`; lengths in mm, loads in N and N·mm,
pressures in MPa. **In `state`, `phase` and `matched` each member also carries `tooth`**, the tooth compliance the
record was computed with (`field.ToothCompliance.record()` plus `found_H`): `rb`, `top` (the unit-module virtual spur's
base and tip radii), `H` (the depth domain, in m_n), `form_depth` (the form circle's depth below the tip, in m_n: the
member's flank starts at r_a − m_n·form_depth), `t` (Chebyshev–Lobatto nodes in the roll parameter
t = sqrt(r² − r_b²)/r_b), `ct` (mm/(N/mm) per unit face width) and `hd` (the depth reference, in m_n) at the nodes, the
order `n` and its measured error `err`. c_t at a load point of depth u (mm) below the tip is the barycentric interpolant
(weights (−1)^k, halved at both ends) at t(top − u/m_n), clamped to the nodes' range; h_d is m_n times the same
interpolant of `hd`. A port reads it (`ToothCompliance::from_nodes`) to be held to the state; its own compliance is
held to `compliance.json`. The state inputs add `T` (N·mm) on member `on` (1 or 2), `phase` (fraction of member 1's pitch
2π/|z1|) and `t` (radians), `friction` ('panel' Coulomb, 'carter' creep), `N` panels, `NSEC` sections per face width,
`grade` 'kz', `nominal` (the seed's pair force and line load from `phase.set_nominal`; immaterial to the result:
×2 and ×¼ move every output ≤ 5.4e-9), `J`. Member 2's helix is Σ − β for an external pair and β for a ring.

| Mesh | What |
|---|---|
| h20 | 17/43, β 20°, b 10, m 1, x 0, a_par, r_e 0.1 both, μ 0.06 |
| h20r | h20 with tip relief C_a 5 µm over L_a 0.4 mm |
| s0 | 17/43 spur, b 10, r_e 0.1, μ 0.06 |
| s0bb | s0 with b₂ 10.5 |
| c1, c10 | h20 at Σ 1° and 10° (member 2's helix Σ − β) |
| ring | ISO 6336-2's ring helical 17/−43, β 15°, m 2, x 0.3/−0.3, b 20, μ 0, r_e 0.2 |
| worm | the shipped 1/40 ZI worm (d₁ 7, m 1, steel 190 GPa/0.29) and its involute wheel (C360 96.5 GPa/0.32), Σ 90°, r_e 0.1, μ 0.06 |

## Tolerances (one rule)

- A closed form: 1e-13 relative (1e-13 absolute where a value passes zero).
- A root or a fixed point: ten times its own stopping tolerance (each record's `tol.basis` names it).
- A field state: ten times the Python solve's floor, measured as the largest change under the model's exact
  symmetries — every length ×10 and ×0.1 (`r6/homog_*.txt`), and the seed's nominal load ×2 and ×¼. Floors: p 5.4e-9,
  F_edge 3.5e-9, D 3.0e-10, L 4.3e-10, F 3.2e-12, torques 6e-13. So D 3e-9, p 5e-8, F_edge 5e-8, L 5e-9, F and the
  torques 1e-10; station loads 1e-6 of the line's largest; the number of lines and every end tag exact; where a
  maximum sits 2e-3 of the line's length (a flat maximum's location is the square root of its value's floor).
- The tooth compliance (`compliance.json`): rel 1e-11. It is a converged quantity: stiff.py's quadrature to 1e-15
  (Gauss–Legendre 48 against 96), the interpolant to ≤ 1e-12 by construction (its order is doubled until the
  deviation from direct evaluation at the next grid's new nodes is ≤ 1e-12, `ToothCompliance.TOL`). Version 1 said
  5e-5 for an order-20 interpolant in depth that erred 3.2e-3 on the z 17 spur.
- A panel (`kernel/panel`): 1e-12 × `scale`, the larger of the panel's two terms (each known to ~1e-15 of itself):
  relative 3e-12 far out, absolute near the kernel's zero at r ~ h. The records equal a 50-digit quadrature of the
  panel's definition to ≤ 3.3e-14 × scale (`oracle-fix/check_panel_dec.py`). A port computing a separated panel as
  surface minus depth through G fails them: it needs a cancellation-free form (`kernel.panel_sep`).
- A strip's half-width c: 1e-13 mm absolute (contact2d's Brent stops at 1e-14 (1 + |a| + |b|)).

A port that reproduces a record beyond its tolerance has found either its own defect or a Python one; both are
answered by the record's `fn` and the gates in the porting plan's table of steps, never by widening the tolerance.

## What the port found in the prototype

The records keep the prototype's values; the port is held to the physics and to methods that share nothing with
the prototype, never widened to a flaw. What each step found:

- **P2 (`kernel.py`).**
  - `G` below its table (`r < e^−28`) is linear in `r`, `G(r₀) r/r₀`, where `G` is `(4/π) r ln(4/r)`: 38 % low at
    `r = 1e-20`. Under `1e-12` absolute, so no record reaches it; the port takes the small-`r` form, and the
    law that holds `G` to its definition plants the linear tail.
  - The line-limit check (`t_kernel.py`; `kernel.py`'s docstring says "checked to 1e-9") summed panels over
    `±10⁵ h` and read the `3.1e-12` left as agreement; that residual is the omitted tail beyond `±L`,
    `(1 + ν) ν h²/(2π E L²)`, exactly. The port's law adds the tail in closed form and holds the rest to
    rounding.
  - `shape_C` clamps a curvature ratio outside `[0, 1]` into it; the port refuses one.
  - Absence carried as a number: `dG(0) = inf`, the bare half-space as `h = inf`, and `n_width` returning its
    cap for `h/b` not `> 0`. The port types each (`None`; a panel with no depth; a panel refused).
  - `G_direct` is exact only in absolute terms at small `r` (a few `1e-16`, where `G` is `~ r ln(1/r)`): it
    subtracts the `ln sin θ` singularity over the whole range. The port keeps the method and states the limit:
    a panel `2e-3 b` long on the point still holds `1e-13` of itself (the panel law), the relative error grows
    as one over the length below that, and no panel of a solve is near that short.
  - This README's "34 cases" for `kernel/panel`: the file holds 35.

## Provenance of this copy

Copied from `~/.cache/gearcalc-work/contact-proto/oracle/` (the prototype's tree, outside the repository): **version 2**,
generated 2026-09-30 01:59 as `index.json` records (`version` 2, its `changes` list), with no failed record
(`errors: []`); the logs are not copied, and neither is version 1 (`../oracle_v1/` above is the prototype's, not a
path here). `make_oracle.py` is the script that wrote them, kept here as their provenance: it reads the prototype's
Python, whose md5 sums `index.json` lists, and does not run from this directory. The JSON files and the script are
byte for byte the prototype's; this README is the one file changed: its three section-number references name their
sections instead, since the repository's pointer check refuses a bare one, and this section is added.
The oracle's own test (`the_copy_is_version_two_and_whole`, in `crates/gear-core/src/field/oracle.rs`) holds every
file here but this README to the table below by its md5, and refuses a file in this directory the table does not list.

| File | md5 |
|---|---|
| `across.json` | `601c864dab33f0199582a0e2384936cd` |
| `compliance.json` | `b586a8635796669801f539a55f64f957` |
| `form.json` | `1038c07ebb310ffa00edb1433ab30185` |
| `gap.json` | `f4821ac3273367fe5d4e4feabbd251c3` |
| `index.json` | `096f1df9fbdffd3bae6fe3bef281f7a6` |
| `kernel.json` | `35b0e86116762ee4ae94c9bbf65f8ccc` |
| `matched.json` | `767a343c73704b47a504b0d430bbf032` |
| `phase.json` | `d82dc4a98aa436e0019b3d876c2d35c4` |
| `state.json` | `e18caecebac3acb775b3889832d92bac` |
| `make_oracle.py` | `40318c9e471ec2c44bbe520285715f33` |

Each step of the port reads its records from here (`crates/gear-core/src/field/`, one oracle test per module).
