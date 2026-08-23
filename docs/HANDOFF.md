# Handoff

Where the project stands, what is decided, and what to be careful of.

`docs/DESIGN.md` is the design of record and is current as of the head of
`main`; this file is the shorter route in. Where the two disagree, DESIGN.md
wins and this file is stale.

---

## 1. State

**Milestones 0–10 complete and in CI. 322 tests, ~26 s.** Milestone 11 is
under way: the ring-drawing defect below is fixed, and the rim circle with it.

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
| `gear-wasm` | eleven entry points, JSON in / JSON out |
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
lengthwise curvature as its parameter, line contact its degenerate value. **Worm
drives and crossed gear pairs are one stage**, differing only in whether the
first member's diameter is given or derived from a helix angle: `sin γ = cos β`
does the rest.

**Internal gears.** `ring.rs` and `shaper.rs`: the ring's flank, its profile
shift, a shaper-cut fillet **at the centre distance the shift puts the tool at**,
the flank/fillet tangency, the generation limit, two mesh interference
conditions, and a bending rating. Verified by simulating the cut — 2.5–2.7 µm
across shifts −0.4 … +0.5. A cut that generates **no** fillet is `fillet: None`
rather than a fillet of zero length, and every consumer answers it; the drawn
outline is gated on density and on the involute law (§12). The viewport shades
the rim rather than the bore, out to `Ring::rim_radius`.

**Planetary sets.** `planetary.rs` and `train/planetary.rs`: the planet shift
that makes the two centre distances agree, the ring search, layout checks, Willis
kinematics, Pennestrì–Freudenstein efficiency in all six arrangements, and the
backlash referred to the output shaft.

**Trains.** Spur/helical, worm **and planetary** stages in one train, torque,
backlash and cycle accumulation, efficiency and backlash reported in **both**
drive directions.

**Metrology.** Span over teeth and over-pins for external gears; **between-pins
for rings**, from the same relation at the opposite sign. JGMA 116-02 tolerance
tables.

**Materials, export, UI.** An eight-material library with per-value provenance ·
DXF with exact arcs for external *and* internal gears · gear tabs with an
internal option and its own measurement, geartrain tabs with spur, worm and
planetary stages.

### Driving it without a browser

```bash
cargo run --bin gear-cli -- show 17 0.2            # one gear's derived geometry
cargo run --bin gear-cli -- materials              # the library, with each value's basis
cargo run --bin gear-cli -- strength 17 43 2.0     # a worked mesh, end to end
cargo run --bin gear-cli -- train                  # a two-stage train
cargo run --bin gear-cli -- train mixed            # ...with a worm stage in it
cargo run --bin gear-cli -- worm 1 40 7 90         # a worm pair, both directions
cargo run --bin gear-cli -- wormstage 1 40 7 2     # a worm stage, end to end
cargo run --bin gear-cli -- crossed 17 23 90       # a crossed pair, swept over the split
cargo run --bin gear-cli -- planetary 17 17 3      # every ring count that can work
cargo run --bin gear-cli -- planetstage 24 18 60 3 # a planetary stage, six modes
cargo run --release --bin gear-cli -- verify 100   # the two-sided cutter check
python3 tools/worm_flank_curvature.py              # ZI vs ZN vs ZA, from the surface
```

`gear-cli strength 17 43 2.0` is the regression canary. Its numbers — `σ_F`
69.2 / 63.4 MPa, `σ_H` 692.7 MPa, ρ 1.723 mm, η 98.741 % — have survived every
refactor since milestone 5 **unchanged to the last digit**, including the whole
contact unification, the signed-mesh collapse and the bending-continuity fix, and
that check has caught more than the test suite has in that area.

---

## 2. The rules, and why they are rules

**No engineering calculation in TypeScript.** If a number appears in the UI, Rust
computed it. TypeScript formats. This is what keeps the Rust test suite
meaningful — otherwise logic migrates to where nothing tests it.

**Inputs are the only state.** Outputs are recomputed, never stored, so nothing
can go stale. Shared-within-a-stage values live once on the stage; `k₂ = 2 − k₁`
is derived, so the invariant is unwritable rather than merely tested. A planetary
set makes the same point harder: its two meshes want *different* invariants
(`k₁ + k₂ = 2` external, `k₁ = k₂` internal) and share the planet, so one stored
`k` fixes all three.

**Closed form unless it genuinely cannot be.** Nine scalar solves exist (DESIGN
§5), each monotone, each bracketed, none an optimiser, none with a tuning
parameter. When a published closed form turns out to be a *fit* — Hamrock–Dowson
for the contact ellipse's aspect ratio, the Lewis table for worm bending — the
solve is the honest route and the fit is refused.

**Find the parameter, not the branch.** Where two cases look different, look for
the value of one construction that reproduces the other. Line contact is the
zero of the lengthwise curvature; the rack is the shaper at `z_c → ∞`; a spur
gear is a helical one at `β = 0`; **a ring is a gear with a negative tooth
count**. This is not tidiness — every surviving `match kind` is a place where two
answers can silently disagree, and §12 is largely a record of exactly that.

**No ISO/AGMA correction factors** — `Y_β`, `K_A`, `K_v`, `K_Fβ`, `K_Fα`, `Z_ε`,
`Z_β`, and a planetary mesh-load factor. Their validated bands are narrow against
modern designs, they are only balanced as a complete set against `σ_Flim` values
this project does not have, and they buy precision at the cost of accuracy.
**`Y_S` is the deliberate exception** and the reasoning is in DESIGN §4.7.

**Helical is not a lesser case: parity with spur throughout.** If a spur gear
gets a number, the helical one does too — rated on its virtual spur section, not
refused and not rated transversely (which mixes planes and under-predicts by
about `cos β`). This is why `Ring::virtual_spur` and a fractional-tooth-count
`Ring` exist.

**An input limit means "could this gear exist?", not "would anyone want it?"** A
guard that refuses a legal shape will one day refuse a legitimate design.
`tests/extremes.rs` is the standing evidence. The converse also holds: a *cutter*
whose corner rounds overlap is not a tool, and a shaft with `T ω ≤ 0` is not an
input, and both are refused.

**Material data is the one place estimates are shipped**, deliberately. Every
value carries a `basis` and anything that is not a plain datasheet reading must
carry a note saying what it is. A test enforces that. The planet's reversed
bending allowable is derived this way rather than being a bare multiplier.

**Say what is not modelled, next to the number.** A worm stage shows no bending
stress and says why on screen; a ring shows the smallest tooth count its design
could have had; a planetary set states in its own notes that it assumes the
planets share load equally. Cheaper than a footnote nobody reads.

**And say nothing where nothing is known — but check that "nothing" is not itself
a discontinuity.** Refusing to answer looked like caution when the bending
correction hit a flank tangency. It was a cliff: a number becoming no number,
with nothing physical happening at that tooth count. See §4 below.

---

## 3. Decisions that would otherwise be re-litigated

Each was reached by measurement and is expensive to rediscover.

**Lewis parabola over the 30° tangent** for the critical section. The 30° tangent
is independent of where the load acts, which is the one property the cantilever
model is meant to have. `CriticalSection::TangentAngle` is retained for a
standards-comparable number. NASA TM-107012 makes the same choice for internal
teeth, independently.

**A ring is a gear with a negative tooth count, shift and radius.** Every
internal relation is the external one under that sign — the tooth sums, the
efficiency term, the operating radii, the relative curvature, the contact path.
`MeshKind` appears in arithmetic nowhere.

**On a ring it is the *space* that the external formulas describe**, not the
tooth: the space is where the pinion's tooth goes and is generated the way a
pinion's tooth is. So a larger `k` or `x` makes a ring's tooth *thinner*. Not a
free choice — measured against tooth thicknesses at the operating circles, the
tooth reading is 0.63 mm from zero backlash at `k = 1.2`.

**A ring has no dedendum input and no root-radius coefficient.** Both are its
cutter's: the root circle is where the tool reaches, `a_cut + r_tip` exactly.

**A profile shift is *where the tool sits*, for a shaper.** A rack's pitch line is
a machine setting so shifting it leaves the rolling alone; two pinions have their
ratio fixed by their tooth counts, so the pitch point moves with the centre
distance and the rolling circles with it. One factor `a / a_ref` carries all of
it, and is exactly 1 at zero shift.

**`ρ_F` is a fillet property at any tooth size.** When the critical section
climbs onto the involute flank the notch is still the fillet, read at the
junction. Reading the involute's own curvature there is not a notch radius.

**Load sharing along the path is deferred, with a written rationale.** Measured:
once sharing is allowed the governing point *becomes* the HPSTC, so a calibrated
stiffness model buys 0.0–0.2 %.

**The S-N curve was withdrawn.** Fitting Basquin needs two points on a fatigue
curve; those do not exist for six of the eight materials.

**Torque, not force, is what a `Load` stores.** Every force is a projection, and
four are in play differing by `cos α_t`, `cos α_w`, `cos β_b`.

**Contact is one formula.** `contact_stress` takes a lengthwise curvature;
`PARALLEL_AXES` is a named zero, and at it the elliptical patch's peak pressure
is *exactly* zero. Line contact is a degenerate value, not a branch.

**A worm stage reports no bending stress.** The tooth whose form would be
measured is not the tooth that is loaded, and no standard rates worm bending.

**The worm is a ZI (involute helicoid).** Measured against ZN and ZA rather than
assumed; ZN comes out 1–15 % *below* ZI as the lead angle rises, so ZI is the
conservative reading.

**Efficiency and backlash are reported directionally, for every stage kind.**
Nothing decides in advance which meshes are symmetric — a parallel-axis mesh's
equality is *arrived at*.

**Each stage kind keeps its own result type.** A worm stage has no bending
stress; a planetary has three shafts, two meshes and a planet that is neither.
Forcing those into one shape would mean a row of `Option`s and a comment
apologising for each.

**A planetary needs the held shaft named, not just the driven one.** Three shafts
means naming two; the specification names one, which does not determine an
answer. DESIGN §8.1.

**A ring is cut by a shaper, and the tool is part of the part.** Two rings
identical in teeth, module and depth are different parts if they were shaped
differently. `Ring::new` takes the cutter and keeps it.

---

## 4. Traps

Things that looked reasonable, were wrong, and cost time. All are in DESIGN §12;
these are the ones most likely to be stepped on again.

- **A default written down in two languages is the same trap as a duplicated
  formula, and worse, because only one side has tests.** The gear tab's cutter
  carried the *rack's* 0.38 tip round in TypeScript while Rust held 0.2 — a
  20-tooth shaper's tip is 0.377 modules wide and cannot hold two 0.38 rounds —
  so every ring the UI drew was cut by a tool that generates no fillet. The core
  was right throughout and could not see it. Defaults are now served from
  `gear_wasm::defaults`, and a fresh tab cannot be built before the core loads.
  **If you find yourself typing an engineering number into `.ts`, that is the
  bug.**
- **An absent thing is not a zero-length thing.** The missing fillet was stored
  as `s_j = s_root = 0`, so everything downstream cheerfully asked for a curve
  that was not there — and the failure was silent and *plausible*: a `NaN` arc
  length collapsed a 600-point outline to seven, which draws as a polygon that
  looks deliberate. `Ring::fillet` is an `Option` now. When something can be
  absent, say so in the type, or every consumer has to remember.
- **Assert the property, not a number the failure also satisfies.**
  `outline.len() > 200` passed while sixty teeth returned seven points each. The
  gate that catches it asserts points *per tooth* against the number requested,
  and that each flank chord stands `√(r² − r_b²)` from the centre — the involute
  law, checkable from the drawn points alone. Before trusting a new gate, run it
  against the broken code: `git worktree add` a detached HEAD, copy the test in,
  and watch it fail.
- **A duplicated formula is a place where two answers can differ, and the copy
  nothing exercises is the one that is wrong.** The hand-written internal
  relative curvature was wrong two ways at once — 50 % at the pitch point of a
  17/51 pair, and negative on a 25/41 — and unreachable, because `ContactPath`
  refused internal meshes. Both would have gone live together.
- **A check built from the thing under test measures nothing.** The ring cut
  simulation derived the cutter's tooth the same way the model did, so it agreed
  to 2.7 µm on a ring whose cutter was 0.44 mm out of place. Ask what the check
  would say in the failing case — and if you cannot make it fail, it is not a
  check.
- **Refusing to answer can be the discontinuity.** `Y_S` was declined on a flank
  tangency to avoid a 17 % jump. But a number becoming no number is also a jump,
  and nothing physical happens at 151 teeth. The cause was one step further
  back: `ρ_F` had stopped being a fillet property.
- **A limit check confirms a trend, not a resolution.** Two correct limit checks
  passed on a radial-assembly test whose rotation sweep was three orders of
  magnitude too coarse, because each moved the answer by far more than the
  resolution.
- **Units are a diagnosis.** That same test produced thresholds scaling with ring
  *size* when a tooth-passing condition must depend mainly on the tooth
  *difference*. A result whose units are wrong is wrong however plausible.
- **Test the direction nothing else exercises.** Sixteen tests on the planetary
  power algebra, including two exact closed forms, and a backward efficiency of
  101.571 % survived all of them — because every one drove forward with a
  positive torque and a positive speed. It was found by looking at the UI.
- **`minimum_profile_shift` is a lower bound, not a recommendation.** It is −1.76
  at z = 43. The automatic value is `max(x_min, 0)`.
- **`Y_F` and the load point must move together.** The two helical corrections
  pull in opposite directions.
- **Hertz must be evaluated at both single-pair boundaries.** Checking only the
  inner one made one physical mesh give two answers depending on labelling.
- **"A helical mesh slides along its teeth."** It does not, with parallel axes.
- **For an internal pair, more centre distance is *deeper* engagement.** `a = r_r
  − r_p`, so separating the axes pushes the pinion toward the rim. Adding
  assembly clearance makes radial insertion harder, not easier.
- **Rack figures do not carry over to a pinion cutter.** 0.38 modules is the
  rack's tip round; on a 20-tooth cutter the tip is 0.377 mm wide.
- **A sweep must cover a whole engagement.** One circular pitch of travel is not
  enough.
- **Choose the coordinate the curves are well behaved in.** Comparing the ring's
  cut by angular gap at equal radius reported 18 µm of "error" where the fillet
  is stationary in radius.
- **Do not predict a threshold by hand when the computation exists to find it.**
  Assert the *comparison* — a shorter ring tooth clears more pinions — rather
  than the number.
- **A green local test run does not imply a green build.** `git add` before
  `nix build`; flakes only see tracked files.
- **Typechecking is not running unless you run it.** `cd web && npm run check`.
- **Run the app.** Two real defects this session were found on screen and by
  nothing else.

---

## 5. Open items

| Item | Where | Blocks |
|---|---|---|
| Radial assembly — **shelved**; attempted, diagnosed, withdrawn, findings kept | §4.11 | nothing |
| Equal planet load sharing is assumed, and said so in every result | §4.9 | nothing |
| The cut simulation cannot see below the generation limit: its cutter has no fillet | §4.11 | nothing |
| Mesh-phase coefficient setting the optimal λ | §4.10 | only the angular-profile-shift milestone |
| Tooth thickness tolerance (JGMA 1103-01, unavailable) | §4.6 | min/max on span and over-pins only |
| Worm contact ratio — the zone of action for a throated wheel | §4.5.1 | nothing |
| Span over teeth for a ring — rare in practice, not derived; between-pins is done | §4.6 | nothing |
| Worm profile drawing and DXF; a planetary set has no drawing either | §4.5.1, §8 | nothing |
| `Driven By` as a train direction on a worm stage | §4.9 | nothing |
| A coupled glass POM grade, if one is wanted back | §6.4 | nothing |

Known-approximate, documented at the call site rather than hidden:

- **`Y_β` omitted** — helical bending is conservative against a published ISO
  rating by up to ~25 %.
- **The axial compression term is omitted** from bending, internal and external
  alike, following ISO rather than AGMA. It relieves stress by order 10 %, so
  leaving it out is the conservative direction; do not compare to an AGMA `J`
  without saying so.
- **A ZN worm's contact stress is 1–15 % below the reported ZI figure.**
- **A ring's flank below its generation limit is not a generated involute** —
  about 0.08 mm on ordinary designs. Flagged per part.
- **Hardened 4340's fatigue allowable is the weakest number in the library.**

---

## 6. Next

### Milestone 11 — polish

Train import/export, confirmations, error surfacing, docs.

### After

| Item | Note |
|---|---|
| Angular profile shift | §4.10. Blocked on the mesh-phase coefficient; the acceptance gate for any replacement model is already written — it must first reproduce `j_t = 2a′(inv α′ − inv α_w)` |
| A planetary set's drawing | the viewport draws single gears; a set needs the carrier and N planets placed |

---

## 7. Working notes

- **Verify against something that shares no code.** The rack simulation, the
  pin-tangency measurement, `ezdxf`, the contact-half-width route, the AGM
  against Carlson's integrals, the numerical average of instantaneous loss, the
  parametric-surface curvature against the analytic one, the ring cut simulated
  from the cutter alone, a huge ring and a huge external gear rating as the same
  rack tooth, a planetary's play at two output shafts differing by exactly the
  ratio its kinematics computes separately. Every one of those caught something
  self-consistent tests had passed.
- **Ask what property the answer must have.** The ring's flank disagreement was
  located by noticing the simulated envelope was *not an involute of the ring's
  base circle* — a property conjugate action guarantees, checkable without
  knowing the answer. The planetary ring search's contiguity is the same kind of
  property: monotonicity makes a gap a bug by construction, and a gap is how the
  domain-boundary rounding was found.
- **Prefer laws to numbers.** "An internal mesh is less curved than the external
  pair of the same teeth", "a ring's tooth is the stronger", "friction never
  pays", "efficiency never exceeds one" — each is checkable without knowing the
  answer, and each has caught something.
- **Run the app.** `nix shell nixpkgs#chromium --command chromium --headless
  --no-sandbox --disable-gpu --virtual-time-budget=25000 --dump-dom
  http://localhost:5173`; both `--dump-dom` and `--screenshot` run the JS, so
  they prove the app mounted and computed. Temporarily defaulting a tab to the
  state under test is a fair way to reach a panel a click would otherwise be
  needed for — revert it afterwards.
- **Record corrections rather than editing them away.** DESIGN §12 exists because
  the pattern is more useful than any single entry: *the errors that survived
  longest were the ones that looked reasonable and were never checked against
  something independent.* Two entries there are now corrections **of
  corrections** — the second diagnosis was right about the symptom and wrong
  about the cause, and both times the wrong cause looked like caution.
