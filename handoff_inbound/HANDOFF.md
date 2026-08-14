# Involute gear generator — handoff

Audience: an agent picking this up cold. Covers what the code is for, what it
actually does, what was tried and abandoned, where the traps are, and what to do
next. Read `ARCHITECTURE.md` first if you want the short human-oriented tour.

---

## 1. Intent

### 1.1 Original intent (now retired)

The task began as: produce **two closed-form continuous functions `x(T)`, `y(T)`**
describing an entire involute gear cross-section — involute flank, trochoid root
fillet, major (tip) arc and minor (root) arc — for a host evaluator that accepted
only a single parametric expression pair, a fixed operator/function set
(`+ - * / ^`, trig and inverse trig, `abs exp log sqr sgn`, `pi`), named constant
"input variables", and no branching, loops or piecewise definitions.

That was achieved and validated. The techniques are recorded in §6 because they
are reusable, but **none of that code is in the current deliverable**.

### 1.2 Why it was abandoned

The host evaluator turned out to have hard limits far below what the geometry
needs:

- it could not handle 30 constants (the compressed solution's minimum), nor
  anything near the 71 of the readable version;
- it had an effective **1000-character limit per formula**, against ~2500
  characters for the compressed form and ~2700 fully numeric.

The user elected to pivot to a different evaluator/tool rather than keep
compressing. That released every artificial constraint: branching, loops,
iteration, piecewise definitions and unlimited intermediates are all now fine.

### 1.3 Current intent

A **clean, accurate, extensible gear geometry generator plus a test suite that
can actually prove it right.** Priorities, in the user's words: accuracy and
extensibility for the generator, and likewise for the checks. Inputs stay in
degrees; internals are radians because that is what the mathematics wants.

Deliverables the user cares about now: correct geometry, graphical output, and a
passing test suite. Formula/constant emission is explicitly **not** wanted at
this time.

---

## 2. Current state — what works

`gear.py` (generator) and `gear_tests.py` (suite) are the live code.
`sweep.py` / `tier3.py` drive parameter sweeps; `render.py` makes the figures.

### 2.1 Verified results

| Check | Scope | Result |
|---|---|---|
| Tier 1 (all cheap checks) | 6480 cases | **0 failures**, 2422 undercut |
| Tier 2 (+ fillet envelope) | 159 cases | **0 failures**, worst 5.5e-8 |
| Tier 3 (+ full rack simulation) | 44 cases | **0 failures** |
| Cutter penetration | tier 3 | **exactly 0.00** every case |
| Deviation from rack | tier 3 | worst 5.0e-4 mm |
| Fillet is tip-round envelope | tier 3 | worst 4.6e-8 mm |
| Analytic vs polyline distance | 114 runs, 3 seeds | worst 1.2e-7 (= chord floor) |
| Involute thickness law | per case | ~1e-16 rad |
| Flank/fillet junction gap | per case | ~1e-16 mm |
| Sampling evenness (max/mean step) | 6480 cases | median 1.73, worst 3.27 |

Module 1 throughout, so millimetres are also fractions of module.

### 2.2 The headline fix

Undercut gears used to show a **visible step** between the involute flank and
the trochoid fillet. Cause: the flank's lower limit was clamped to the base
circle (`u_j = max(L,0)/rb`) and a straight "bridge" segment spanned the
leftover gap. The flank must instead continue **below** the base circle until it
genuinely intersects the fillet.

| junction gap | before | after |
|---|---|---|
| z=8, x=0 | 0.320 mm | 6.3e-16 mm |
| z=3, x=0.5 | 0.315 mm | 3.3e-16 mm |

The bridge section is gone; a half-tooth is now four sections, not five.
`Gear(params, legacy_clamp=True)` reproduces the old broken behaviour and is
retained **only** so the suite can demonstrate it detects the fault.

---

## 3. The mathematics as implemented

All in `gear.py`. Angles internal = radians. `theta` always means the magnitude
of the angle from the **tooth centreline**: 0 at the tooth centre, `pi/z` at mid
tooth-space.

### 3.1 Normal → transverse

Inputs are a normal-module system, the cross-section is transverse:

```
mt      = m / cos(beta)                     transverse module
alpha_t = atan(tan(alpha_n) / cos(beta))    transverse pressure angle
R       = mt * z / 2                        pitch radius
rb      = R * cos(alpha_t)                  base radius
```

Radial dimensions use the **normal** module (correct): `bd = m*(hf - x)`,
`rf = R - bd`, `ra = R + m*(ha + x)`. Transverse thickness at the pitch circle is
`st = m*(pi/2 + 2*x*tan(alpha_n)) / cos(beta)`, giving `psi_p = st/(2R)` and
`psi_b = psi_p + inv(alpha_t)`.

### 3.2 Involute flank

Parametrised by roll parameter `u = tan(alpha_r)`:

```
r(u)     = rb * sqrt(1 + u^2)
theta(u) = psi_b - (u - atan(u))
u_tip    = sqrt((ra/rb)^2 - 1)
```

### 3.3 Trochoid fillet — the rack-generated envelope

The fillet is swept by the **rounded tip corner** of the generating rack. Let
`rho` be the cutter tip radius, the round's centre sit `bc = bd - rho` below the
rolling line and `ac = st/2 + bc*tan(alpha_t) + rho/cos(alpha_t)` to the side.
With `s` the rack travel parameter (`s = 0` puts the corner at the root):

```
D = hypot(s, bc)
k = 1 + rho/D
fixed-frame point = (k*s,  R - k*bc)
r(s)     = hypot(k*s, R - k*bc)
theta(s) = atan2(k*s, R - k*bc) - (s - ac)/R
```

Derived from the envelope condition (the contact normal passes through the pitch
point). Two consequences worth knowing:

- `r` depends on `D` **alone**, and `dr²/dD > 0`, so radius is strictly monotone
  in `|s|`. This is what makes the profile a well-behaved graph over radius.
- `r² = (D+rho)² + R² - 2R·bc - 2R·bc·rho/D`, which is a cubic in `D` — used by
  the old closed-form work, kept here as documentation.

### 3.4 The flank/fillet junction — the important part

`L = R*sin(alpha_t) - bc/sin(alpha_t) - rho` is the signed distance from the base
tangent point to where the rack's straight flank runs out. **`L < 0` is exactly
the undercut condition.**

- **Not undercut (`L >= 0`):** the straight flank ends precisely where the tip
  round begins, so flank and fillet meet **tangentially** at a point available in
  closed form: `u_j = L/rb`, `s_j = -bc/tan(alpha_t)`. Exact, no iteration.
- **Undercut (`L < 0`):** the round has eaten past the flank's limit and the two
  curves **cross**. Solved, not assumed: bracket `s` between where the fillet
  crosses the base circle and where it has clearly passed outside the flank, then
  `brentq` on the angular difference. `_solve_junction()` in `gear.py`.

Because they cross rather than touch, the undercut profile has a genuine corner
there — the undercut notch. That is correct geometry.

### 3.5 Tip radius cap (pointed teeth)

`ra` is capped at the pointed-tooth radius, found exactly by solving
`psi_b = u - atan(u)` with `brentq`. (The retired closed-form version used the
approximation `alpha ≈ (3v)^(1/3) - 0.4v`, accurate to ~6e-5 rad — fine there,
unnecessary here.)

### 3.6 Fillet size cap

The cutter's tip rounds must fit the tooth space. Each round consumes
`rho*(1 - sin alpha_t)/cos alpha_t` of tip width, so with rack tooth width at the
tip line `w_tip`:

```
rho_max = w_tip * cos(alpha_t) / (2*(1 - sin alpha_t))
```

**This was previously wrong** (`w_tip/(2 cos alpha_t)`), which silently shrank the
requested fillet on every profile-shifted gear — e.g. a requested 0.38 became
0.334. A useful side effect of the correct form: it is algebraically equivalent
to `ac/R <= pi/z`, so satisfying it **guarantees a non-negative root arc**. The
suite asserts that.

### 3.7 Severed teeth

At very low tooth counts with negative profile shift, the fillet reaches the
tooth centreline: the two fillets bounding one tooth overlap and the cutter
removes the **entire tooth**; material beyond is detached. Verified physically
real for z=3, x=-0.5 (fillet crosses `theta=0` over radii 0.279–0.953).

`_check_severed()` detects it, truncates the profile at the centreline so the
output stays a simple closed curve, sets `severed=True`, and records a clamp
note. Severed gears have `u_j`/`u_tip` = NaN and only two sections
(fillet + root arc) — **any new code touching the flank must check
`g.severed` first.**

### 3.8 Clamps

All degenerate inputs are clamped rather than rejected, and every clamp appends a
human-readable string to `g.clamps.notes`: pressure angle raised to 0.5°, cutter
depth kept positive and below 0.9R, tooth thickness kept positive and under the
pitch, fillet capped, tip capped at the pointed-tooth limit. Check
`g.clamps.any()` / `.notes` when a result looks unexpected — the geometry may not
be the geometry that was requested.

---

## 4. The test suite — and the long fight to make it trustworthy

**This section is the most valuable part of the handoff.** Every item below was a
real defect that produced confident, wrong output.

### 4.1 Design principle: bound the profile from both sides

The suite's core is a simulation of the actual generating rack, checking two
opposite bounds:

- **penetration** — no gear point may lie inside the cutter at any phase
  (material the tool would have removed is still there);
- **deviation** — every generated point must be *touched* by the cutter at its
  closest approach (the profile sits further from the tool than any cut could
  leave it).

Penetration alone is insufficient: an arbitrarily undersized profile passes it
trivially. Only both together pin the profile uniquely. The original suite had
penetration only.

Note honestly: **deviation would not have caught the original bridge bug** (it
read 8.9e-6 on the broken profile). What detects that fault is penetration
(with a correct sweep), junction gap, and inner-envelope. Redundant checks earn
their keep.

### 4.2 Bugs found *in the test suite*

1. **Rack sweep far too narrow.** The rack was swept ±0.5 pitch. The fillet's
   generating phase is **~1.07 pitches** away, so the window never contained the
   moment the fillet is cut. This single error hid both failure modes — with a
   correct range the old construction shows 0.15–0.22 mm of penetration, so the
   *existing* penetration check should have caught the original bug on its own.
   Fixed by `rack_travel_range()`, which derives the span from the geometry
   (flank contact range via `xi = tau/cos(alpha_t) - st/2`, fillet travel
   `[s_j - ac, -ac]`, root arc `[0, R*half_pitch]`, plus 0.6 pitch of padding).

2. **Insufficient rack copies.** With too few periodic copies of the cutter
   tooth, the relevant one is missed and a false deviation of ~8.5e-4 appears.

3. **Wrong envelope rule.** An "inner envelope" check asserted the boundary is
   `min(theta_involute, theta_trochoid)` at every radius. That fails on *correct*
   gears: below the junction the extended involute lies at a smaller angle, but
   the rack's straight flank is truncated there and never cuts that deep. The
   rule is only valid **within each curve's own generated domain** — the involute
   exists from `r_form = rb*hypot(1, max(L/rb, 0))` up. Corrected in
   `check_inner_envelope`.

4. **Wrong invariant: theta monotonicity.** Undercut profiles are *legitimately*
   re-entrant — the fillet curls back under the flank. The correct invariant is
   monotone **radius** (the profile is a graph over r, so it cannot
   self-intersect) plus `0 <= theta <= pi/z`. 161 sweep cases were failing on
   this misconception.

5. **Interpolation across constant-radius arcs.** Lookups used `np.interp` over a
   profile containing hundreds of points at exactly `ra` (tip arc) and `rf` (root
   arc), where radius is not invertible. Restricted to the strictly monotone
   flank+fillet span.

6. **Sharp-corner fillet check measured to sample points.** With `RootRadius = 0`
   the fillet lies *on* the tip-round centre path, so the point-sampling spacing
   (~1e-4) dominated the quantity being measured. Now measures to the polyline
   **segments**; error dropped from 2.0e-4 to 4.2e-8.

7. **Comparing models outside their shared domain.** The analytic distance treats
   the rack tooth as extending upward indefinitely; the drawn outline is
   truncated at the rack's root line, so its flank segments end there. Comparing
   above the tip radius produced a bogus 1.9e-2 disagreement. Gear points never
   rise above `ra`, so the comparison is restricted to that region.

8. **`id(g)` as a cache key — the nondeterminism.** `rack_base` cached the rack
   outline in a module dict keyed on `(id(g), n_round)`. CPython reuses addresses
   after garbage collection, so in a batch loop a freshly built `Gear` could be
   handed a **previous gear's rack outline**. Symptoms: a reproducible 0.9606 mm
   phantom penetration in batch runs that vanished in isolation, with the two
   `Gear` objects provably identical (every float attribute equal, `half_profile`
   arrays differing by exactly 0.0). The earlier unexplained 45 mm distance
   disagreement is almost certainly the same cause. **Fixed** by moving the cache
   onto the instance (`g.__dict__["_rack_cache"]`). The batch chunk that
   reproducibly failed now reports penetration 0.00 and zero failures.

   *Lesson for whoever continues: when results depend on execution history but
   the inputs are provably identical, suspect the harness, and specifically
   suspect any cache keyed on object identity.*

### 4.3 Accuracy floors, and how they were separated

Two distinct floors were identified and must not be confused with geometry error:

- **Rack polyline chord error** — exactly `rho*(1 - cos(delta/2))`. With 160
  points per round this is 2.8e-6 and was showing up as an identical "residual"
  for unrelated gears, which is what gave it away.
- **Phase discretisation** — distance to the cutter is quadratic in phase near
  contact. Small gears rotate far more per unit of rack travel, so the phase
  count now bounds **gear rotation**, not rack travel. A parabolic refinement of
  the sampled minimum is also applied (it helps the large-gear cases; it does
  little for tiny gears, where the nearest-feature/copy switches between phases).

### 4.4 Self-tests of the tests

- `test_rack_model` verifies the cutter's tip round is tangent to **both** the
  flank and the tip line (both distances = `rho`, to 1e-9). The user suspected
  this as a fault source; it was correct, and is now asserted every run.
- `test_sdf_matches_polyline` cross-checks the analytic distance against the
  independently constructed outline.
- `check_fillet_is_envelope` deliberately avoids the envelope derivation
  entirely: it asserts every fillet point lies exactly `rho` from the tip-round
  **centre path**. This is what proved the fillet correct while the rack
  simulation was still misreporting.

---

## 5. Debug artifacts and known rough edges

Several debugging passes timed out mid-flight. The following do **not** reflect
design intent — treat them as cleanup candidates, and do not infer intent from
them:

1. **`tooth_sdf` is dead code.** An exact analytic signed distance to the cutter
   tooth was written and validated (convex wedge eroded by `rho`, Minkowski-summed
   with a disc; three features instead of ~840 polyline segments; verified to
   1.2e-7 over 114 runs, and ~2x faster). `check_cut` was then reverted to the
   polyline + `matplotlib.path.Path` route and **no longer calls it**.
   *Recommendation: reinstate `tooth_sdf` inside `check_cut` and keep the polyline
   purely as the cross-check.* It removes the chord-error floor entirely and
   deletes the `Path.contains_points(radius=-1e-12)` containment test, which is
   the least trustworthy line in the suite.
2. **Tuning constants were changed by timed-out passes**, not by decision:
   `rack_base(n_round=220)` (was 160, then 420), `check_cut(npts=150)` (was 260),
   and the `nphase` clip now reads `/1e-3` with cap `4000` (a tightening pass to
   `/7e-4`, cap 9000 was applied and then lost). Current values pass the suite;
   they are not optimal.
3. **`nphase`'s lower bound is vestigial.** `per_pitch * (hi-lo) / pitch` survives
   as the clip floor from when `per_pitch` was the primary control. Harmless,
   confusing.
4. **`tier3.py` is a workaround, not a design.** It exists purely to chunk tier-3
   across separate processes because single runs exceeded the execution limit, and
   it accumulates results into `tier3.json`. Fold it into `sweep.py` once runtime
   allows.
5. **Minor:** unused import `GearParams` in `gear_tests.py` and `Gear` in
   `tier3.py`; `Clamps` has per-reason boolean fields that are set but never read
   (`any()` only inspects `notes`); `inv_u` is defined at the bottom of `gear.py`
   though used in `__init__` (works, reads oddly); `test_rack_model` computes its
   "round discretisation" figure over only half the vertex array.
6. **Superseded files** from the retired closed-form effort, kept only as
   reference: `gearbuild.py`, `gearbuild_rad_backup.py`, `emit.py`, `reformat.py`,
   `validate.py`, `gearplot.py`. `validate.py` in particular contains an older,
   *known-inadequate* cutter check. **Do not use it.** Likewise the emitted
   `constants_*.txt` / `x_of_T_*.txt` / `y_of_T_*.txt` outputs are stale — they
   predate the fillet-clamp fix and the junction fix.

---

## 6. Techniques from the retired closed-form work

Recorded because they are non-obvious and may be wanted again if a
single-expression target ever returns.

- **Periodicity without loops.** `t = 0.5 - atan(cot(pi*T))/pi` is a sawtooth
  giving `frac(T)`, so one tooth's formula serves all teeth; `T - t` is the tooth
  index. A tiny epsilon (`T + 1e-9`) moves the poles off the integers. Note it
  cannot be rounded to 5 decimals — at `0.00000` it divides by zero at every
  integer `T`; write it as a product (`0.00001*0.0001`) if decimals are capped.
- **Mirror folding.** `w = |2t - 1|` traverses a half-tooth out and back and
  `sgn(2t-1)` supplies the side, halving the section count.
- **Branchless piecewise via telescoping clamps.**
  `f(w) = f(0) + sum_i [F_i(clamp(w, W_{i-1}, W_i)) - F_i(W_{i-1})]` is exactly
  continuous at joints — no indicator windows, no dependence on `sgn(0)`.
  `clamp01(q) = (abs(q) - abs(q-1) + 1)/2`.
- **Angle units are not a `pi -> 180` substitution.** Arc/radius ratios and the
  involute function `tan(a) - a` are intrinsically radian quantities and need
  explicit `180/pi` factors. An earlier note claiming a simple swap was wrong.
- **Cross-checking two builds of the same maths** (degree vs radian) found a
  numerical fault — a diverging fixed-point iteration — that no geometric test
  caught. Worth remembering as a general technique.

---

## 7. Recommended next steps

Roughly in priority order.

1. **Reinstate `tooth_sdf` in `check_cut`** (§5.1). Exact, faster, removes the
   chord floor and the fragile containment test. Keep the polyline as the
   independent cross-check.
2. **Broaden tier 3.** 44 cases is thin for the definitive check. With
   `tooth_sdf` restored it should be affordable to run several hundred.
3. **Re-derive the tuning constants deliberately** (§5.2) and record why each
   value was chosen, with a convergence test rather than a guess.
4. **Add a self-intersection test on the full closed polygon.** Currently
   simplicity is argued indirectly (monotone radius + theta within the half
   pitch). Direct verification would be stronger, especially for severed and
   heavily undercut cases.
5. **Add regression fixtures.** Pin the known-good numbers for a handful of
   reference gears (junction radius, `theta0`, `theta_a`, tip/root radii) so
   future refactors fail loudly. `legacy_clamp=True` should stay as the
   negative fixture.
6. **Sweep module and helix more widely.** Almost all testing used module 1;
   module scaling is linear and low-risk, but untested is untested. Helix beyond
   45° is likewise unexplored.
7. **Then extensions**, once the above is solid: internal (ring) gears, tip
   chamfer/rounding, root relief, backlash/tooth-thinning allowance, a
   protuberance cutter, and 3D lofting along the helix.
8. **If a parametric `x(T)`, `y(T)` interface is wanted again** for the new tool,
   build it as a thin piecewise wrapper over `half_profile` — the arc-length
   section budget already exists. Do not resurrect the branchless machinery
   unless the target genuinely forbids branching.

---

## 8. File inventory

**Live**

| File | Role |
|---|---|
| `gear.py` | `GearParams`, `Clamps`, `Gear`. All geometry. No test/plot code. |
| `gear_tests.py` | Rack model, analytic distance, all checks, `run()` reporter. |
| `sweep.py` | Parameter grid, `tier1()`, tier 1/2 drivers. |
| `tier3.py` | Chunked tier-3 driver (workaround, see §5.4). |
| `render.py` | Figures: before/after, matrix, undercut detail. |

**Figures**: `before_after.png` (the fix), `matrix.png` (9 gears),
`undercut_detail.png` (notch, flank/fillet/junction), `fillet.png`
(fillet is a trochoid, not an arc), `tooth.png`, `numeric_T19_check.png`.

**Superseded** (reference only, see §5.6): `gearbuild.py`,
`gearbuild_rad_backup.py`, `emit.py`, `reformat.py`, `validate.py`,
`gearplot.py`, and all emitted `.txt` formula/constant files.

## 9. Quick start

```python
from gear import Gear, GearParams
from gear_tests import run

g = Gear(GearParams(module=1, pressure_angle=20, teeth=17, profile_shift=0.2,
                    helix_angle=0, addendum=1, dedendum=1.25, root_radius=0.38))
x, y = g.profile(per_tooth=400)          # closed cross-section, CCW
r, th = g.half_profile(400)              # (radius, angle-from-tooth-centre)
print(g.undercut, g.severed, g.clamps.notes)

run(GearParams(teeth=8))                 # full report for one gear
```

```bash
python3 sweep.py 1          # tier 1, ~6500 cases, ~35 s
python3 sweep.py 2 41       # tier 2, subset
python3 tier3.py 149 0 4    # tier 3, chunk 0 of 4
```

`GearParams` field order is positional-safe:
`(module, pressure_angle, teeth, profile_shift, helix_angle, addendum, dedendum,
root_radius)`. Angles in degrees; `addendum`, `dedendum`, `root_radius` are
multiples of the **normal** module.
