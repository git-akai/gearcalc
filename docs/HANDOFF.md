# Handoff

Where the project stands, what is decided, and what to be careful of.

`docs/DESIGN.md` is the design of record and is current as of the head of `main`;
this file is the shorter route in. Where the two disagree, DESIGN.md wins and
this file is stale.

---

## 1. State

**Milestones 0–6 complete and in CI; milestone 7's contact unification is done
and the screw-gear mathematics with it. What remains of 7 is the worm *stage*.
200 tests, ~27 s.**

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
geartrain with torque, backlash and cycle accumulation · the gear tab and the
geartrain tab with its stage accordion · DXF export.

### Driving it without a browser

```bash
cargo run --bin gear-cli -- show 17 0.2          # one gear's derived geometry
cargo run --bin gear-cli -- materials            # the library, with each value's basis
cargo run --bin gear-cli -- strength 17 43 2.0   # a worked mesh, end to end
cargo run --bin gear-cli -- train                # a two-stage train, end to end
cargo run --bin gear-cli -- worm 1 40 7 90       # a worm pair, both directions
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

### Milestone 7 — worm stage

**Unify the contact section first.** The full plan is DESIGN §4.7; the shape of
it is that three things unify, one parameter each, and in every case the present
parallel-axis result is the degenerate value:

| | unified by | parallel axes is |
|---|---|---|
| Geometry — path of contact | shaft angle `Σ` | `Σ = 0` |
| Pressure — Hertz | lengthwise relative curvature `1/R_L` | `1/R_L = 0` |
| Friction — efficiency | the sliding **velocity vector** | lengthwise component 0 |

Use **Carlson symmetric elliptic integrals**, not the classical `K(e)`/`E(e)`
form: the classical one requires knowing which semi-axis is major — itself a
branch — and is ill-conditioned exactly as the ellipse degenerates, which is the
limit that matters here.

The one real discontinuity is geometric, not elastic: a tooth has finite face
width, so an ellipse longer than the face is truncated by the tooth. Hence
`σ_H = max(σ_elliptical, σ_line over the available length)`, exact at both ends.

**Order matters.** Steps 1–4 below change no answer the tool currently gives,
which is the point — unify *before* there is anything new to get wrong.

1. ✅ Carlson integrals, tested alone. `gear-core/src/elliptic.rs`.
2. ✅ General Hertz on them — sphere-on-sphere and sphere-on-plane have exact
   closed forms and need no gear. `gear-core/src/hertz.rs`.
3. ✅ Swap `contact_stress` to the general form at `1/R_L = 0`. **The acceptance
   gate must pass before any crossed-axis geometry is added**: every existing
   contact and efficiency check, plus the canary to the last digit. It passed,
   and the canary is bit-identical rather than merely close — a test asserts
   `==` against the line formula, because agreement to a tolerance would have
   been equally consistent with the line term having been re-derived.
4. ✅ Sliding as a vector; parallel-axis efficiency unchanged first.
   `contact::sliding_velocity`. This one **corrected the design** rather than
   confirming it — see §4 below.
5. 🔶 Worm geometry, `sin γ = z m_n/d` (exact — no iteration), self-locking.
   **Built and verified** in `screw.rs`: pitch-point geometry, sliding, and both
   drive directions derived from a force balance rather than quoted — at 90° it
   reproduces the two published closed forms to 1e-14, and energy balances at
   every shaft angle. Drive it with `gear-cli worm 1 40 7 90`.

**What is left of milestone 7 is the worm *stage*, not its mathematics.** The
mesh is complete: geometry, sliding, both efficiencies, self-locking, the
relative curvatures and the contact patch, all in `screw.rs` and driveable with
`gear-cli worm`. What remains is a `WormStage` beside `SpurStage` — which is what
finally splits `train.rs` into a directory — then torque and backlash through the
train, the wasm boundary, and the UI.

Three things that stage inherits and should not re-litigate:

- **Rate the contact on the wheel's torque, not the worm's.** Which torque is
  held fixed decides which way friction moves the flank load — down at fixed
  input, up at fixed output — and only the second is the conservative reading.
  `Screw::normal_force` takes a `Member` so the choice has to be made explicitly.
- **No bending stress**, decided below.
- **The worm is a ZI (involute helicoid)**, which is what makes each flank a
  cylinder and the parallel case an exact limit. ZA and ZN worms would need their
  own flank curvature.

What steps 1–4 leave in place for step 5: `contact_stress` already takes a
lengthwise curvature (`PARALLEL_AXES` is the named zero every current call site
passes), `elliptical_contact` already returns the patch for any curvature pair,
and `sliding_velocity` already takes an arbitrary second axis. Step 5 supplies a
different `axis_2` and a non-zero `1/R_L`; it should not need to add a branch to
any of them.

Both of milestone 7's open decisions are now taken:

- **Efficiency is genuinely direction-dependent** for a worm, unlike every mesh
  so far. Built: `screw.rs` derives both directions from a force balance, and
  self-locking falls out as a sign change at `μ ≥ cos α_n tan γ`.
- **A worm stage reports no bending stress.** Decided against showing the
  parallel-axis figure marked indicative, because there is no reason to think it
  is near the truth: the tooth whose form would be measured is not the tooth that
  is loaded, the load case differs in kind rather than by a factor, and no
  standard rates worm bending, so nothing could check it. Reasoning in DESIGN
  §4.5.1. The stage reports contact stress, sliding velocity and power loss
  instead — which is what worm drives are actually limited by.

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
