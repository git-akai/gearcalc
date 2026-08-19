# Handoff

Where the project stands, what is decided, and what to be careful of.

`docs/DESIGN.md` is the design of record and is current as of the head of
`main`; this file is the shorter route in. Where the two disagree, DESIGN.md
wins and this file is stale.

---

## 1. State

**Milestones 0–8 complete and in CI. 247 tests, ~25 s.**

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
| `gear-wasm` | ten entry points, JSON in / JSON out |
| `web` | Svelte 5 + TypeScript. Layout and event handling only |

### What works

**Parallel-axis gearing.** Involute + trochoid profile, undercut, severed teeth,
validated against a rack simulation from both sides over 1080 cases · primitives
(safeguarded `inv⁻¹`, Brent, bracketed Newton) · mesh, centre distance, exact
backlash, contact path · metrology (span, over-pins, JGMA 116-02 tables) ·
strength (critical section, form factor, bending stress, Hertz, face width,
helical throughout) · efficiency · automatic profile shift and altered addendum.

**Crossed axes.** `screw.rs`: lead angle `sin γ = z m_n/d` exact, both
efficiencies from a force balance, self-locking, sliding as a vector, elliptical
contact. The contact section was unified *first* — one Hertz formula with
lengthwise curvature as its parameter, line contact its degenerate value.

**Internal gears.** `ring.rs` and `shaper.rs`: the ring's flank, a shaper-cut
fillet, the flank/fillet tangency, the generation limit, and two mesh
interference conditions. Verified by simulating the cut.

**Trains.** Spur/helical and worm stages in one train, torque, backlash and cycle
accumulation, efficiency and backlash reported in **both** drive directions.

**Materials, export, UI.** An eight-material library with per-value provenance ·
DXF with exact arcs for external *and* internal gears · gear tabs with an
internal option, geartrain tabs with spur and worm stages.

### Driving it without a browser

```bash
cargo run --bin gear-cli -- show 17 0.2            # one gear's derived geometry
cargo run --bin gear-cli -- materials              # the library, with each value's basis
cargo run --bin gear-cli -- strength 17 43 2.0     # a worked mesh, end to end
cargo run --bin gear-cli -- train                  # a two-stage train
cargo run --bin gear-cli -- train mixed            # ...with a worm stage in it
cargo run --bin gear-cli -- worm 1 40 7 90         # a worm pair, both directions
cargo run --bin gear-cli -- wormstage 1 40 7 2     # a worm stage, end to end
cargo run --release --bin gear-cli -- verify 100   # the two-sided cutter check
python3 tools/worm_flank_curvature.py              # ZI vs ZN vs ZA, from the surface
```

`gear-cli strength 17 43 2.0` is the regression canary. Its numbers — `σ_F`
69.2 / 63.4 MPa, `σ_H` 692.7 MPa, ρ 1.723 mm, η 98.741 % — have survived every
refactor since milestone 5 **unchanged to the last digit**, including the whole
contact unification, and that check has caught more than the test suite has in
that area.

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

**Closed form unless it genuinely cannot be.** Eight scalar solves exist (DESIGN
§5), each monotone, each bracketed, none an optimiser, none with a tuning
parameter. When a published closed form turns out to be a *fit* — Hamrock–Dowson
for the contact ellipse's aspect ratio, the Lewis table for worm bending — the
solve is the honest route and the fit is refused.

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
`tests/extremes.rs` is the standing evidence. The converse also holds: a
*cutter* whose corner rounds overlap is not a tool, and is refused.

**Material data is the one place estimates are shipped**, deliberately, so the
calculator can produce a ballpark before the user has sourced anything. Every
value carries a `basis` — `datasheet`, `derived`, `chart`, `estimated`,
`overridden` — and anything that is not a plain datasheet reading must carry a
note saying what it is. A test enforces that.

**Say what is not modelled, next to the number.** A worm stage shows no bending
stress and says why on screen; its contact stress carries the ZI-flank caveat; a
ring shows the smallest tooth count its design could have had. Cheaper than a
footnote nobody reads, and it is what makes an omission a decision rather than a
gap.

---

## 3. Decisions that would otherwise be re-litigated

Each was reached by measurement and is expensive to rediscover.

**Lewis parabola over the 30° tangent** for the critical section. The 30° tangent
is independent of where the load acts, which is the one property the cantilever
model is meant to have. `CriticalSection::TangentAngle` is retained for a
standards-comparable number.

**Load sharing is deferred, with a written rationale.** Measured: once sharing is
allowed the governing point *becomes* the HPSTC, so a calibrated stiffness model
buys 0.0–0.2 %. Two conditions would change that and are named in §4.7.

**The S-N curve was withdrawn.** Fitting Basquin needs two points on a fatigue
curve; those do not exist for six of the eight materials. Two scalar allowables
replaced it.

**Torque, not force, is what a `Load` stores.** Every force is a projection, and
four are in play differing by `cos α_t`, `cos α_w`, `cos β_b`.

**Helical is not a special case anywhere.** There is no `if β == 0` in library
code. Spur results are values of the helical formulas.

**Contact is one formula.** `contact_stress` takes a lengthwise curvature;
`PARALLEL_AXES` is a named zero, and at it the elliptical patch's peak pressure
is *exactly* zero so `max(elliptical, line)` returns the line term bit for bit.
Line contact is a degenerate value, not a branch.

**A worm stage reports no bending stress.** The tooth whose form would be
measured is not the tooth that is loaded, the load case differs in kind rather
than by a factor, and no standard rates worm bending — so nothing could check
it. DESIGN §4.5.1.

**The worm is a ZI (involute helicoid).** Measured against ZN and ZA rather than
assumed: all three are the same surface family, the choice moves **only** contact
stress, and ZN comes out 1–15 % *below* ZI as the lead angle rises. Keeping ZI is
the conservative reading and it is the conjugate partner of the involute helical
wheel. The caveat is reported beside the number.

**Efficiency and backlash are reported directionally, for every stage kind.**
The specification asks for both directions on every stage and at train level.
`contact::efficiency` takes a `Drive`; `Directional::of` evaluates it twice.
Nothing decides in advance which meshes are symmetric — a parallel-axis mesh's
equality is *arrived at*, and an asymmetric-profile gear would report two
different numbers through the same code.

**Each stage kind keeps its own result type.** A worm stage has no bending
stress, no minimum face width from contact, and two efficiencies; forcing that
into `SpurResult` would have meant four `Option`s and a comment apologising for
each. What the kinds share is vocabulary, not the shape of their answers.

**A ring is cut by a shaper, and the tool is part of the part.** Two rings
identical in teeth, module and depth are different parts if they were shaped
differently. `Ring::new` takes the cutter.

**Every input bound lives in Rust**, invariant ones included, carrying its own
exclusivity and its own wording.

---

## 4. Traps

Things that looked reasonable, were wrong, and cost time. All are in DESIGN §12;
these are the ones most likely to be stepped on again.

- **`minimum_profile_shift` is a lower bound, not a recommendation.** It is −1.76
  at z = 43. Applied literally to both gears of a pair it drives `inv α_w`
  negative and the mesh leaves the involute domain. The automatic value is
  `max(x_min, 0)`.
- **`Y_F` and the load point must move together.** The two helical corrections
  pull in opposite directions; having one without the other is worse than
  neither.
- **Hertz must be evaluated at both single-pair boundaries.** Checking only the
  inner one made one physical mesh give two answers depending on labelling.
- **"A helical mesh slides along its teeth."** It does not, with parallel axes,
  and two documents said it did. Both surface velocities are `ω ẑ × r`, so the
  sliding has no axial component; the contact line is inclined and the sliding is
  exactly perpendicular to it at every helix angle. The efficiency formula was
  already exact and was being apologised for.
- **A check that cannot distinguish two cases is not evidence for either.** The
  ring cut simulation's corner-centre trajectory matched `corner_centre_at`
  *exactly*, which read like confirmation for several steps — but that match is
  invariant under mirroring the cutter's tooth, which is precisely what was
  wrong. Ask what a check would have said in the failing case.
- **Rack figures do not carry over to a pinion cutter.** A 0.38-module tip round
  is comfortable on a rack, whose tooth is wide at its tip; on a 20-tooth cutter
  with a 1.25 addendum the tip is 0.377 mm wide and two such rounds cannot both
  live on it.
- **A sweep must cover a whole engagement.** One circular pitch of travel is not
  enough; it left the ring's flank near its tip ungenerated and looked like a
  0.1 mm geometry error.
- **Choose the coordinate the curves are well behaved in.** Comparing the ring's
  cut by angular gap at equal radius reported 18 µm of "error" at the fillet's
  crown, where the fillet is *stationary in radius*. Point-to-curve distance
  removed it entirely.
- **Do not predict a threshold by hand when the computation exists to find it.**
  Three expectations in `ring.rs`'s history were wrong that way. Where a
  threshold is not independently known, assert the *comparison* — a shorter ring
  tooth clears more pinions — rather than the number.
- **A green local test run does not imply a green build.** `git add` before
  `nix build`; flakes only see tracked files. This has bitten twice.
- **Typechecking is not running unless you run it.** `cd web && npm run check`.

---

## 5. Open items

| Item | Where | Blocks |
|---|---|---|
| Radial-assembly interference — a *swept-motion* condition, deliberately not guessed | §4.11 | milestone 9 |
| The cut simulation cannot see below the generation limit: its cutter has no fillet | §4.11 | nothing |
| Mesh-phase coefficient setting the optimal λ | §4.10 | only the angular-profile-shift milestone |
| Tooth thickness tolerance (JGMA 1103-01, unavailable) | §4.6 | min/max on span and over-pins only |
| Worm contact ratio — the zone of action for a throated wheel | §4.5.1 | nothing |
| Worm profile drawing and DXF | §4.5.1 | nothing |
| `Driven By` as a train direction, rather than a reported efficiency | §4.9 | nothing |
| A coupled glass POM grade, if one is wanted back | §6.4 | nothing |

Known-approximate, documented at the call site rather than hidden:

- **`Y_β` omitted** — helical bending is conservative against a published ISO
  rating by up to ~25 %. Do not compare to an ISO rating without saying so.
- **A ZN worm's contact stress is 1–15 % below the reported ZI figure**, rising
  with lead angle.
- **A ring's flank below its generation limit is not a generated involute** —
  the profile extends the involute there, which is optimistic in a band of about
  0.08 mm on ordinary designs. Flagged per part.
- **Hardened 4340's fatigue allowable is the weakest number in the library.**
  0.5 × UTS with no published figure at that temper; likely nearer 700 than 750.

---

## 6. Next

### Milestone 9 — planetary stage

Most of what it needs exists. What it must supply itself is listed first,
because those are the two that need thought rather than assembly.

1. **Radial assembly.** The one interference condition §4.11 leaves out, and a
   planetary set is exactly what asks: can the planet be dropped in sideways?
   It is a swept-motion question — the teeth must pass each other on the way in
   — and writing it as a comparison of tip circles gives a negative clearance for
   every meshing pair, which is how you know that is not what it is. The tip
   circles do intersect at `cos θ₂ = (d² + r_a2² − r_a1²)/(2 d r_a2)` and its
   mate, which is the right place to start; the fallback the project would accept
   is a swept overlap test of the two generated profiles, in the spirit of
   `check_ring_cut`.
2. **The planet shift solve** that makes sun–planet and planet–ring centre
   distances agree. §4.8 has the analytic derivative and the closed-form bracket,
   and the ring search is provably complete because the required shift is
   strictly monotone in ring tooth count.

Then the assembly, which should be mostly wiring:

3. A `PlanetaryStage` beside `SpurStage` and `WormStage`, with **its own result
   type** — sun, planet and ring answers are not three copies of one shape.
4. Layout checks: equal spacing (`(z_sun + z_ring) / N` integral), planet–planet
   clearance, and the interference conditions of §4.11 on the planet–ring mesh.
5. Pennestrì–Freudenstein efficiency (§4.5.2) — closed form, all six drive modes
   from one piece of algebra. Note that it needs mesh efficiencies at *relative*
   speeds, not fixed-frame ones.
6. Tooth cycles: sun and ring see `N_planets` per revolution and a planet is a
   special case again (§4.9).

**What it inherits, and should not rebuild.** An internal mesh is a *negative
curvature*, and `hertz::relative_curvatures` already takes those — so contact
should need no new code, only geometry. `contact_stress` already takes a
lengthwise curvature. `Directional` already carries both drive senses. And
`ring::mesh_with` already gives the planet–ring interference conditions.

### After

| # | Milestone | Note |
|---|---|---|
| 10 | Crossed-axis spur | nearly free: `screw.rs` and the general Hertz both exist |
| 11 | Polish | train import/export, confirmations, error surfacing |
| — | Angular profile shift | §4.10. Blocked on the mesh-phase coefficient; the acceptance gate for any replacement model is already written — it must first reproduce `j_t = 2a′(inv α′ − inv α_w)` |

---

## 7. Working notes

- **Verify against something that shares no code.** The rack simulation, the
  pin-tangency measurement, `ezdxf`, the contact-half-width route, the AGM
  against Carlson's integrals, the numerical average of instantaneous loss, the
  parametric-surface curvature against the analytic one, the ring cut simulated
  from the cutter alone. Every one of those caught something self-consistent
  tests had passed.
- **Ask what property the answer must have.** The ring's flank disagreement was
  located by noticing the simulated envelope was *not an involute of the ring's
  base circle* — a property conjugate action guarantees, checkable without
  knowing the answer. That turned a symptom into a direction.
- **Run the app.** `nix shell nixpkgs#chromium --command chromium --headless
  --no-sandbox --disable-gpu --virtual-time-budget=20000 --screenshot=out.png
  http://localhost:5173`; `--dump-dom` gives the rendered DOM for grepping. Both
  run the JS, so they prove the app mounted and computed. Temporarily defaulting
  a tab to the state under test is a fair way to reach a panel a click would
  otherwise be needed for — revert it afterwards.
- **Record corrections rather than editing them away.** DESIGN §12 exists because
  the pattern is more useful than any single entry: *the errors that survived
  longest were the ones that looked reasonable and were never checked against
  something independent.*
