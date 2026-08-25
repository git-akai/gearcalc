# Handoff

Where the project stands, what is decided, and what to be careful of.

`docs/DESIGN.md` is the design of record and is current as of the head of
`main`; this file is the shorter route in. Where the two disagree, DESIGN.md
wins and this file is stale.

---

## 1. State

**Milestones 0–11 complete and in CI. 351 tests, ~26 s.**

Milestone 11's named scope — geartrain import/export, confirmations, error
surfacing, docs — is done, and geartrain import/export was the last unbuilt item
of the original specification. What ran past it is the crossed-axis work the
§4.5.1 audit set out, and that is finished too: the path of contact, the contact
ratio, contact rated along the path, a face width sized from `ε ≥ 1`, and one
friction balance containing both efficiency formulas. Crossed-axis **bending** is
the one thing that audit named and did not deliver; §5 says why, and it is not a
gap waiting to be filled.

```bash
nix develop                       # or `direnv allow` once
cargo nextest run                 # the suite
nix flake check                   # what CI runs: build, clippy --deny warnings, fmt, tests
cd web && npm run dev             # the application
```

| Crate | Holds |
|---|---|
| `gear-core` | all mathematics. `serde` is its only dependency, deliberately |
| `gear-io` | DXF writer, TOML material library and geartrain document |
| `gear-wasm` | fourteen entry points, JSON in / JSON out — including the UI's defaults, so they have one home |
| `web` | Svelte 5 + TypeScript. Layout and event handling only |

### What works

**Parallel-axis gearing.** Involute + trochoid profile, undercut, severed teeth,
validated against a rack simulation from both sides over 1080 cases · primitives
(safeguarded `inv⁻¹`, Brent, bracketed Newton) · mesh, centre distance, exact
backlash, contact path · metrology (span, over-pins, JGMA 116-02 tables) ·
strength (critical section, form factor, bending stress, Hertz, face width,
helical throughout) · efficiency · automatic profile shift and altered addendum.

**Crossed axes.** `screw.rs`, and it is one model rather than a family:

- *Geometry.* Lead angle `sin γ = z m_n/d` exact, elliptical contact, sliding as
  a vector — and **the path of contact**: the line tangent to both base cylinders
  whose direction two properties of an involute helicoid fix, with its zone,
  contact ratio and axial travel (§4.5.1). The parallel case is a **degeneracy**
  of it, not a value: at `Σ = 0` the line becomes a plane and contact spreads
  from a point to a line.
- *One friction balance.* `F = F_n(n̂ + μv̂)`, moments about each axis, averaged
  along the path. It **contains both** older formulas — the classical screw one
  at the pitch point to 1e-12 including its exact self-locking threshold, and the
  parallel-axis loss integral at `Σ → 0` to a hundredth of a point. Every stage's
  efficiency and self-locking friction now come from it, in both directions.
- *Rating.* Contact is taken along the path, at the two single-pair boundaries as
  well as the pitch point, with the flank load from the same place as the stress.
  The pitch point alone under-stated it by 0.7–2.2 %: the relative radius peaks
  near there, so it is close to the gentlest place on the path.
- *Entry.* A crossed gear pair is **a spur stage with an axis angle**, as the
  specification has it — `β₁ = Σ/2 + β_add`, `β₂ = Σ/2 − β_add`. Three stage
  kinds, not four. What still differs between a worm drive and a gear pair is one
  input: whether the first member's diameter is given or derived from a helix
  angle.
- *Sizing.* A crossed pair's face width is automatic from `ε ≥ 1` — a
  **geometric** minimum, since no stress there depends on the width. A worm keeps
  its published proportions (DIN/ČSN, BS 721) with the source on screen. Both are
  labelled with which kind of minimum they are, because they differ by 2.4× and
  answer different questions.

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
DXF with exact arcs for external *and* internal gears · **geartrains exported and
imported as TOML**, inputs only, import making a new tab · gear tabs with an
internal option and its own measurement, geartrain tabs with spur, worm and
planetary stages.

### Driving it without a browser

```bash
cargo run --bin gear-cli -- show 17 0.2            # one gear's derived geometry
cargo run --bin gear-cli -- materials              # the library, with each value's basis
cargo run --bin gear-cli -- strength 17 43 2.0     # a worked mesh, end to end
cargo run --bin gear-cli -- train                  # a two-stage train
cargo run --bin gear-cli -- train mixed            # ...with a worm stage in it
cargo run --bin gear-cli -- trainfile [path]       # a train to TOML and back, answers compared
cargo run --bin gear-cli -- worm 1 40 7 90         # a worm pair, both directions
cargo run --bin gear-cli -- wormstage 1 40 7 2     # a worm stage, end to end
cargo run --bin gear-cli -- crossed 17 23 90       # a crossed pair, swept over the split
cargo run --bin gear-cli -- planetary 17 17 3      # every ring count that can work
cargo run --bin gear-cli -- planetstage 24 18 60 3 # a planetary stage, six modes
cargo run --release --bin gear-cli -- verify 100   # the two-sided cutter check
python3 tools/worm_flank_curvature.py              # ZI vs ZN vs ZA, from the surface
python3 tools/crossed_path.py                      # the crossed path, from the surfaces
```

The last two share no code with the crate — that is their whole purpose. `crossed_path.py` builds both flanks as parametric surfaces and reaches the
line of action through differential geometry; the crate reaches it through a
construction in lines and angles. On a 17/23 pair at 45°/45°, shafts at 90°,
they give ε = 1.777921670 and 1.777921669562.

**The worm canary moved, once, deliberately.** `wormstage 1 40 7 2` went
68.691 → 68.430 % forward and 55.254 → 54.417 % backward when the friction
balance replaced the pitch-point formula (§4.5.1). Its old figures were the same
balance sampled at the one point on the path where the term now added is zero;
they were not more correct. Everything downstream followed: wheel torque
54.9531 → 54.7441 N·m, flank load 3510.8 → 3505.8 MPa.

`gear-cli strength 17 43 2.0` is the regression canary. Its numbers — `σ_F`
69.2 / 63.4 MPa, `σ_H` 692.7 MPa, ρ 1.723 mm, η 98.741 % — have survived every
refactor since milestone 5 **unchanged to the last digit**, including the whole
contact unification, the signed-mesh collapse and the bending-continuity fix, and
that check has caught more than the test suite has in that area.

---

## 2. The rules, and why they are rules

**No engineering calculation in TypeScript.** If a number appears in the UI, Rust
computed it. TypeScript formats. This is what keeps the Rust test suite
meaningful — otherwise logic migrates to where nothing tests it. **A default is
one of those numbers**: they come from `gear_wasm::defaults`, and the reason is
§12's — the one that was written down twice drifted, and only the side without
tests was wrong. A diameter is another: it is `2 × radius` in Rust, not here.

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

**Two automatic face widths, and they mean different things.** A spur stage's
inverts a stress; a crossed pair's is the width at which contact stays continuous
(`ε = 1`), because no stress there depends on the width at all. Both are shown as
minimums with the kind named — 1.96 mm from continuity against 4.69 mm from a
worm proportion is not a contradiction, it is two questions.

**The policy is about factors that multiply a stress, not about conventions as
such.** A worm's length and a wormwheel's face width are shipped as
recommendations from DIN/ČSN and BS 721, with their sources on screen — because
neither enters any stress here: a point contact's peak pressure is bit-identical
at 4 mm and 40 mm of face width. A convention that cannot move an answer informs
a choice; one that multiplies a stress silently moves a number a part is sized
against. That is the line, and DESIGN §4.5.1 states it where the formulas are.

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

**Efficiency is one force balance, not a formula per axis angle.**
`contact::Contact` — a point, a normal, two axes — resolves `F = F_n(±n̂ + μv̂)`
into a moment about each shaft. It *contains* both formulas it replaced: the
classical screw one at the pitch point to 1e-12, its self-locking threshold
exactly, and the parallel-axis loss integral as `Σ → 0` to a hundredth of a
point. The residual at that limit is the parallel formula's own `O(μ²)`
linearisation, identified by watching the gap fall linearly with `μ` (0.0153 →
0.00047), not a defect in the balance. Crossed efficiency is now the average
along the real path of contact rather than a pitch-point value.

**A crossed gear pair is a spur stage with an axis angle.** `β₁ = Σ/2 + β_add`,
`β₂ = Σ/2 − β_add`, so the parallel pair is that angle's zero and one section
serves both. The stage translates itself into the screw pair it is. This is the
specification's own arrangement, and it is the "find the parameter" rule applied
to a whole tab rather than to a formula.

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
- **An input that moves no number needs saying so — and the fix is not always to
  hide it.** A ring's profile shift box was connected to nothing for a whole
  milestone (§12). A crossed pair looks like the same case: it is solved at its
  pitch point, so shift, addendum, dedendum and root radius reach none of its
  figures. But they are not meaningless — they are the tooth that will be
  **cut**, and a designer specifying a crossed pair is specifying those parts.
  So they are offered, and the panel says once, plainly, what they do and do not
  reach. The fault is the silence, not the field. When merging two forms, check
  every field against what the solve reads, then decide whether it describes
  something real anyway.
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
- **A gate on a *ratio* cannot see a scale error.** `moment_per_force` returned a
  power where a torque was wanted, putting a factor `z₂/z₁` — 40× on a worm — into
  every flank load, and 3.42× into a stress through the cube root. All four
  efficiency tests stayed green, because efficiency is a ratio and the factor
  cancelled in it. If every gate on a quantity divides it by something, none of
  them constrains its size: add one that asserts an absolute value against a
  formula from outside.
- **A disclosure can expire, and a stale one is a false statement.** "This
  efficiency omits profile sliding" was true of the old model and became wrong of
  the new one, which integrates the sliding along the path — it then fired at
  `Σ = 0.05°` on the *more* exact number. Deleted, not reworded. When a note
  describes a limitation, it belongs to the code that has the limitation; check
  the notes when you replace the model.
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

Nothing here blocks anything. Two are **decisions with reasons**, not gaps: they
would need a convention this project refuses, and the reason is on screen where
the number would have been.

| Item | Where | Note |
|---|---|---|
| **Crossed-axis bending** | §4.5.1 | *Decided, not pending.* The path gives the load's position along the profile; `σ_F = F_t/(b·m)·Y_F·Y_S` is a cantilever loaded across its whole **face**, and a crossed pair's load is a point. An effective width is a convention that multiplies a stress, which §4.7 refuses — so the stage says it is not rated and why |
| Equal planet load sharing is assumed | §4.9 | *Decided.* The remedy is a mesh-load factor of the kind §4.7 declines; said in every planetary result's notes |
| Radial assembly — attempted, diagnosed, **shelved** with its findings | §4.11 | |
| The **enveloping** (throated) wheel's zone of action | §4.5.1 | The cylindrical one is derived; a worm reports it as a floor, with its assumed tooth height named |
| Mesh-phase coefficient setting the optimal λ | §4.10 | The only thing blocking the angular-profile-shift milestone |
| Tooth thickness tolerance (JGMA 1103-01, unavailable) | §4.6 | min/max on span and over-pins only |
| The cut simulation cannot see below the generation limit: its cutter has no fillet | §4.11 | 0.08 mm on ordinary designs, flagged per part |
| Span over teeth for a ring | §4.6 | Rare in practice, not derived; between-pins is done and the tab says which is which |
| Worm profile drawing and DXF; a planetary set has no drawing either | §4.5.1, §8 | |
| A ring's own bounds are not reported for a stage member | §4.11 | The gear card shows a rack's buildable range, which is not a ring's — so it shows nothing there and says so |
| `Driven By` as a train direction on a worm stage | §4.9 | |
| A coupled glass POM grade, if one is wanted back | §6.4 | |

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
- **A note slot is as tall as the tallest note that field can show *now*** (§8.0).
  Every real message fits: the bound notes are the tall ones and validation
  messages are short. A validation message longer than its field's bound note
  would still move the controls when it appeared.

---

## 6. Next

Every milestone through 11 is met, so this is no longer a queue with a head. What
is below is what a next session would pick from, not what it owes.

### Milestone 11 — done

The named scope: the ring-drawing defect and the default that caused it (§4
traps), the ring's rim circle, the worm/wormwheel recommended proportions,
geartrain import/export, and the UI corrections.

**Train import/export** was the last unbuilt item of the original specification:
`gear_io::train` (`{ name, train }`, inputs only, with a header comment),
`import_train`/`export_train` at the boundary, Export and Import on the geartrain
tab, and `gear-cli trainfile`. The round trip is checked as *answers* rather than
as bytes, in the CLI, across the boundary and in the browser. The silent
`saveDxf` failure is fixed too.

**UI.** The geartrain tab's three stage sections are one visual language — the
planetary section was rebuilt on the **same gear card** the spur stage uses, so a
sun, a planet, a ring and a spur gear are one definition rather than several that
drift; the four reference circles read as **diameters** (computed in Rust,
because doubling is arithmetic); the viewport **zooms about the cursor**; every
note sits in a slot holding all the notes that field could show, so the controls
do not move when one appears; inputs are anchored on the box and notes are
right-justified to its edge. See DESIGN §8.0 — and note it was verified by
measuring `getBoundingClientRect()`, because screenshots here are not
pixel-deterministic.

### The crossed-axis work — done, and past its brief

The §4.5.1 audit asked where the code branched on line-versus-point contact and
found five places. All five are one model now (§1, "Crossed axes"), and the
efficiency branch went with them: one force balance, verified to contain both
formulas it replaced. The audit's own bending prediction did not survive contact
with §4.7 — see §5, which records that as a decision rather than a gap, and §12
of DESIGN.md, which records that the audit was wrong to promise it.

### After

| Item | Note |
|---|---|
| Angular profile shift | §4.10. Blocked on the mesh-phase coefficient; the acceptance gate for any replacement model is already written — it must first reproduce `j_t = 2a′(inv α′ − inv α_w)` |
| A planetary set's drawing | the viewport draws single gears; a set needs the carrier and N planets placed |
| A worm's profile drawing, and its DXF | §8. A crossed pair draws as its two helical gears already |
| The enveloping wheel's zone of action | would turn a worm's `ε` from a floor into the number |
| Further UI work | as it is asked for |

---

## 7. Working notes

- **Prefer a law to a threshold when disclosing a model's limit.** The crossed
  efficiency is short by an amount that grows as the shaft angle falls. Warning
  "below 5°" would have been a convention; the test used instead is that
  crossing shafts can only *add* sliding, so a crossed pair beating the same
  teeth run parallel is the model admitting it lost some. No number chosen, and
  it stays right if the model changes.
- **Having the load's position is not having the load.** The audit said the
  contact path would unblock crossed-axis bending. It did not: the path gives
  where on the *profile* the load acts, and the beam formula needs how it is
  spread across the *face*. A point load on a wide tooth is a plate problem, and
  picking an effective width is exactly the sort of convention that moves a
  stress. Two missing ingredients looked like one until the formula was read.
- **A phrase describing the real part is not a modelled property.** "The wheel
  is throated" was true of worm drives and false of this crate — nothing here
  throats anything — and it got used as the reason to withhold a number that the
  cylindrical model produces perfectly well, while every other number in the same
  result came from that model unquestioned. Inherited justifications need
  re-reading against the code each time they are leaned on; this one had been
  written when nothing at all was derived, which made it true for a reason that
  had since expired.
- **Audit a unification claim against the code, not the intention.** "One model
  for both" decays quietly. The crossed-axis audit (DESIGN §4.5.1) found the
  contact model genuinely unified — `PARALLEL_AXES` is a value, and the crossed
  curvatures return exactly zero at `Σ = 0` — and the whole divergence one level
  up, in a single missing object: `ContactPath` is built in a common transverse
  plane, which crossed axes do not have. Five apparently separate gaps turned out
  to be five consumers of it.
- **Verify against something that shares no code.** The rack simulation, the
  pin-tangency measurement, `ezdxf`, the contact-half-width route, the AGM
  against Carlson's integrals, the numerical average of instantaneous loss, the
  parametric-surface curvature against the analytic one, the crossed path of
  contact differentiated off the flanks rather than constructed, the ring cut simulated
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
- **A scratch verification is not a verification once the directory is gone.**
  The crossed-axis derivation was checked in numpy before a line of Rust was
  written, and those scripts lived in a temporary directory while DESIGN.md
  cited their results as settled. They are `tools/crossed_path.py` now. Promoting
  them found two faults in the checks themselves — a bisection that converged on
  the wrong branch of an `atan2` wrap, and a comparison against only one of a
  tooth's two flanks — neither of which affected the crate, and both of which
  would have been invisible for as long as the scripts were unrunnable.
- **Record corrections rather than editing them away.** DESIGN §12 exists because
  the pattern is more useful than any single entry: *the errors that survived
  longest were the ones that looked reasonable and were never checked against
  something independent.* Two entries there are now corrections **of
  corrections** — the second diagnosis was right about the symptom and wrong
  about the cause, and both times the wrong cause looked like caution.
