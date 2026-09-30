# Unified contact model: phase-exact prototype (the expensive mode)

Authors: contact-proto … contact-proto-5 (rounds one to five); contact-proto-6 (2026-09-29, round six, this
revision; item 5 by its fork). Read-only on the repo; pure Python, no numpy.
- **Scripts and outputs:** `~/.cache/gearcalc-work/contact-proto/` (README lists them; round six's runs are in `r6/`,
  earlier rounds' texts are `contact-model.round{2,3,4,5}.md`, round five's code `trace_round5.py`, `field_round5.py`,
  `coupled_round5.py`, `mw_tfield_round5.py`).
- **Differential oracle for the port:** `contact-proto/oracle/` (one script, `make_oracle.py`; JSON records, README).
- **Built on:** `work/plan.md` §2 and §5, `work/design-graph.md` §3, `review/contact-verify6.md`.

**Status.** By the owner's direction mid-round (Python run time is the bottleneck; the model moves to Rust and
development continues there), items 1, 2, 3 and 8 are finished; 4 and 5 are finished in the prototype and continue in
Rust; 6 moves to Rust, and 7 has its headline row (§2.7) with the rest in Rust (§5.4). Every figure below was run on the round-six code (`trace.py` md5 `095ff51d`, then
`field.py`/`coupled.py` with the tooth at unit module); every process capped at 1.8 GB.

**Verdict of this round.**
1. **No loaded line is cut** (§1.1, §2.1). The trace runs over the whole valley inside the field and two sections
   past it; round five's CAP (30 µm above the seed) is gone, and a loaded end where the valley is lost is a refusal.
   The involute worm wheel: 142 loaded `cap` ends → 0, and like for like the matched wheel reads −30.7 % (the
   verifier's −30.6 %). **The field is homogeneous**: every length ×10 and ×0.1 (torque
   ×k³) leaves every dimensionless output equal to ≤ 5.6e-9 on six meshes (round five: +6.6 % at ×10).
2. **New finding: round five's maximum at a phase was under-resolved** (§1.3, §2.2). The peak where a valley runs onto
   a round is narrower than a panel; 24 uniform panels over 1.5× the seed's span, read at the stations, gave
   1338 MPa against a converged 1990 (h20, 2 N·m, 0.1183 p: −33 %). Round six lays panels by the strip's own scale and
   reads each region's maximum along the line: N 24 is within 0.25 % of N 48 and N 96.
3. **Every constant is derived, shown immaterial, or a named input** (§2.3). CAP, EXT, LAM, HMIN, TURN are gone; no
   length in mm is left in the model.
4. **Maxima** (§2.4): region maxima searched piece by piece equal a dense reference to 5e-14 on 491 random strips
   (round five: one 2.35 % jump; now 6e-8). The offset-grid jump detector finds the stated spur reversal on three
   grids, including the one that lands on it, and nothing else; with creep there is none.
5. **The pitch-point reversal** (§2.5): Carter's creep is implemented (closed form per panel): a C¹ ramp over ±μa of
   roll, Coulomb outside it to the last digit. The exact curve for any piecewise-quadratic strip is derived in closed
   form; Carter is its Hertz case and errs up to 0.79 of μP on a round's strip inside the window.
6. **The foundation reads the gear's blank** (§2.6, the fork): monotone in rim, web and seat; −12 … −3 % against the
   whole-gear FE with the wind-up; round five's rim-as-rigid-support read webbed rims +23 … +62 % stiff. The tooth
   beside shares 15–82 % of the body's deformation, so the blank mostly sets the approach, not the load split: the
   port models tooth-to-tooth coupling through the body.
7. **§5 is the porting plan**: module order, the Python each module replaces, the oracle records it must reproduce
   with tolerances derived from the Python's own floors, and what continues in Rust.

## 1. What changed in the model (`trace.py`, `field.py`, `coupled.py`, `mw_tfield.py`)

### 1.1 The trace runs over the whole valley (`TField.trace`, `smin`)
- Each pair's valley is traced on the fixed sections X·d = k dz until it has been outside the field for **NBEYOND = 2**
  consecutive sections. Two, because the clip end's interval of the Hermite interpolant uses the nodes either side of it
  and, for central-chord tangents, their neighbours: with two outside, the interval holding the clip end does not depend
  on where the trace stopped. Round five also stopped at 30 µm of gap above the seed (CAP) and 0.2 mm beyond (EXT).
- The cap on sections is the number that fit in the field's bounding sphere (`field_radius()/dz` + 3); exceeding it
  is a refusal. The across search in `smin` starts at 1e-4 m_n and stops at the field's diameter (round five: 1e-4 mm
  to 26 mm); its fixed point stops at 1e-10 m_n.
- **A loaded end that is not a face, a root/form circle, a hob tip or the gap reaching D is refused** (`cut`), and so
  is a loaded pair at the enumeration bound |j| = J. Round five's `touch` guard saw only `gap` ends.

### 1.2 The span is the solution's own (`TField.solve`)
- The loaded set lies inside {g ≤ D} (D − g_i = c_t q_i + Σ K_ij q_j with every term ≥ 0), so the panels are laid over
  {g < Dmax} with Dmax = D: a fixed point, solved by secant on f(x) − x. Where the span covers the contact, f moves only
  through the panels, so 2–5 coupled solves reach |D − Dmax| ≤ 1e-10 (D − dlo), the coupled solve's own floor. Round
  five used Dmax = dlo + 1.5 (D0 − dlo), a factor on the seed.
- The coupled solve's first iterate is the seed's own slices (tooth in series with the 2-D line at the nominal load);
  round five's used c + 0.02 mm²/N. The seed's approach bracket runs from 1e-4 m_n to the whole tooth depth.

### 1.3 Panels by the strip's scale; maxima along the line (`panel_edges`, `line_max`)
- **Panels** equidistribute √kz·(Chebyshev weight): y = y_l + (y_r − y_l)(1 − cos θ)/2 resolves the ends, and the
  strip's half-width scales as kz^−½ while the lengthwise kernel decays over a few half-widths, so a panel on a round
  (kz ≈ 1/r_e) is √(kz_round/kz_flank) times shorter than on the flank. kz steps where the valley crosses a form's step
  line: those crossings are located (a fixed θ grid, then a bracketed root), √kz is Gauss-integrated piece by piece,
  and the cumulative weight is inverted piecewise linearly, so every edge moves continuously with every input.
- **The report** reads each region's maximum along the line, not at the stations: the stations' strips, then golden
  section between the neighbours of the two best stations (KREF 2), the line load linear between stations. The edge's
  load share is integrated over the panels whose edge fraction varies across them.

### 1.4 Region maxima piece by piece (`max_pieces`)
- A strip's pressure is analytic between the section's curvature steps (with log kinks at them). Each piece is sampled
  on its own cosine-graded nodes (NPIECE 12) and every sampled local maximum is refined by golden section. Round five
  sampled one 48-node grid over [0, π] and refined its two best maxima (contact-verify6 §2). `contact2d` uses the same.

### 1.5 Friction: Carter's creep as an option (`carter_slide`, `parms`)
- `friction='carter'`: Q/(μP) = 1 − (1 − ξ/ξ*)² below ξ* = μ a kz, Coulomb beyond, ξ the creepage (sliding over
  rolling speed). The panel mean over the chord is closed form piecewise (the window, where |v| < s* = v_r ξ*, takes
  v(2/s* − |v|/s*²); ∫v|v| and ∫|v|³ closed form); a panel the window misses is `panel_slide` exactly. It needs no new
  property: a is the strip's own half-width, updated with the load in the b fixed point.

### 1.6 Homogeneous by construction
- The generated tooth is built at unit module and read at depth/m_n (field.Member, coupled.derive_foundation). Built at
  m_n itself, the near-undercut z 17 moved its form radius by 1.6e-6 between m 1 and m 10 (an ill-conditioned
  trochoid–involute crossing in stiff.py) and the Chebyshev domain with it: D 3e-6, F_edge 2e-5 at ×10; now 1e-12.
- Every search step and tolerance in the field and the matched wheel is in m_n (§2.3).

## 2. Evidence

### 2.1 Items 1 and 7 (in part): the cut, and homogeneity (`t6_homog.py` → `r6/homog_*.txt`)
- **Worm, involute wheel** (ZI 1/40, r_e 0.1, worm 2 N·m) at 0.3183 / 0.7216 / 0.9299 p: p_max 5846.7 / 8586.9 /
  9106.2 MPa, L 4.75 / 5.46 / 5.50 mm, **no cut end** (round five: 6582 / 9076 / 9589, L 3.96–4.49, 142 loaded `cap`
  ends in 24 states; the verifier's untruncated CAP 0.3: 5817 / 8578 / 9093).
- **Homogeneity law** (every length ×k, T ×k³; p, D/k, L/k, F/k², F_edge/F, the output torque, n, the end tags):

  | Mesh (phases) | worst relative difference, k 10 / 0.1 |
  |---|---|
  | h20 2 N·m (0.1183, 0.5183); h20 20 N·m (0.3183, 0.62) | 5.6e-9 / 3.8e-9; 1.6e-9 / 3.6e-9 |
  | h20 20 N·m, tip relief 5 µm (0.3183) | 3.2e-9 / 3.2e-9 |
  | s0 spur 20 N·m (0.3183, 0.62) | 1.1e-12 / 6.8e-13 |
  | c10 crossed 10° (0.4183) | 4.5e-14 / 7.7e-14 |
  | worm, involute wheel (0.3183, 0.7216, 0.9299) | 2.7e-9 / 2.2e-9 |

  D, L and F agree to ≈ 1e-12; the residue is in the pressures read along the line (the golden search's floor; the
  same outputs move 5e-9 when only the seed's nominal load is ×2 or ×¼). Round five at ×10: h20 20 N·m +6.6 % (CAP),
  s0 2e-5 (the tooth's domain).

### 2.2 Convergence (`t6_conv*.py`, `t6_res.py` → `r6/conv*.txt`, `r6/res6_h20.txt`)

| Case, phase | N 24 | N 48 | N 96 | N 192 |
|---|---|---|---|---|
| h20 2 N·m, 0.1183 p — round five (uniform, span 1.5×, stations) | 1338.0 | 1817.2 | 1947.7 | – |
| — uniform, round-six span, stations | 1804.6 | 1994.0 | 1949.0 | – |
| — cosine-graded, stations | 1954.5 | 1993.9 | 1985.8 | 1993.0 |
| — **round six** (√kz-Chebyshev, line reading) | **1988.4** | **1993.0** | – | – |
| h20 2 N·m, 0.3183 p — round six | 1990.7 | 1989.3 | 1990.5 | – |
| h20 20 N·m, 0.1183 p — round six | 5939.7 | 5944.9 | – | – |
| c10, 0.4183 p (face end) — round six | 1348.9 | 1351.0 | 1351.8 | – |

- The peak (h20 2 N·m, 0.3183 p) sits on member 2's round where the valley runs onto it near a `gap` end: 0.33 mm of an 8.7 mm line, the load
  rising from 0 to 12 N/mm over it, p 1968 there against ≈ 700 on the flank beside it.
- Round five's maxima over phase were far less affected than its states (the phase search seeks the phase where a
  station sits on the peak). Two whole analyses rerun (`oracle/phase.json`): h20 2 N·m 1992.9 on member 2's round
  mid-face against round five's 2010.8 at the face end (−0.9 %, and the location moved); s0 20 N·m 5939.9 both.
- Other resolutions: NSEC 40 → 80 (dz = b/40 → b/80) moves p_max 7.7e-5, D 1.5e-6; KREF 2, 4, 24 give the same maxima;
  a dense scan along the lines (801 points) reads 1987.7 against the refined 1990.7; the tooth's Chebyshev n 20 errs
  4.4e-5 against direct evaluation (n 40: 3.7e-7); D, L, F converge at N 24 to ≤ 0.2 %.

### 2.3 Item 2: every constant (round five → round six)

| Constant | Round six | Basis |
|---|---|---|
| CAP 0.03 mm, EXT 0.2 mm | gone; NBEYOND = 2 sections | the Hermite stencil (§1.1) |
| HMIN, TURN | gone | unused since round four's fixed sections |
| KMAX 6000 | sections in the field's bounding sphere + 3 | geometry; exceeding it is a refusal |
| LAM 1.5 | gone: span {g < D}, SPAN_TOL 1e-10 | the loaded set lies in {g ≤ D}; the solve's floor |
| `smin` 1e-4 mm → 26 mm, stop 1e-10 mm | 1e-4 m_n → the field's diameter; 1e-10 m_n | the gear's scale; the field's size |
| seed bracket 1e-4 mm → 1.6 mm | 1e-4 m_n → the whole tooth depth | an approach beyond it is no gear |
| first iterate (0.5 Dmax − g)/(c + 0.02) | the seed's slices | the derived line compliance |
| guards q 1e-9 N/mm, b 1e-6 mm, qn 1e-3 | 1e-12 × nominal q, 1e-6 m_n, removed | never read into a loaded result |
| y0 golden 60 rounds to 1e-12 mm; anchor tolerances 1e-12, 1e-10 mm | 80 rounds to 1e-13 m_n; × m_n | the anchor is only a seed (a2 law) |
| tooth built at m_n | built at unit module | exact homogeneity (§1.6) |
| N 24 panels, NSEC 40 | kept | resolutions, converged (§2.2) |
| 48-node θ grid | NPIECE 12 per piece | = a dense reference to 5e-14 (§2.4) |
| KREF 2 (new) | – | 2, 4, 24 identical |
| Chebyshev n 20 (tooth) | kept | 4.4e-5 against direct evaluation |
| J = 4 pairs | kept, guarded | a loaded pair at the bound is refused |
| set_nominal at 0.37 p | kept | immaterial: nominal ×2, ×¼ moves every output ≤ 5.4e-9 |
| kernel TOL 1e-10, NMAX 64, G table (ln r −28 … 14, step 0.02) | kept | the width rule's own error; table = quadrature to 1e-11 |
| Gauss orders, golden rounds, bisection 1e-10 p | kept | resolutions, each converged |
| matched wheel: TOL 1e-12, NEAR 0.05, EXTU 1, FAR 1 (mm) | × m_n | convergence; a seed choice; the continuation past the hob tip; a count's class |
| matched wheel: across bracket 1.6 mm | the wheel's whole depth (the same 8 steps at m 1) | a section minimum lies inside the tooth |
| matched wheel: 25 × 25 seed table, caps 30 / 40 | kept | seeds and counted caps (0 stalled or failed, round five) |
| μ 0.06; E, ν; ρ_f 0.38, h_a 1, h_f 1.25, α_n 20 | named inputs | the mesh's; the material's; the cutter's |
| image ψ = 1 | the model (the mirror) | ψ = 1.561 is an instrument (Guilbault 2010) |
| rim default 1.2 h_t / 3.5 m_n | a named blank default (§2.6) | ISO 6336-3's Y_B thresholds are not a stiffness |
| C_M | named, on the mesh, default 1 | ISO 6336-1: 0.8, the measured correction |

### 2.4 Item 3: maxima and the jump detector (`t6_strip.py` → `r6/strip6.txt`; `t6_scan.py`, `jumps7.py` → `r6/jumps7_s0.txt`)
- **Strip law**: report/dense − 1 over 265 strips (the verifier's generator) and 226 with both members' rounds and tip
  relief inside: flank −4.3e-15, edge −4.9e-14, no over-read above 1e-9. The verifier's two-round strip across its
  8e-17 bisection: 3067.7754 / 3067.7756 (round five 2997.3 / 3067.8); a 200-step sweep of the round's position is
  smooth (largest neighbour change 5.8e-5).
- **Detector law** (the offset grid, the single-step test, the two-step test; every suspect zoomed by 12 halvings and
  called continuous only if the change shrinks), on s0 over 0.70–0.80 p at 1e-3 p:

  | Grid | Coulomb | Zoom | Carter |
  |---|---|---|---|
  | i/1000 (lands on 0.750) | single-step silent; two-step: D 4.13e-2, F 4.32e-2, p 2.23e-2 | stays at 3.2e-2 / 1.7e-2 to 5e-7 p: a jump | – |
  | (i + 0.3183)/1000 | single-step: D 4.09e-2, F 4.27e-2, p 2.19e-2 | stays to 2.4e-7 p | no suspect |
  | (i + 0.7071)/1000 | single-step: D 4.09e-2, F 4.28e-2, p 2.19e-2 | stays to 2.4e-7 p | – |

  No other suspect in the window, no error, no cut. The full 1000-phase scans on four bases are the port's gate (§5.3).

### 2.5 Item 4: the reversal and creep (`t6_carter.py` → `r6/{coulomb,carter}_s0.jsonl`; `t6_creep.py` → `r6/creep6.txt`)
- **Field** (s0, 20 N·m, 41 phases over 0.745–0.755 p): Coulomb F 2559.84 → 2450.42 N between 0.74975 and 0.75025
  (−4.27 %, p −2.16 %; the grid point on 0.75 reads 2534.8, a split); Carter a C¹ ramp over 0.74835–0.75165 p (±μa of
  roll, 3.3e-3 p), equal to Coulomb outside it to the digit. Maxima over phase are the Coulomb one-sided values.
- **The exact curve for any strip** (quasi-identical materials): with the stick zone [d, x_l] at the leading edge the
  corrective traction solves the normal problem's equation on [d, x_l], so ξ(d) = μ(1/π)∫₀^π h′(m′ + c′cos φ)dφ and
  Q = μP − μ(E*/2)c′∫h′ cos φ dφ: the across contact's own I₀ and I₁. ξ = 0 at full stick (its I₀ = 0), ξ* = μh′(x_l).
  - Hertz: Carter to 4.4e-16.
  - 400 strips straddling a round (both rolling directions): Carter with the field's ξ* errs up to 0.79 of μP (median
    0.02, 90 % 0.31); with the exact ξ*, up to 0.35. The exact curve is monotone in all 400.
- **Where Carter holds**: a Hertzian strip of like materials in steady rolling — the flank of steel spur and helical
  gears, where the reversal is. Elsewhere: a round's strip (above); unlike materials first order in Goodman's β
  (steel/bronze −0.073, steel/PA66 −0.16); crossed axes never reach the window (sliding ≫ μa·rolling). Oil-lubricated
  traction near pure rolling is the lubricant's and wider; Carter's is the narrowest grounded ramp, and it changes no
  rated maximum.
- **Recommendation**: `TractionLaw::Creep` as the default, evaluated by the exact curve (Carter where no step lies in
  the strip), Coulomb as the named alternative. It removes the model's one discontinuity with physics that is always
  present and needs no new input; "first order in μ" (T06.9) no longer holds inside the window, and nowhere else.

### 2.6 Item 5: the foundation reads the gear's blank (the fork: `r6/found6.py`, `fe6.py` → `t_rim6_{fe,diff,iso}.txt`)
- **Blank**: rim s_R at the full face b over a web b_w down to a rigid seat r_i (H_b = r_f − r_i). Flamant spreading in a
  layer of axial width t moves the load line by (2F/πE′t) ln(h₂/h₁), so **ln H_eff = ln s_R + (b/b_w) ln(H_b/s_R)**
  (solid: H_eff = H_b whatever s_R), and the wind-up, common to every pair (it shifts D only), is
  c_wind = r_b²/(4πG)[(b/b_w)(1/r_i² − 1/r_w²) + (1/r_w² − 1/r_f²)].
- **Against the whole-ring plane-strain FE** (17/43 m 1, eleven blanks: seats r_f/1.4 … r_f/4 and s_R 2.7–4.5 over
  b_w/b 0.5 and 0.25): model + wind-up −12.1 … −2.8 % of the FE's c′ (always softer); model alone vs FE local −9.2 …
  +11.8 %; round five's H = s_R on webbed rims +23 … +62 % (a thin rim on a web read stiff). Monotone in all three:
  seat deeper, rim thinner, web narrower each soften the FE whole, the FE local and the model.
- **Coupling through the body**: the tooth one base pitch away moves by 15–20 % of its own compliance on a thin rim,
  49–58 % on a deep solid body, 62–82 % under a web. The exact two-pair split barely depends on the blank (pair A
  0.427–0.434 over eleven); the field, reading each pair alone, overstates it by +2.3 … +5.3 % with H_eff. Self minus
  shared (what moves load) spreads 12–19 % over the blanks, while self spreads 5×.
- **Default**: a seat chosen to reproduce ISO 6336-1's c′_th ranges 0.39–3.30 m_n and follows the shift: not a
  default. The port models the pair-to-pair term from the same half-plane at the root spacing; then the blank sets only
  D, and an unset blank is a named option ("solid, seat unknown": D without wind-up, with a Note).

### 2.7 Items 6 and 7: to Rust
- Face-end flag: the Note and its gate are specified (§5.3, P9). Round five's face-end case (h20 2 N·m) now peaks
  mid-face (§2.2), so the bias is to be re-measured where a maximum does sit at a face end (c10 at 0.4183 p does, a
  crossed point contact), with the calibration widened (Guilbault's ν 0.15 point: ψ = 1.255; a quarter-space
  iteration as the instrument). This continues in Rust.
- Matched worm like for like (worm 2 N·m, r_e 0.1, whole analyses at N 24 on round six's field, `r6/mw6_*.txt`):
  matched 6310.5 MPa (η 67.02 %) against involute 9106.4 (η 67.87 %): **−30.7 %** (the verifier's −30.6 % with CAP
  0.3; round five's capped −34.1 %). The involute run had 85 states whose width fixed point stopped at its cap and 6
  whose span secant did (the worm's short, many lines): a port gate. The other three rows move to Rust (≈ 1 h each here).

## 3. Carried from earlier rounds
- The traced, seed-independent valley; the exact 2-D across contact (the verifier's BEM < 1e-5); images; the sharp
  edge refused (r_e → 0 diverges as r_e^−0.37); friction per panel continuous at the zero of sliding (round five,
  confirmed by contact-verify6 at its own planted phases); the ISO 6336-2 table of round five §2.6 (mid-line, unaffected
  by §1.3's peak); the matched wheel's robust projection.

## 4. Open issues (size and sign)
- **Tooth-to-tooth coupling through the body** is absent: pair shares overstated by +2 … +5 % (§2.6); the port adds it.
- **Round five's phase maxima** were read at N 24 uniform panels: −0.9 % and 0.0 % on the two rerun (states: to −33 %);
  the rest rerun in Rust.
- **The spur reversal** (Coulomb): −4.3 % of load at one phase; creep removes it (§2.5).
- **The free end is a smooth wall**: a face-end maximum high by about 1.5 %, one calibration point.
- **The derived tooth**: −12 … −3 % in c′ against the whole FE (§2.6); ISO's C_M 0.8 lies outside every model.
- **The sharp edge** stays refused (the singular-field track, `work/notch-research.md`).
- **The worm's fixed points**: 85 of 353 involute-wheel states stop the width iteration at its cap and 6 the span's
  secant (§2.7); the maxima agree with the verifier's to 0.14 %, but the port's gate (no non-converged state) fails there.

## 5. Stage 3: the Rust port (porting plan)

The field moves to Rust now and its development continues there. It lands as `crates/gear-core/src/field/`, additive:
the fast mode stays bit-identical under the identity harness (`gear-cli identity`). Each step ports named Python
functions, reproduces the oracle's records for them, and then carries the gates the prototype could not afford.

### 5.1 Where it sits, and its data (principle 10; `design-graph.md` §3)
- **After redesign G.** The field reads each gear's generated tooth — flank closed form (signed z, helix, base
  radius), form and root circles, tip, and the fillet the compliance integrates — from G's one generator (σ, κ), never
  from `BuiltMember`, which G deletes. Until G lands, `FieldGear::from_spec` builds from the oracle's plain member
  spec (z, β, x, b, m_n, α_n, h_a, h_f, ρ_f, E, ν, form, blank); a law then holds G's output equal to it on every
  oracle member.
- **On the gear** (read by that gear alone; a `FieldGear` is built from `&Gear` and its generated tooth, type-enforced):
  `form: ToothForm { tip_relief: Option<Relief { depth, length }>, edge_radius: Option<f64>, end_relief:
  Option<Relief> }` and `blank: GearBlank { rim_thickness, web_width, seat_radius }`, all optional. Today's
  `MemberGear::rim_thickness` moves into the blank, and its doc changes: it now reaches contact.
- **On the mesh**: the friction μ it has, `traction: TractionLaw { Creep, Coulomb }` (default Creep, §2.5) and
  `stiffness_factor` C_M (default 1, theory; ISO 6336-1's 0.8 the stated alternative; a relieved design is reported at
  both). **On the centre**: distance and Σ, as today.
- **Names** not to reuse (they exist): `Rating` (train/mod.rs), `Piece` (train/edits.rs), `Section` (tooth.rs), `Body`
  (kinematics.rs), `Span` (metrology.rs), `Contact` (contact.rs), `Mesh`/`Gear`/`Centre`, and `solve.rs` (the root
  finders). The field's modules and types are named below.

### 5.2 Two modes, the rated figure, and the final optimisation
- **Fast mode** (default): today's closed forms, every solve and the search. **Field mode**: `Analysis::Field` on a
  case, once per mesh per load case, on the loads the fast mode rated (`CaseLoad`). Reports `MeshReport.field:
  Option<FieldReport>`: p_max with where and region, p_flank, p_edge, the edge share, the load-weighted length, the
  efficiency, the ISO-point readings, the forms and blanks used. Refuses an unset edge radius.
- **The figure it rates**: σ_H,field, the field's pressure at ISO's points B, C, D (phase roots, both one-sided limits,
  the largest), because σ_H,lim is calibrated against that reading; the field's load share replaces Z_ε and K_Hβ's
  approximations. The field maximum (2.7–4× σ_H with r_e 0.1 m_n and no relief) is reported, not rated: rating it is a
  named option, off, until the notch-research track gives an allowable for a local peak from common properties. This
  keeps the 2026-09-29 ruling (ISO's points rated, the field maximum beside them).
- **The final optimisation** (2026-09-30: "a final optimisation run once everything is constrained"): the fast search
  finds the design and never calls the field; `refine`, on demand, re-ranks the fast search's K best by σ_H,field, then
  searches locally (bounded, derivative-free) over the inputs only the field reads — tip relief depth and length, end
  relief, the edge radius within its admissible range — minimising the field maximum (the edge contact the fast mode
  cannot see) while the rated σ_H,field keeps the required S_H, every fast-mode input held. Laws binding the two:
  refine never leaves the fast mode's admissible set; never ends worse than its start in either figure; where both
  modes apply (flank, mid-face, no edge contact) they agree within round five's ISO table; the fast winner is reported
  beside the refined one. The field's continuity in every input (P6, P8) is what makes the local search well posed.

### 5.3 Order, the Python each step replaces, the oracle it reproduces, and its gates
The oracle (`contact-proto/oracle/`, from `make_oracle.py`; README) holds each record's inputs as plain numbers and the
Python's outputs. **Tolerances follow one rule**: a closed form matches to floating point (1e-13); a root or a fixed
point to ten times its own stopping tolerance; a field state to ten times the Python solve's floor, measured as the
largest change under the model's exact symmetries (every length ×10 and ×0.1; the seed's nominal load ×2 and ×¼):
D 3e-9, p_max/p_flank/p_edge/F_edge 5e-8, L 5e-9, F and the torques 1e-10, station q 1e-6 of the line's maximum; the
count of lines and every end tag exact; where a maximum sits 2e-3 of the line (a flat maximum's location is the square
root of its value's floor).

| Step | Module (types) | Replaces (Python) | Oracle (records) | Gates beyond the oracle |
|---|---|---|---|---|
| P1 | `field/form.rs` (`ToothForm`, `Relief`) | `form.Form` (value, slope, curvature across the edge; `on_edge`; the corner angle) | `form.json` (20: five gears × four forms × 25 σ) | C¹ at every joint; curvature 1/r_e on the round; a C⁰ form refused |
| P2 | `field/kernel.rs` (`Kernel`, `Panel`) | `kernel.G`/`dG` (R_D from `elliptic.rs`), `panel`, `depth_panel`, `n_width`, `line_limit`, `shape_C`; `carlson.aspect` | `kernel.json` (5: G 1e-10; panel 1e-9; the rest closed) | the line limit; the Hertz ellipse at O(N⁻²); the width rule to 1e-10 |
| P3 | `field/across.rs` (`Strip`, `StripReport`) | `trace._pieces_of`, `_segments`, `_I`, `_conj`, `_strip`, `contact2d`, `strip_report`, `max_pieces`, `panel_slide`, `carter_slide`; the exact creep curve (`t6_creep.py`) | `across.json` (43 strips, the verifier's two-round one included: c 1e-12, p 1e-10, F_edge 1e-10; 20 friction panels 1e-12) | region maxima = a dense reference to 1e-12 on 491 random strips; Hertz off a step to 1e-12; the BEM (`bem2d.py`) to 1e-5; the creep curve = Carter on Hertz to 1e-15 and monotone |
| P4 | `field/compliance.rs` (`ToothCompliance` in module units, `GearBlank`) | `field.Member`'s tooth (stiff.py's potential energy over the generated profile, at unit module), `foundation.coefficients`, `coupled.derive_foundation`/`rim_of` → the blank's H_eff and wind-up (`r6/found6.py`) | `compliance.json` (7: six gears × nine radii at 5e-5, the interpolant's error; foundation 1e-9) | the FE table of §2.6 (`r6/fe6_*.json`: whole −12 … −3 %, local −9 … +12 %); monotone in rim, web, seat; exact homogeneity; the ring from its own generated profile (stiff.py used the rack tooth) |
| P5 | `field/gap.rs` (`FieldGear`, `Gap`, `Valley`) | `field.Field` (frames, `d1`, `d2`, `grad`, `anchor` as a seed), `valley.VField.part`/`Kform`, `trace.TField.trace`, `smin`, `interp`, `valley`, `loaded`, `pair`, `face_crossings`, `section`, `flank_curv`, `fmargin`, `field_radius` | `gap.json` (5 meshes: h20, s0, c10, ring, worm; per pair the anchor, clip ends and tags, y0, trace nodes, seven valley points with d1, d2, σ, curvature and steps; closed 1e-13, valley 1e-9 m_n) | `tools/contact_oracle.py` (ISO 21771 play, a₀) to 1e-12 mm; \|∇d\| = 1; seed independence 1e-8; **no loaded line cut**: a loaded end at a lost valley is a refusal, over the preset grid and the worm; the J bound refused |
| P6 | `field/coupled.rs` (`Station`, `ContactLine`, `FieldState`) | `trace.TField.solve` (seed, the span's fixed point), `stations` and `panel_edges` (√kz-Chebyshev panels), `kmatrix` (images), `_coupled` (active set, D in closed form per iterate, the width fixed point, creep's update), `parms` | `state.json` (22: h20 at 2 and 20 N·m, relief, N 48, ×10 and ×0.1, s0 and its one-sided pitch-point states with Coulomb and creep, unequal faces, c1, c10, the ring, the worm both ways) | equilibrium 1e-12; complementarity; **homogeneity** ×0.1/×10 ≤ 1e-8 on every preset; **continuity**: every input ±1e-9 proportional at planted phases (the zero of sliding on a panel edge and on a station, a pair entering, a tag change); **convergence**: p_max N 24 vs 48 ≤ 0.3 %, NSEC 40 vs 80, KREF 2 vs 24; the span in ≤ 6 solves; no width or span fixed point stopped at its cap on the preset grid and the worm (the prototype's worm fails this, §4) |
| P7 | `field/report.rs` (`FieldReport`) | `TField.state`, `line_max`, `strip_at`, `qline`, `edge_on_panel` | `state.json` (p_max, p_flank, p_edge, where, F_edge) | the line reading ≥ a dense scan along the line (801 points) and equal to it after refinement |
| P8 | `field/phase.rs` (`PhaseInterval`) | `phase.breakpoints`, `phase4.analyse4` | `phase.json` (s0, h20: breakpoints 2e-10 p; means and maxima 1e-7) | **the detector law**: 1000 phases on the grids i/1000, (i + 0.3183)/1000 and (i + 0.7071)/1000 on h20, s0, c1, c10; single- and two-step tests; every suspect zoomed; with Coulomb the s0 reversal is found on all three grids at its closed-form size (F −4.27 %, p −2.16 %), with creep there is none; the maximum over phase ≥ the dense grid's |
| P9 | wiring (`Analysis`, `MeshReport.field`, strings ×5, `field_rate`, `gear-cli field`) | – | – | the fast mode bit-identical; the ISO-point reading within round five's table; notes fire both ways: `field.edge_radius_unset` (refusal), `field.max_at_face_end` (the maximum on a panel that touches a face clip: overstated by ≤ ~1.5 %), `field.blank_default` (a blank input defaulted), `field.stiffness_factor` (a relieved design at C_M 1 and 0.8); `check_wasm.sh` probes `field_rate` |
| P10 | `field/gap.rs` second flank (`Flank::{Involute, Matched}`), not exposed | `mw_tfield.MWTField` (`foot5`, `S_ext`, `hobtip`, `smin`, `flank_curv`, `seeds5`), `mw_surface`, `mw_field` | `matched.json` (one state; projection 1e-12 m_n) | play j = s_h/(cos α_n cos γ) exactly; the conjugate gap ≤ 1e-12 mm; the projection never stalls or fails |

- **Constants**: a check in the style of `check_units.py` lists every float literal in `field/` with its row of §2.3's
  table (derived, a resolution with its convergence gate, or a named input); one without a row fails.
- Cost: the prototype runs a state in 0.7–10 s of Python (h20 ≈ 3 s at N 24); the port's target is round four's
  5–24 s per whole analysis. Nothing in P1–P8 enumerates or branches on the kind of mesh: spur, helical, crossed,
  worm and ring are parameters (signed z, Σ).

### 5.4 What continues in Rust
- **Creep (item 4)**: `TractionLaw::Creep` evaluated by the exact curve per strip (§2.5), the default; P3's gate.
- **The blank (item 5)**: `GearBlank`, H_eff and the wind-up (P4), then the pair-to-pair term through the body from the
  same half-plane at the root spacing (the missing coupling, §2.6), gated against the FE's two-pair splits
  (`r6/t_rim6_fe.txt` §3: pair A's share 0.4303 ± 0.0041 over eleven blanks) and its shared displacements
  (`fe6_17_solid4.json`: 0.02334 of a self 0.03996 µm/(N/mm)).
- **The face-end Note (item 6)**: P9; widen the mirror's calibration (ν 0.15, and a quarter-space iteration).
- **The matched wheel like for like (item 7)**: the three remaining rows of round five §2.5 on the ported field, and
  the involute worm's non-converged fixed points (§4).
- **Round five's phase tables** rerun on the port (ISO examples, face-end bracket, C_M, the relieved design at the
  blank's default), since their states were read at N 24 uniform panels (§2.2).

### 5.5 What it subsumes from the audit
- **As the reference its proofs need** (the task stays in the fast mode; its law reads the field): T06.5 (load share
  conserves load), T06.8 (efficiency from the per-panel loss), T06.9 (first order in μ: exact outside creep's window),
  T06.10 (the load-weighted length), T08.3's σ_H comparison, T21.9 (specific sliding). T08.11's finite-line contact is
  P2's job; its independent DC-FFT instrument stays the validation, since it truncates the face where the field mirrors.
- **Replaced in the field mode**: T07.19, T08.12 and T07.2/4/5/15's crossed face (redesign X's interval model is the
  fast mode's), strength#6 (Z_ε at B), T21.13 (compliance load sharing: P4 + P6), T21.11 (lead crowning: `end_relief`).
- **Spike S-C** (plan §3) closes with this plan: the unified model is the field; `Path`'s two implementations stay as
  the fast mode.
