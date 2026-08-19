# Handoff

Where the project stands, what is decided, and what to be careful of.

`docs/DESIGN.md` is the design of record and is current as of the head of `main`;
this file is the shorter route in. Where the two disagree, DESIGN.md wins and
this file is stale.

---

## 1. State

**Milestones 0–7 complete and in CI. 216 tests, ~26 s.**

```bash
nix develop                       # or `direnv allow` once
cargo nextest run                 # the suite
nix flake check                   # what CI runs: build, clippy --deny warnings, fmt, tests
cd web && npm run dev             # the application
```

| Crate | Holds |
|---|---|
| `gear-core` | all mathematics. `serde` is its only dependency, deliberately |
| `gear-io` | DXF writer, TOML material library |
| `gear-wasm` | four entry points, JSON in / JSON out |
| `web` | Svelte 5 + TypeScript. Layout and event handling only |

### What works

Geometry (involute + trochoid profile, undercut, severed teeth, validated against
a rack simulation from both sides over 1080 cases) · primitives (safeguarded
`inv⁻¹`, Brent, bracketed Newton) · mesh (centre distance, exact backlash,
contact path) · metrology (span, over-pins, JGMA 116-02 tables) · strength
(critical section, form factor, bending stress, Hertz line contact, face width,
helical throughout) · efficiency · an eight-material library with per-value
provenance · automatic profile shift and altered addendum · a spur/helical
geartrain with torque, backlash and cycle accumulation · **crossed-axis screw
gearing — lead angle, both efficiencies, self-locking, elliptical contact and
backlash — and a worm stage a train can hold beside a spur one** · the gear tab
and the geartrain tab with its stage accordion, spur and worm · DXF export.

### Driving it without a browser

```bash
cargo run --bin gear-cli -- show 17 0.2          # one gear's derived geometry
cargo run --bin gear-cli -- materials            # the library, with each value's basis
cargo run --bin gear-cli -- strength 17 43 2.0   # a worked mesh, end to end
cargo run --bin gear-cli -- train                # a two-stage train, end to end
cargo run --bin gear-cli -- worm 1 40 7 90       # a worm pair, both directions
cargo run --bin gear-cli -- wormstage 1 40 7 2   # a worm stage, end to end
cargo run --release --bin gear-cli -- verify 100 # the two-sided cutter check
```

`gear-cli strength 17 43 2.0` is the regression canary for this codebase. Its
numbers — `σ_F` 69.2 / 63.4 MPa, `σ_H` 692.7 MPa, ρ 1.723 mm, η 98.741 % — have
survived three refactors of the strength path **unchanged to the last digit**,
and that check has caught more than the test suite has in that area.

---

## 2. The rules, and why they are rules

**No engineering calculation in TypeScript.** If a number appears in the UI, Rust
computed it. TypeScript formats. This is what keeps the Rust test suite
meaningful — otherwise logic migrates to where nothing tests it. It has real
teeth: it is why picking between two published material states could not be done
in the view, and why every input bound is returned by Rust rather than declared
in the front end.

**Inputs are the only state.** Outputs are recomputed, never stored, so nothing
can go stale. Shared-within-a-stage values live once on the stage; `k₂ = 2 − k₁`
is derived, so the invariant is unwritable rather than merely tested.

**Closed form unless it genuinely cannot be.** Five scalar solves exist, each
monotone, each bracketed, none an optimiser, none with a tuning parameter.

**No ISO/AGMA correction factors** — `Y_β`, `K_A`, `K_v`, `K_Fβ`, `K_Fα`, `Z_ε`,
`Z_β`. Their validated bands are narrow against modern designs, they are only
balanced as a complete set against `σ_Flim` values this project does not have,
and they buy precision at the cost of accuracy. **`Y_S` is the deliberate
exception** and the reasoning is in DESIGN §4.7 — it points the other way
(`≥ 1`, so omitting it would be *un*conservative by roughly 2×), it is local
rather than population-calibrated, and its validity range is reported per result.

**An input limit means "could this gear exist?", not "would anyone want it?"** A
guard that refuses a legal shape will one day refuse a legitimate design. A
one-tooth gear, an 85° helix and a negative addendum are all constructible, and
`tests/extremes.rs` is the standing evidence.

**Material data is the one place estimates are shipped**, deliberately, so the
calculator can produce a ballpark before the user has sourced anything. Every
value carries a `basis` — `datasheet`, `derived`, `chart`, `estimated`,
`overridden` — and anything that is not a plain datasheet reading must carry a
note saying what it is. A test enforces that.

---

## 3. Decisions that would otherwise be re-litigated

Each of these was reached by measurement and is expensive to rediscover.

**Lewis parabola over the 30° tangent** for the critical section. The 30° tangent
is independent of where the load acts, which is the one property the cantilever
model is meant to have. `CriticalSection::TangentAngle` is retained for a
standards-comparable number.

**Load sharing is deferred, with a written rationale.** Measured: once sharing is
allowed the governing point *becomes* the HPSTC, so a calibrated stiffness model
buys 0.0–0.2 %. Two conditions would change that and are named in §4.7.

**The S-N curve was withdrawn.** Fitting Basquin needs two points on a fatigue
curve; those do not exist for six of the eight materials. Two scalar allowables
replaced it, pairing with the peak and cyclic input torques the spec already had.

**Torque, not force, is what a `Load` stores.** Every force is a projection, and
four are in play differing by `cos α_t`, `cos α_w`, `cos β_b`. Torque is
invariant under any redefinition of a radius.

**Helical is not a special case anywhere.** There is no `if β == 0` in library
code. Spur results are values of the helical formulas — checked by the canary
above.

**One condition per material entry.** Heat treatment was already separate
entries; moisture used to be paired numbers inside one entry. Collapsing the
second onto the first deleted a field, a resolver and three types.

**Every input bound lives in Rust**, invariant ones included, carrying its own
exclusivity and its own wording.

---

## 4. Traps

Things that looked reasonable, were wrong, and cost time. All are recorded in
DESIGN §12; these are the ones most likely to be stepped on again.

- **`minimum_profile_shift` is a lower bound, not a recommendation.** It is −1.76
  at z = 43. Applying it literally to both gears of a pair drives `inv α_w`
  negative and the mesh leaves the involute domain entirely — a default 17:43
  stage simply would not solve. The automatic value is `max(x_min, 0)`.
- **`Y_F` and the load point must move together.** Measuring the form on the
  virtual spur gear while passing the *real* gear's roll parameter puts the load
  at the wrong place on the flank. The two helical corrections pull in opposite
  directions; having one without the other is worse than having neither.
- **`ε_αn = ε_α/cos²β_b` is ISO's relation, not an identity.** Measured against a
  directly-built virtual pair it is 0.20 % off at β = 30°, costing 0.38 % on
  `Y_F`. Bounded, and stated rather than assumed.
- **The `/ε_α` in the efficiency formula is load-sharing bookkeeping.** Drop it
  and the mesh implicitly transmits `ε_α F_n`.
- **Hertz must be evaluated at both single-pair boundaries.** Checking only the
  inner one made one physical mesh give two answers depending on which gear was
  labelled 1.
- **`profile()` on a virtual spur gear is meaningless** — fractional tooth count.
  A `debug_assert` catches it.
- **A green local test run does not imply a green build.** Twice the working tree
  held something a fresh checkout does not. `git add` before `nix build`; flakes
  only see tracked files.
- **Typechecking is not running.** The headless render caught an automatic face
  width overflowing its field as `0.42646(` while every test was green.
- **"A helical mesh slides along its teeth."** It does not, with parallel axes,
  and two documents said it did. Both surface velocities are `ω ẑ × r`, so the
  sliding has no axial component; the contact line is inclined and the sliding
  is exactly perpendicular to it at every helix angle. The efficiency formula
  was already exact and was being apologised for. The general lesson is the one
  §12 keeps making: the claim sounded physical, and nothing had measured it.

---

## 5. Open items

| Item | Where | Blocks |
|---|---|---|
| Crossed-axis contact — **unified; steps 1–4 built, worm geometry pending** | §4.7 | milestone 7 |
| Mesh-phase coefficient setting the optimal λ | §4.10 | only the angular-profile-shift milestone |
| Sinusoidal `x(θ)`, or another interpolation | §4.10 | same |
| What the eccentric mechanism can physically follow | §4.10 | same |
| Tooth thickness tolerance (JGMA 1103-01, unavailable) | §4.6 | min/max on span and over-pins only |
| A coupled glass POM grade, if one is wanted back | §6.4 | nothing |

Known-approximate, documented at the call site rather than hidden:

- **`Y_β` omitted** — helical bending is conservative against a published ISO
  rating by up to ~25 % at high helix and overlap. Do not compare to an ISO
  rating without saying so.
- **Efficiency is exact for parallel axes, and the single friction coefficient
  is the limit that remains.** The earlier note here said profile-only sliding
  under-stated helical loss; building the sliding as a vector showed there is no
  lengthwise component to charge for when the axes are parallel — the sliding is
  transverse and the contact line is not. DESIGN §12.
- **Hardened 4340's fatigue allowable is the weakest number in the library.**
  0.5 × UTS with no published figure at that temper, and the endurance ratio is
  known to fall away above ~1400 MPa UTS. Likely nearer 700 than 750.

Spec fields not yet surfaced (all additive, none blocking): axis angle (belongs
with milestone 10), train import/export, tab copy/delete confirmations on the
geartrain side.

---

## 6. Next

### Milestone 8 — ring gear geometry

**Started.** `shaper.rs` has the generating curve: the fillet swept by a pinion
cutter's tip corner, for an external or an internal workpiece. The rack is its
`z_c → ∞` limit and that is *measured*, so the two are one construction rather
than two — the residue at a million cutter teeth is the pitch line's own
curvature term, not slop.

`ring.rs` has the involute side: the radii the other way round, and the one sign
that makes a ring a ring — its tooth *widens* outward, because what narrows is
the space. Checked as the complement it is, swept across the flank rather than
sampled at the pitch circle where the backwards sign would also pass.

**What is left, in order:**

1. ✅ **The fillet's phase**, from the offset-involute fact — the locus at `ρ`
   from an involute along its normal is another involute of the same base
   circle, origin shifted by `ρ/r_b`. Exact, and confirmed against the rack's
   `a_c` in the limit.
2. ✅ **The junction.** It is a *tangency*, not a crossing — the cutter's flank
   ends exactly where its round begins — so bracketing found nothing, correctly.
   Closed form from the line of action instead: conjugate points share a
   position on it, and for an internal pair the two distances differ by
   `a sin α_t` rather than summing to it.
3. ✅ **Assembly** — `Ring::sections`, `half_profile` and `profile` mirror the
   external gear's, with the radius climbing rather than falling.
4. ✅ **A CAD-grade outline and the export path.** `Ring::outline` gives exact
   arcs at a chord tolerance like the external gear's; `gear_to_dxf` was
   generalised to `outline_to_dxf(polyline, circles, opts)` so both kinds feed
   one writer; `solve_ring`, `ring_profile` and `export_ring_dxf` cross the wasm
   boundary; and the gear tab has an **Internal (ring) gear** option with its
   cutter, verified in the browser through the real wasm.

   Measurement over teeth and pins and the strength rating are *not* shown for a
   ring, with the reason on screen: they are external-gear constructions and the
   internal equivalents do not exist here yet.
5. **Interference checks** a rack cutter never needs: involute (trimming),
   trochoid, and radial-assembly. The `z_ring − z_cutter ≥ 10` rule of thumb is
   a convention; the geometric condition behind it is what belongs here.
6. ✅ **The gate** — `verify::check_ring_cut` simulates the cut and compares:
   **3.6 µm** over four ring/cutter pairs, which is its own binning. It found
   three faults on the way, one of them in the geometry (the default cutter was
   not a manufacturable tool) and two in itself. DESIGN §12 has the account; the
   part worth carrying is that the check which *seemed* to exonerate the
   simulation was invariant under the very mirroring that was wrong.

   Superseded plan: `verify.rs`'s two-sided bound (penetration *and* deviation)
   against the shaper. The envelope test in `shaper.rs` is that idea applied to
   one curve; this applies it to the whole profile.

**A ring's smallest tooth count is a function, not a number.** The tip clears
the base circle while `z > 2 h_a cos β / (1 − cos α_t)`: 34 teeth at a full
addendum and 20°, but 20 at a 0.6 addendum, 22 at 25°, 63 at 14.5°, and 23 at a
30° helix. The module cancels, as it must for a statement about tooth counts.
The familiar "internal gears need about 34 teeth" is one row of that table and
would be wrong for the other five.

**One trap already paid for.** The rolling sense has *no* `σ` in it. An internal
workpiece does turn the other way relative to its cutter — but the cutter also
turns the other way for increasing travel, because its corner points outward
rather than inward, and the two reversals cancel. Assuming otherwise put the
fillet 0.017 mm inside the tool half a tooth later, which the envelope test
caught and no smoke test would have.



The largest remaining piece of *new geometry*: an internal profile cut by a
pinion-shaped shaper rather than a rack, its trochoid, and the interference
checks a rack cutter never needs. Its gate is its own rack-equivalent
validation, the way milestone 1's was.

Milestone 7 is done. What it left, and what a ring gear should copy:

- **The contact model takes parameters, not branches.** `contact_stress` takes a
  lengthwise curvature (`PARALLEL_AXES` is the named zero), `elliptical_contact`
  takes any curvature pair, `sliding_velocity` any second axis, and
  `relative_curvatures` any per-body principal curvatures including negative
  ones. An internal mesh is a negative curvature, so it should need no new
  contact code at all — only geometry.
- **Each stage kind keeps its own result type**, and the train reads what it
  needs through a small interface (`ratio`, `efficiency`, `output_backlash`,
  `set_kinematics`). A planetary stage will want the same freedom; do not force
  it into `SpurResult`.
- **Say what is not modelled, next to the number.** The worm stage reports no
  bending stress and states why on screen, and its contact stress carries the
  ZI-flank caveat. That is cheaper than a footnote nobody reads.

Deliberately not built in milestone 7, none of it blocking:

| Gap | Why |
|---|---|
| Worm contact ratio | needs the zone of action for a throated wheel; the model is a cylindrical (crossed-helical) pair |
| Worm profile drawing and DXF | the gear tab draws parallel-axis involutes; a worm needs its own section |
| Automatic worm length and wheel face width | the published rules are proportions — conventions, which §4.7's policy excludes |
| `Driven By` on a worm stage | torque propagates worm→wheel; back-driving is reported as an efficiency, not modelled as a train direction |
| Throated wheels | a throated wheel is concave along the contact, lowering `1/R_L`; the cylindrical figure is conservative. It would enter as a curvature, not a rework |

### After

| # | Milestone | Note |
|---|---|---|
| 8 | Ring gear geometry | largest remaining piece of new geometry; pinion-shaped cutter trochoid, tip interference |
| 9 | Planetary stage | ring search is provably complete — required planet shift is monotone in `z_ring`, with a closed-form bracket |
| 10 | Crossed-axis spur | nearly free once milestone 7 exists, which is why worm comes first |
| 11 | Polish | train import/export, confirmations, error surfacing |
| — | Angular profile shift | §4.10. Blocked on the mesh-phase coefficient, and the acceptance gate for any replacement model is already written: it must first reproduce `j_t = 2a′(inv α′ − inv α_w)` |

---

## 7. Working notes

- **Verify against something that shares no code.** The rack simulation, the
  pin-tangency measurement against the generated flank, `ezdxf`, the
  contact-half-width route, the numerical average of instantaneous loss. Every
  one of those caught something self-consistent tests had passed.
- **Run the app.** `nix shell nixpkgs#chromium --command chromium --headless
  --no-sandbox --disable-gpu --virtual-time-budget=15000 --screenshot=out.png
  http://localhost:5173` renders it; `--dump-dom` gives the rendered DOM for
  grepping. Both run the JS, so they prove the app mounted and computed.
- **Record corrections rather than editing them away.** DESIGN §12 exists because
  the pattern is more useful than any single entry: *the errors that survived
  longest were the ones that looked reasonable and were never checked against
  something independent.*
