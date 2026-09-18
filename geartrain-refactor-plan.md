# Geartrain Model Refactor — Plan

**What this is.** A worked plan for replacing the geartrain's kinematic model
with one graph and one solver, so that multiple inputs and outputs, compound and
meshed-planet epicyclics, and trains with mobility above 1 are *values* of the
model rather than kinds added beside it.

It answers `geartrain-refactor-handoff.md`, which is design-only and written
without sight of this repository. Where the handoff is right this says so and
takes it; where it is wrong for this codebase this says why, and what to take
instead. Both documents are working notes and are deleted once built, per the
precedent in `README.md`.

**The claim in one sentence.** This crate already computes the handoff's mesh
constraint three times — in `planetary::power`, in the hula stage's mapping onto
it, and in the train's ratio walk — and the refactor is to compute it once, over
a graph the stage kinds declare, and to read speeds, torques, backlash and tooth
cycles off the same matrix.

---

## 1. What is here today, measured

`crates/gear-core` is 27 modules. The geartrain lives in five of them:

| File | Lines | What it holds |
|---|---|---|
| `train/mod.rs` | 8499 | the shared vocabulary, relief, the load cases, `solve_train` |
| `train/pair.rs` | 1441 | two gears on shafts at any angle |
| `train/planetary.rs` | 2711 | sun, planet, ring, carrier |
| `train/hula.rs` | 3055 | four gears, two meshes, one crank |
| `train/crossed.rs` | 2601 | the crossed-axis mesh of a pair |
| `planetary.rs` | 1557 | the layout solve and Willis/Pennestrì kinematics |
| `hula.rs` | 861 | the hula arrangement's own kinematics and layout |

605 tests pass at `293924f`, in 28 s.

### 1.1 The three places the same kinematics is written

**`planetary::power`** takes a basic ratio, an `Arrangement { input, fixed }` and
three role indices, and solves Willis plus torque equilibrium with `η₀^w`. Its
own doc comment already makes the generalising claim:

> Sun, carrier and ring name the three *roles*… So any three-shaft epicyclic can
> be put through it by naming its own members in those roles — which is why the
> ratio arrives as a number rather than as a set of planetary tooth counts.

**`train/hula.rs` takes it up on that**, literally:

```rust
const GROUNDED: PlanetaryShaft = PlanetaryShaft::Sun;
const OUTPUT:   PlanetaryShaft = PlanetaryShaft::Ring;
const CRANK:    PlanetaryShaft = PlanetaryShaft::Carrier;
```

Four gears and two internal meshes, folded into a three-shaft basic ratio
`z₂z₄ / (z₂z₄ − z₁z₃)`. It works. It is also a role-mapping written by hand for
one arrangement, and there is no second one it would serve.

**`solve_train` writes it a third time** as a chain: `turns_per_port_turn` is
`Π i` forward and backward, `carry` walks the ratios, and the backlash referral
divides by everything downstream. Those are the graph solution for the special
case of a path, hand-derived.

### 1.2 The ceiling, in the project's own words

`docs/state.md`, **Not built**:

> **A third port** — A train has a start and an end; a load case names one. A
> stage kind with a third shaft a load could enter by would add a value to
> `Port`, and nothing else knows a direction.

`docs/rationale.md`, **Each stage kind keeps its own result type**:

> The test of the division is what a new kind would cost. It should be new
> *kinematics* and no new rating machinery at all… **That claim is untested
> until something tests it.**

This refactor is that test. It is also the answer to the third port.

### 1.3 Four flaws found on the way in

All are things to fix, not things to work around.

**(a) A geometric refusal kills the kinematics.** `solve_train` returns
`Err(TrainError::InStage { .. })` the moment any stage fails, and the whole
train then has no ratio, no efficiency, no backlash — including for the stages
that solved. But a ratio needs tooth counts and topology and nothing else. A
Wolfrom mid-sizing is *normally* a train whose centre distances do not yet
close, and the handoff is right that refusing to report its ratio would be the
single most obstructive behaviour available. The fix is not a flag; it is that
the kinematic solve does not depend on the geometric one, which the split
below makes structural.

**(b) `Arrangement` is on the wrong side of the boundary.** Which shaft is held
is a fact about how the *train* is wired, not about the set's geometry — a
planetary set with its ring grounded and one with its ring coupled to the next
stage's sun are the same set. Today it is a stage input, which is why a
planetary's ratio is a single number and why two epicyclic stages cannot share
a shaft. `docs/rationale.md#a-planetary-needs-the-held-shaft-named` argues
correctly that the shaft must be *named* — it is the level it is named at that
was wrong.

**(c) `TrainError::NoContact` carries three different meanings**, and says the
wrong one twice. Found by Phase 0's `unclosed` fixture, which reports *"the
teeth never come into contact"* for a set whose teeth are fine and whose two
centre distances simply cannot be brought together. The three sites:

| `train/planetary.rs` | what actually happened | what it says |
|---|---|---|
| `planetary::solve(…).ok_or(NoContact)` | no planet shift closes the set | the teeth never touch |
| `planetary::power(…).ok_or(NoContact)` ×2 | no self-consistent power flow — self-locking, or a shaft named as the input that is not driving | the teeth never touch |
| `ContactPath::new(…).ok_or(NoContact)` ×2 | the teeth genuinely never touch | correct |

This is a *user-visible* wrong sentence, since the variant renders through
`error.train_no_contact` in five catalogues. **Split in Phase 3b** into
`NoCommonDistance` and `NoPowerFlow`, each with its own key and five messages.

Two things the split turned up, both worth more than the wording:

- **`NoCommonDistance` is now fired from the model** in `gear_io::strings`'s
  sweep, where `NoContact` beside it is still *constructed by hand* — which
  that block's own comment says is the weaker standard, since it checks that a
  key has a message and not that the case is live.
- **`NoPowerFlow` cannot be fired at all**, and saying so took a measurement. A
  set asks `planetary::power` twice and only the forward call is a `?`; the
  backward one treats a set that cannot be back-driven as an *answer*, which is
  what self-locking is. A genuinely driving input refused **never**, across 1.1
  million combinations. So it is an `UNFIRED` exemption with its evidence — and
  with a standing test, `a_driving_input_always_has_a_flow`, that fails if the
  absence stops being true and that also asserts the two refusals which *are*
  reachable, so it is a statement about driving inputs rather than about
  `power` never saying no.

A third variant, `Wiring`, was drafted and **withdrawn**: it had no producer
yet, and a note nothing can fire is a thing this project adds with evidence
rather than in advance. It returns with the call site that needs it.

**(d) `StageResult::ratio()` means two different things.** A pair's is
`Mesh::ratio`, whose own doc says *"ignoring sign"*; an epicyclic set's is
`PlanetaryResult::ratio`, whose own doc says *"negative when the output
reverses"*. So a spur stage cannot report that its output turns backwards —
which every external pair's does — while a set can, and `TrainResult::total_ratio`
is the product of the two meanings. `docs/reference.md#trains` states both halves
(*"per stage `i = z_out/z_in`… and a planetary's comes from its own
kinematics"*) without noticing they disagree.

It is already visible in the corpus: `kinematics.txt` shows a pair delivering
`+4.9751 Nm` and a set `−11.6755 Nm`, and it is not only a readout — `carry()`
divides by `ratio()`, so the sign reaches every reported torque downstream of an
epicyclic stage and none downstream of a pair.

**It is also two live bugs, found and fixed in Phase 3b**, and neither was
reachable from a shipped fixture because no shipped train puts a reversing
stage next to another:

- **a stage after a reversing one could not be solved at all.** The referral
  handed it a negative torque, and a negative tangential force has no Hertzian
  contact to press at any face width — so it refused with *"the teeth never
  come into contact"*, which is true of nothing. A planetary set with its
  carrier held (`i = −6`) could not be followed by any stage.
- **a backlash came back 23.5 % light.** Play does not cancel, and a reversing
  stage downstream made an upstream stage's contribution *subtract*: 0.0422°
  reported where the two stages have 0.0552°. That train solved happily and
  simply under-reported, which is the worse of the two failure modes.

The resolution separates what the accessor was conflating. **A referral is a
magnitude; the direction of rotation is the motion's.** `carry` and the
backlash walk take `.abs()` with the reason at the site, and the *ratio* itself
becomes the signed kinematic fact the graph gives. Which convention a **readout**
should use — a signed ratio, or a magnitude with the direction stated beside it
— is a UI question for Phase 4, and a different one from what the model holds.

---

## 2. The finding this is built on: one matrix, four readings

Write every mesh in the frame of the member carrying its axes. With **signed**
tooth counts — a ring is a gear with a negative tooth count, which is already
`MeshKind::sign()` and already this crate's standing convention:

```text
z_i (ω_i − ω_f) + z_j (ω_j − ω_f) = 0
```

Assemble one row per mesh and one per rigid coupling over the vector of shaft
speeds. Call it `A`. Then:

| Reading | What it gives | What it replaces |
|---|---|---|
| `A ω = 0`, nullspace | speeds, ratios, mobility | `planetary::power`'s Willis, `Stage::ratio`, `turns_per_port_turn` |
| `τ ∈ rowspace(A)` | torques, reactions, power split | `planetary::power`'s equilibrium, `carry`, the pair's projection |
| `A θ = Δ` | backlash at any shaft | the chain referral, the set's `z_s(θ_s−θ_c) + z_r(θ_r−θ_c) = Δ` |
| `\|ω_m − ω_f\| / \|ω_ref\| × N` | tooth cycles | `engagements`, already written this way |

The third is worth dwelling on, because the handoff misses it. `docs/reference.md#planetary-sets`
already derives a set's backlash as `z_s(θ_s − θ_c) + z_r(θ_r − θ_c) = Δ` — the
same system with the planet eliminated and a non-zero right-hand side;
`docs/reference.md#trains` derives the train's accumulation as that system
solved for a chain. They are the same matrix with a load on it.
Four things that are written four ways today are four readings of one object,
and that — not tidiness — is the argument for the refactor. Every one of those
four is a place two answers can silently disagree, and the corrections log is
largely a record of exactly that happening.

**One free assertion, and it is worth less than the handoff claims.** The
all-ones vector satisfies every mesh row and every coupling row: a train locked
solid turns as one body. The handoff offers this as the check that localises a
sign or frame bug, and this document repeated it.

**It does not catch either.** Measured in Phase 1, by the method `CLAUDE.md`
prescribes — flip a ring's tooth count to positive, run the suite, see what
fires. The lock-up test passed; only the cross-check against `planetary::power`
failed. The reason is that the row constructor derives the frame's coefficient
from the counts it was handed, so the sum telescopes to zero whatever they are:
*a check built from the thing under test measures nothing*, which is a pattern
`docs/corrections.md` already names, met in the first check the new module
offered.

What it is worth, stated: it catches a row assembled anywhere but the
constructors, and a frame coefficient written by hand — the classic
transposition — and nothing else. The gate for a wrong **sign** or a
misattributed **frame** is Phase 2's cross-check against the model being
replaced, and the independent Python check beside it. Both are kept; neither is
described as doing the other's job.

---

## 3. The handoff, judged

### Taken

- **The frame-relative mesh row.** Correct, and it is the whole refactor.
- **Sun / ring / planet are not kinds.** Correct kinematically.
- **`frame` stored resolved rather than inferred at each use.** Correct, and it
  matches this crate's habit of storing the solved answer (`planetary::Layout`).
- **Mobility, and under-determined as an answer.** Correct, and it is the piece
  that makes differentials expressible at all.
- **Multi-input and multi-output are one kinematic object.** Correct, and it
  deletes a question the UI should never have asked.
- **The torque layer is the transpose of the matrix already assembled.**
  Correct, and cheap.
- **Legacy archetypes become test fixtures.** Correct, and in-idiom: this crate
  already keeps `Tooth::with_flank_clamped_at_base` as a negative fixture for
  exactly this reason.
- **Closure must never gate the ratio.** Correct, and §1.3(a) is the live fault.

### Rejected, with reasons

- **A stored `sigma` per mesh.** This crate already carries the sign as
  `MeshKind::sign()`, documented as "the whole of the difference between the two
  kinds", and carrying it a second time on the netlist is one idea written down
  twice — the first pattern in `docs/corrections.md`. A mesh's sign is a
  property of the mesh's own geometry and is *asked* of it. Where the handoff is
  right is that bevel and face pairs break the `internal ⇒ −1` default; the
  answer there is a third value of `MeshKind`, not a field beside it.
- **BigInt rationals.** Right instinct, wrong size. Tooth counts are `u32` and
  the products a ratio needs are a handful of them; `i128` holds any train
  anyone will build, and overflow is a checked refusal rather than an allocation
  strategy. The precedent is in the crate: `hula::Ratio` is already an exact
  integer pair, kept so "because it *is* integers".
- **"There is no such thing as a sun, a ring, or a planet."** Half right. It is
  true of the kinematics and false of the geometry: a central member's axis is
  the carrier's axis and a planet's is offset from it, and *that distinction is
  what the closure relation is written on*. The handoff's own primitives carry
  it as `carriedBy`; its prose overstates, and taking the prose literally would
  delete the one structural fact the geometry layer needs.
- **"Geometric closure is an advisory."** In a ratio explorer, yes. Here,
  closure is **solved**: the planet's profile shift is the free variable that
  makes two centre distances agree, and demoting that to a warning would throw
  away the tool's core value. Both are true at once — closure is solved where
  something can absorb it, reported where nothing can, and never a gate on the
  ratio.
- **A flat netlist replacing stages.** See §5.3. The stage is where the
  *assembly* relations live, and they are genuinely per-arrangement.

### Missed

- **Losses.** The handoff ships the ideal torque layer and gates the lossy one
  as "meaningfully harder". This crate already solves the lossy case for three
  shafts, with a two-branch search on the sign of the rolling power and two
  consistency conditions, and self-locking as a refusal. The generalisation —
  a power-flow direction per **mesh** rather than one per set — is the hardest
  single piece of this work and is named as such in §5.6.
- **Backlash as a reading of the same matrix** (§2).
- **Tooth cycles.** `engagements` is already carrier-relative and already the
  graph's answer; the handoff does not mention cycles at all, and they are half
  of what this tool is for.
- **That the hula stage is already a compound-planet epicyclic** (§5.4).

---

## 4. The model

### 4.1 The kinematic layer — a new module that knows nothing about gears

`gear-core/src/kinematics.rs`, in the discipline of `solve.rs` and `hertz.rs`:
no gears, no teeth, no geometry, no stages.

```rust
/// A body with one angular velocity. Ground is one of these.
struct Shaft(usize);

/// One row: two signed tooth counts and the frame they are seen from.
struct MeshRow { a: Shaft, b: Shaft, za: i64, zb: i64, frame: Shaft }

/// Rigid identification of two shafts. A coupling, a locked clutch and
/// grounding are one mechanism.
struct Coupling(Shaft, Shaft);

/// What is asked of a shaft, and there are only three things.
enum Condition { Ground, Drive(Rational), Free }
```

**Ground is a frame like any other, which happens to be held.** Not a different
kind of thing: it is a shaft, it appears in mesh rows as a frame, it carries
torque, and what makes it ground is the condition on it. Shaft zero is
bookkeeping — every stage needs the *same* one — not physics.

**There is no housing.** An element is fixed to ground, carries a load, or is
attached to another element. Calling the reference a housing invites two wrong
readings and both bite in the arrangements this refactor exists for: that the
reference is a component with an interface to size, and that a **frame** must
stand still. A frame is whatever shaft carries a mesh's axes, and in a Wolfrom
that is a carrier — turning, and shared by several meshes at once.
`LoadCase::reacted` is the same idea under an older name: *reacted at the far
end* is the far port held, and §10.4's question is whether the word buys
anything the condition does not.

Two readings follow that are easy to conflate, so they are named apart: **a held
shaft's reaction** is the entry on that shaft, and **what is taken to ground**
is the sum over the held shafts. A holding constraint is external, so its
reaction sits on the shaft it holds; ground itself carries only what something
*meshes* against it, which in a pure epicyclic is nothing at all.

The crate does use "housing" elsewhere, for the real part whose bore centres set
a centre distance — a worm's wheel absorbing a housing distance by its shift.
That is a different thing with a different job, and the two are named apart on
purpose. `tools/check_units.py` exists because a name meaning two things in two
modules is how a bug is made; this is the same discipline applied to a noun.

and over them:

- `assemble(...) -> System` — the matrix, with the lock-up invariant asserted.
- `System::mobility() -> usize` — rank over the connected component, with
  shafts that no constraint touches **named** rather than counted (the handoff
  is right that they inflate `m` silently).
- `System::speeds(&[Condition]) -> Solution` — a single answer at `k = m`, an
  affine family at `k < m`, and a named conflict at `k > m`.
- `System::torques(&Solution) -> …` — the transpose solve, ideal.
- `System::displace(&[Δ]) -> …` — the backlash reading.

Rank and elimination are fraction-free (Bareiss) over `Rational`, so there is no
tolerance anywhere in the kinematics and no "a bound records where the sweep
stopped".

`gear-core/src/ratio.rs` holds the exact rational: `i128` numerator and
denominator, normalised, with checked arithmetic.

*(An earlier draft said `hula::Ratio` would be deleted into it as "the same
type with a narrower name". It is not the same type: its denominator `D` is
**un-normalised on purpose**, because `|D|` is the hula design rule and a
normalised `324/1` would lose the `D = 4` that a `1296/4` carries. What the two
share is a value, and that is checked — `a_hula_arrangement_reduces_by_its_two_products`
— rather than merged.)*

### 4.2 Exact where the answer is integers, floating where it is not

A clean seam, stated once:

| Exact | Floating |
|---|---|
| speeds, ratios, mobility, rank | every stress, every length, every angle |
| the ideal torque split | the lossy torque split (`η` is not rational) |
| backlash *coefficients* | backlash *values* (`Δ` is millimetres) |

This is not gold-plating. It is what makes `D = 0` a refusal rather than a large
number, what makes "these two nodes are coincident" a fact rather than a
comparison against an epsilon, and what makes a recogniser's structural
comparison exact. The crate already took this decision once, for the hula
ratio, for these reasons.

### 4.3 Stages declare their wiring

The stage kinds stay. What leaves them is the kinematics.

`Constrained` gains a fifth question — or, better, a sibling trait `Wired`, so
a kind that is *only* geometry is not forced to answer:

```rust
trait Wired {
    /// The shafts this stage introduces, in a stable order, with a label key.
    fn shafts(&self) -> Vec<ShaftSpec>;
    /// Which shaft each member spins with, and which shaft carries its axis.
    fn mounts(&self) -> Vec<Mount>;
    /// Every mesh, by member index, with the frame it is seen from and how
    /// many parallel instances of it the stage has.
    fn meshes(&self) -> Vec<MeshSpec>;
    /// The shafts a train may couple to, with the names a designer knows.
    fn ports(&self) -> Vec<PortSpec>;
}
```

Filled in:

| Kind | Shafts | Meshes |
|---|---|---|
| `Spur` / `Worm` | two, both carried by ground | one, framed on ground |
| `Planetary` | sun, carrier, ring, planet | (sun,planet) and (planet,ring), framed on carrier, `N` instances |
| `Hula` | gear-1, crank, wobble, gear-4 | (1,2) and (3,4), framed on crank, 1 instance |

The table above is the whole of §1.1's three hand-written mappings, as data.

**This is where the rationale's untested claim gets tested.** A new kind costs
`Wired` and its own assembly relations, and nothing in `MemberRating`,
`MeshReport`, `Bending`, `Loading` or `GearResult` moves — those are already
keyed on members and meshes rather than on named roles.

### 4.4 The general epicyclic stage

**The finding.** A simple planetary set, a Wolfrom, a compound-planet
differential and the hula stage obey *one* geometric law:

> Every mesh between a central member and a gear on planet shaft *P* runs at
> the same centre distance — the carrier radius of *P*.

`train/planetary.rs` calls that distance `centre_distance` and solves the
planet's shift to reach it. `hula.rs` calls it `offset` and says, in its own
words, "the offset is the stage's own quantity rather than either mesh's…the
one distance both meshes run at". They are already the same `Freedom::CentreDistance`
in the relief machinery. They are the same law, written twice.

So the general epicyclic stage is the handoff's mesh grid, with that law as its
closure:

- **central members** — gears coaxial with the carrier, each on its own shaft;
- **planet shafts** — each at one carrier radius, each carrying one or more
  gears (a compound planet), each replicated `N` times about the axis;
- **meshes** — central↔planet, and planet↔planet where the gutter says so;
- **closure** — one radius per planet shaft, absorbed by whichever shift is left
  automatic, exactly as `planetary::solve` does today for the three-member case;
- **layout** — equal spacing, simultaneous meshing, planet-to-planet clearance,
  every one of which is already written and already conditional on `N > 1`.

The nine Wolfrom variants, Ravigneaux, Simpson, stepped and double-planet sets
and meshed-planet differentials are tick patterns in that grid. `Stage::Planetary`
becomes a preset that lays out one central pair and one single-gear planet
shaft.

**And the hula stage is a tick pattern too**: two central members, one compound
planet shaft carrying two gears, `N = 1`, both meshes internal. What is
genuinely its own is not the kinematics and not the closure — it is the
*sizing strategy*: the crank offset is solved from the far-side tip gap, because
at one tooth of difference two tip circles overlap away from the mesh. That is a
mesh-level question (`TipRoom`, which `MeshTrial::tips_are_clear` already asks of
every internal mesh) driving a stage-level solve. It survives as a strategy for
choosing the carrier radius, not as a stage kind.

**It keeps its vocabulary, and the vocabulary is gated by the arrangement.**
Per §10.2, the far-side clearance, the offset stated as a crank distance and
the words a designer of one of these uses all survive as inputs of the
epicyclic stage — *shown where they mean something*. The gate is asked of the
arrangement, never of a kind tag: a stage with one planet and a one-tooth
internal mesh is offered a far-side clearance because that is what such a stage
has, not because somebody ticked "hula". This is the worm's treatment, where
the kind decides a preset, a vocabulary and which inputs are put in front of a
designer, and the core holds one model.

**And the gate is per feature, not one bundle**, because the features do not
share a condition:

| feature | the arrangement that earns it |
|---|---|
| far-side tip clearance, and the carrier radius solved from it | **any** internal mesh whose tip circles cross — which `TipRoom` already computes and `MeshTrial::tips_are_clear` already asks |
| eccentric / crank vocabulary for the carrier | one planet, `N = 1` |
| equal spacing, simultaneous meshing, planet-to-planet clearance | more than one planet |
| the closure solve absorbing into a shift | every planet shaft with more than one mesh |

**These are not the hula stage's features and never were.** They are the
small-tooth-difference internal mesh's, and the thing that wants them next is
the very thing this refactor is for: a **Wolfrom's** second ring commonly runs
at one or two teeth of difference, which is exactly where two tip circles
overlap away from the mesh and where the carrier radius has to answer to the
tip bound rather than to the shifts. A planocentric reducer — sun, one planet,
ring, at a tooth or two of difference — is the same facility again with a
different tick pattern, and is not implemented today at all. Reading them as
one stage kind's quirks is what has kept them out of every other arrangement
that needs them.

**So unification is worth a change in current behaviour**, per §10.2's own
terms. Two follow directly and both are improvements:

- an epicyclic set gains the option of taking its carrier radius from the tip
  bound, which today only a hula stage can do;
- a hula-shaped stage gains the shift-absorbed closure, which today only a set
  can do.

Each is a feature reaching an arrangement that could always have used it. Where
one moves a shipped number, §10.3's rule applies: a reason, and a check that is
not the diff.

### 4.5 Ports, conditions and mobility

**A port becomes a named shaft** rather than one of two positions. `Port::Start`
and `Port::End` survive as the two ports a chain train auto-names, so nothing a
designer has moves; a train of one Wolfrom stage has four.

**The train grows two lists** beside its stages:

```rust
pub struct Train {
    pub stages: Vec<Stage>,
    /// Rigid couplings between ports. The chain is the default: port
    /// `end` of stage k to port `start` of stage k+1.
    pub couplings: Vec<Coupling>,
    /// Ground / drive / free, per port. `Arrangement` moves here.
    pub conditions: Vec<PortCondition>,
    pub load_cases: Vec<LoadCase>,
    pub reversed_bending: bool,
}
```

**Mobility is reported, and relief already knows how to handle it.**
`FreedomGroup { given_at_most, automatic_at_most, order }` and the `settle` walk
above the stage kinds are *exactly* the machinery for "`m` of these may be
given, the first one in relief order that you are not this moment touching gives
way". Applied to conditions instead of geometry inputs, with `given_at_most = m`,
it reuses a walk that is already tested for idempotence and for
order-independence. That reuse is the reason the conditions layer is cheap.

The three outcomes:

| | |
|---|---|
| `k = m` | one answer — today's case, unchanged |
| `k < m` | the affine family: every shaft's speed as `Σ aᵢ ωᵢ` in exact rationals, presented, not refused |
| `k > m` | redundant (consistent) or conflicting; both named, and relief offers the resolution |

### 4.6 Loads, torque and loss

**Ideal first.** `τ ∈ rowspace(A)`, free shafts contributing `τ = 0` rows. That
gives the reaction at ground, the split at a power-split node, and — by taking
`τᵢωᵢ` per shaft and per mesh — circulating power, which today is only visible
as the hula stage's 27 % against its meshes' 99 %.

**Then loss, and this is the hard part.** Today: one sign `w` per *set*, with
`η₀` the product of the meshes, two candidates tried, two conditions deciding —
the assumed sign must be the produced sign, and the output must absorb what the
input delivers. Both conditions are needed and the reference records why the
first alone is not enough.

The generalisation is a sign per **mesh**: power crosses each mesh one way or
the other, and each mesh charges its own `η` in that direction. For `M` meshes
that is `2^M` assignments, filtered by the same two conditions generalised
(each mesh's assumed direction is the direction the solution produces; total
delivered power is absorbed). For a simple planetary, `M = 2` and the
assignments where both meshes carry power the same way reproduce today's `η₀^w`
exactly — which is the thing to prove before anything else is believed.

Three things follow and each must be stated rather than assumed:

- **self-locking is "no consistent assignment"**, which is what
  `planetary::power` returning `None` already means;
- **`LoadCase::reacted` becomes derivable, and stays an input anyway.**
  "Nothing reacted it, so the train turns under the load" is, in this
  formulation, "no `τ` in the rowspace puts that load on that shaft" — a law
  where there is now a flag. Per §10.4 the law is what the solve uses and the
  flag is kept as vocabulary: whether a designer loses anything by its removal
  is a usability question, and it cannot be answered until Phase 4's panel
  exists to compare them on screen;
- **the walk that stops at the first locked stage** (`carry`) becomes "the
  consistent assignment has this mesh at zero", and the self-locking worm test
  is the gate on whether that is true.

---

## 5. What gets deleted

The user's standing instruction is that new functionality broadens or replaces
rather than adds. This is the ledger, and it is the plan's own success
criterion.

| Deleted | Replaced by | Approx. |
|---|---|---|
| `planetary::power`, `Power`, `Arrangement`, `PlanetaryShaft`, `basic_ratio`, `planet_speed` | the graph solve | ~280 lines |
| the hula stage's role-mapping and its forward/backward/at-rest quadruple solve | the same | ~200 |
| `train/planetary.rs`'s forward/backward/at-rest quadruple solve | the same | ~150 |
| `StageResult::ratio/efficiency/backlash` as `match kind` | graph readings | 3 matches |
| `carry`, `turns_per_port_turn`, the backlash referral | the three matrix readings | ~120 |
| `Port` as a two-valued enum | a named shaft | — |
| `ShaftsCase`'s `[f64; 3]` | per-shaft lists | — |
| `Stage::Hula` as a kind | a preset of the epicyclic stage, vocabulary gated by the arrangement | up to ~3000 |
| `Stage::Planetary` as a kind, once the grid reproduces it | a preset | up to ~2700 |
| four stage branches in `TrainPanel.svelte` | two — a pair, and a grid | ~600 |

The two large entries are Phases 5–6 and sit outside the first delivery
(§10.1); their size is conditional on the reproduction actually holding.
Everything above them is unconditional and lands in Phases 0–4.

**What is explicitly *not* deleted**, and why:

- `PairStage` and `train/crossed.rs`. A pair is the only kind with non-parallel
  axes, and shaft angle is a genuine parameter there; folding it into the
  epicyclic grid would be collapsing two different assembly problems, not one.
  Its kinematics joins the graph like everything else.
- `planetary::solve`'s shift absorption, `shift_bracket`, `ring_candidates`.
  These are the closure solve and they generalise rather than go.
- Every rating type. If any of them moves, the division argued in
  `docs/rationale.md#each-stage-kind-keeps-its-own-result-type` was wrong, and
  that is a finding worth having rather than a cost to pay quietly.

---

## 6. Phases

Each ends green — `cargo nextest run`, clippy, fmt, and every check `CLAUDE.md`
marks "yes" — and is committable on its own.

**Phase 0 — characterise.** No model changes. Extend the golden corpus so that
every arrangement of every kind is recorded before anything moves: all six
planetary arrangements already are; add per-shaft speeds and torques for the
hula stage, both drive directions on every kind, and a `gear-cli kinematics`
subcommand printing shafts, speeds, torques and mobility. One row in `COMMANDS`,
`tools/check_golden.sh --write`. **This is the arbiter for every later phase.**

**Phase 1 — the kinematic core, side by side.** `ratio.rs` and `kinematics.rs`,
pure, with no caller. Tested against `planetary::power` on all six arrangements
across a tooth-count grid, against the hula mapping, and against a new
`tools/train_kinematics.py` that builds the same systems from
`fractions.Fraction` and shares no code with Rust — extending
`tools/hula_kinematics.py`, which becomes one case of it. The lock-up invariant
goes in here, and **is run against the broken code before it is trusted** —
which is how it was found to be silent on the two faults it was advertised for
(§2), and why the sign and the frame answer to the cross-check instead.

**Phase 2 — stages declare their wiring.** `Wired` and its four
implementations. Both models live: a test asserts that the graph's ratio,
efficiency-free torque split and backlash agree with each kind's own, on every
preset and over a swept grid. **This is the phase that finds sign and frame
bugs, and the cross-check is the only thing that can** — so a kind's sign
should be read off its own `MeshKind` rather than written into the wiring by
hand, which makes the fault unrepresentable instead of merely detectable.

**Phase 3 — the graph answers.** Split in two when it was reached, because
one half moves no number and the other moves many, and mixing them makes every
diff unreadable.

**3a — the train assembles the graph, and motion is answerable without
geometry.** One `System` over every stage, the chain as coupling rows, and
`Train::motion` reporting exact ratios, shaft speeds and mobility from tooth
counts and topology alone. §1.3(a) lands here: `kinematics.txt`'s `unclosed`
and `chain-unclosed` fixtures print their ratios under the line where the
geometry refused. **The corpus diff is purely additive** — nothing existing
moved, which is what makes 3b's diff readable.

**3b — the existing numbers start coming from the graph.** Kind-matched
accessors and `carry` are replaced by graph readings; `planetary::power` leaves
production for speeds and stays as a test oracle. §1.3(d)'s sign resolution
lands here, which moves every pair's reported ratio and every torque sign
downstream of it, and the error taxonomy is reorganised along the same seam —
where `NoContact`'s three meanings are separated and the two wrong sentences
fixed (§1.3c). Every golden diff is examined one at a time: a change detector's
diff is a question even when the answer looks better.

**Loss stays with the kinds until Phase 7.** The graph's torque solve is ideal;
a stage's efficiency is not, and the per-mesh power direction that makes it
general is the hardest piece of the work. Saying so here is better than a phase
that quietly needs it.

**Phase 4 — ports, conditions, mobility.** Ports as named shafts, conditions as
a list under the existing relief walk, the affine family as a result. Load cases
name a port. `reacted` derived by the rowspace law *and* kept as an input
(§10.4), with the two reported side by side so the comparison can be made on
screen. Wire types, five string catalogues, and the condition panel in the UI.
**This is the phase that delivers multiple inputs and outputs, and where the
first delivery ends** (§10.1). Split when reached, along the same seam as
Phase 3: what the *train* holds, and what the *user* is shown.

**4a — constraints and couplings are the train's.** `train/conditions.rs`:
a `ShaftRef` names ground or one stage's shaft, a `Constraint` is `Held`,
`Driven` or `Free`, and the train carries a list of them beside a list of
`Coupling`s. A kind no longer stores an arrangement — `PlanetaryStage` lost
its `arrangement` field, the document format refuses it and its change log
says where it went — and instead answers `ports()`: which shafts a person may
address, and which of them convention holds. Every stage's `StageBoundary`
is *derived*, constraints laid over convention shaft by shaft, and the chain's
default couplings are read against the constraints so a Driven mid-train
replaces that stage's conventional drive rather than fighting it. The
boundary reports `topology` (each stage's ports) and `motion` (mobility,
every shaft's exact speed and its text, the ratios) beside the ratings, and
`arrange_stage` is the one gesture — `Train::arranged(stage, driven, held)`
— that the panel's "Driven by" and "Held" selects call, so the UI never
composes a constraint itself. The corpus did not move; the wasm recording
gained one entry and lost no number.

**4b — a load case names a shaft.** `Port` is `Start`, `End` or `At(ShaftRef)`:
the two names resolve through the boundaries — `Start` is the carrier of a
carrier-driven set at the head — and a case written at the shaft is the same
case to the bit, which a test holds on three trains and a corpus fixture
prints beside its chain-named twin. The direction a load travels is no longer
a table on the port (`Port::drive`, `Port::far` are gone): `Train::route`
walks the graph from the entry shaft, forward through a stage entered by its
input and backward through one entered by its output, to the far end. Two
refusals arrived with it, by name: a load on a shaft no load can be put on,
and one on a shaft two stages share — the latter the stated boundary of a
one-route, one-efficiency-per-stage loss model, which is Phase 7's to lift.
The route found a fault the walk had hidden: a set behind a pair, driven by
its ring, was handed a boundary whose output was its input and solved at a
ratio of one (`docs/corrections.md`). `MotionReport` lists the open ports,
named, and the panel's two port selects offer exactly those.

**4c — the family, and every other way the conditions can fail, named.**
A train short of conditions reports its motion as a family — each shaft's
speed a particular value plus one term per free shaft, *per turn of a port*
(the solver's own parameter is whatever column fell last, a planet, and
`Solution::rebased` moves it to the first open port the freedom moves) — and
refuses the rating by its own name, `Underdetermined { short }`. Conditions
are absorbed conventions-first so a conflict is named at the designer's
statement (`Overdetermined { at }`); a constraint on a shaft the train lacks,
an `i128` overflow and a stage left more than one free shaft each have a
sentence, and none of them is the wiring one. The mobility reported is the
mechanism's — ground is a shaft in the matrix and the frame in the world.
The panel shows the shaft line: every shaft's exact speed, the family where
there is one, and "1 short" where the ratings wait. Found on the way: the
overlay let a designer's second drive on one set remove the first, so a
differential's two inputs came out as one input and a family.

**4d — every port takes a constraint, on every kind.** Each stage card
lists its ports with one choice each — held, driven, free, or the convention
*as the overlay leaves it*, which the core reports per port (`by_convention`)
rather than the panel guessing from the kind. Two rules made a single select
per shaft enough: a hold the train writes on a stage replaces the kind's
conventional holds on that stage (as a drive already replaced its drive), so
holding a set's carrier releases its ring in one line; and a drive on a shaft
an earlier stage is coupled to is *where the chain enters*, not a condition
— its speed is the coupling's. With those, `Train::arranged` and the
`arrange_stage` entry point had nothing left to do and are gone, the planetary
card's two selects with them. Removing a stage re-indexes the constraints and
couplings that name later stages, which nothing did before.

**4e — closed by analysis, not by code.** Deriving `reacted` by the rowspace
law restates the flag: with the far port allowed a torque the rowspace
supplies one, with it allowed none there is none, and whether it is allowed
one is what the flag says (`docs/rationale.md#a-load-exists-only-where-it-is-reacted`).
What is derivable — where the reaction lands once declared — was already
reported beside it. Relief over constraints is the overlay's rule: a
convention gives way to a statement of its kind, a statement the designer made
does not, and a conflict between two statements is named at the later one
rather than relieved (`docs/rationale.md#a-planetary-needs-the-held-shaft-named`).
**Phase 4 is complete**, and with it the first delivery (§10.1).

**Phase 5 — the general epicyclic stage.** The mesh grid, the one-radius-per-
planet-shaft closure, the gutter toggle. `Stage::Planetary` becomes a preset
once the corpus does not move. Wolfrom, Ravigneaux, Simpson, meshed-planet and
stepped sets arrive with it and cost nothing each.

**Phase 6 — the hula stage absorbed.** The general stage reproduces every hula
figure; the kind goes and its vocabulary stays, gated by the arrangement
(§4.4, §10.2). If any figure cannot be reproduced, the kind stays and
`docs/corrections.md` records why — an honest failure is a better outcome than a
special case smuggled back in.

**Phase 7 — loss generalised, and the audit.** Per-mesh power direction,
circulating-power reporting, sensitivity `dR/dN`. Every figure that moves gets a
reason and an independent check, and every figure that does *not* move gets
evidence that the new path was reached at all (§10.3). Then the pass the
standing instruction asks for: what the new learnings let us ablate, and whether
the four documents describe what now exists.

**Optional, scoped separately.** The lever diagram (the `m = 2` nullspace
rendered — the solution, not a decoration); the inverse problem, searching
integer tooth counts for a target ratio under closure and assembly at once.

---

## 7. Testing

Following `CLAUDE.md`'s order of what has actually caught things.

**Verify against something that shares no code.** `tools/train_kinematics.py`
(Phase 1) builds every system in exact rationals from the topology alone. It
replaces `hula_kinematics.py` rather than sitting beside it — the same
consolidation `tests/common/mod.rs` did to three parameter grids.

**Ask what property the answer must have.** Laws, checkable without knowing the
answer, asserted over every topology:

- lock-up is in the nullspace;
- `Σ τᵢ ωᵢ = 0` ideal; `Σ τᵢ ωᵢ = −losses` with `losses ≥ 0` lossy;
- mobility is invariant under relabelling and under adding a coupling between
  two already-coupled shafts;
- a chain of pairs gives the product of the ratios;
- the exact ratio and its `f64` reading agree to 1e-12 — two representations
  checked against each other;
- a member's engagements from the graph equal the per-kind arithmetic
  (a transitional law, deleted with the arithmetic);
- every member of a reacting stage reports a finite share of the load, signed
  like its own torque — the existing invariant, now over arbitrary topology.

**Prefer a law to a threshold.** The exact arithmetic means mobility, lock-up
and "this ratio is infinite" are decided without an epsilon anywhere.

**Turn every axis, in every context it reaches.** `tests/common/mod.rs` is one
parameter grid with every gear input nameable. The analogue is a **topology
grid**: a generator over tick patterns — number of centrals, number of planet
shafts, gears per shaft, gutter state, internal/external per mesh — so that a
law is asserted over the arrangements rather than over four presets.
`every_kind()` in `train/mod.rs`'s tests becomes one case of it, and the relief
laws that already run over every kind's preset run over the grid instead.

**Before trusting a new gate, run it against the broken code.** For each of the
lock-up invariant, the mobility count and the power balance: `git worktree add`
a detached HEAD, introduce the specific fault the gate is for — a flipped `σ`, a
frame on the wrong shaft, a mesh row with the wrong count — and watch it fail.
This project has three recorded gates that could not fail; this refactor is
exactly the kind that produces a fourth.

**The corpus is not optional.** `CLAUDE.md` records that perturbing five of the
rating model's cited constants leaves the entire test suite silent and the
corpus catches every one. Nothing here is believed on a green `nextest` alone.

**And silence is not agreement.** Per §10.3, a golden diff answers only half the
question. A figure that moved has to be shown right by something that is not the
diff; a figure that did not move has to be shown to have been *reached* — that
the new code path ran and produced it, rather than a fallback producing the old
answer. The mechanism is the one `docs/corrections.md` already names: an opt-in
the harness never switches on is a path the detector cannot see, so each new
path carries a case in the corpus that exercises it, and a coverage assertion in
the tests that the case reaches it.

---

## 8. Documentation, strings and checks

Traced against `CLAUDE.md`'s *To change X, touch these*.

| Change | Files |
|---|---|
| the kinematic model | `kinematics.rs`, `ratio.rs` · `cargo nextest run` · `check_golden.sh` · `check_figures.py` · `check_units.py` |
| a stage-level input (conditions, the grid) | the kind's file and its `Constrained`/`Wired` impls · `train/mod.rs` · `auto.rs` if a search reads it · 5 × `strings_*.toml` · `TrainPanel.svelte` · `check_bindings.sh --write` · `check_strings.py` |
| a load-case input (the port becoming a name) | `LoadCase` · `solve_train` · `gear-wasm`'s `defaults` · `gear-io/src/train.rs`'s change log · 5 × `strings_*.toml` · `TrainPanel.svelte` |
| notes the solve emits (mobility, under-determined, circulating power, disconnected shaft) | `note.rs` · the raising site · 5 × `strings_*.toml` |
| types crossing the boundary | `check_bindings.sh --write`, then `cd web && npm run check` |
| the file format | `gear-io/src/train.rs` — a new entry in the change log per shape change, and **no compatibility shim**, per the module's own standing decision |

**The four documents.** Nothing appears in two of them:

- `reference.md` — a new **Kinematics** section carrying the mesh row, mobility,
  the four readings and the torque layer. *Planetary sets* keeps its geometry
  and loses its kinematics to it; *The hula stage* becomes a preset's section;
  *Trains* loses the ratio product and the backlash referral to it.
- `rationale.md` — new standing entries: *Kinematics is one matrix, read four
  ways*; *A frame is a parameter, not a role*; *Mobility is an answer, not an
  error*; *Closure is solved where something can absorb it and reported where
  nothing can*; *Exact where the answer is integers*. Revised: *A planetary
  needs the held shaft named* (it is named at the train), *Each stage kind keeps
  its own result type* (the verdict on its own untested claim), *A load case is
  a torque, a port and a kind*.
- `corrections.md` — §1.3's two faults, and whatever else is found. The user's
  instruction that existing outputs are not to be trusted means entries here are
  expected, not a sign of trouble.
- `state.md` — *Not built → A third port* struck; new canaries; every new bias
  with its **size and sign**; the ablation ledger's outcome.
- `CLAUDE.md` — the map: new modules with what they must *not* know, new
  *To change X* rows, and any new check.

---

## 9. Risks, and what would change the plan

| Risk | Handling |
|---|---|
| **Sign or frame errors in the assembler** — the most likely defect class | Phase 2's both-models-agree sweep and the independent Python check. **Not the lock-up invariant**, which was measured silent on both (§2) |
| **The lossy generalisation moves shipped numbers** | Expected and allowed (§10.3), never accepted on sight: each moved figure gets a reason traced to the per-mesh direction, and each *unmoved* one gets evidence the new path was reached. The corpus raises the question; `train_kinematics.py`'s power balance answers it |
| **The hula stage does not reduce** | Phase 6 is allowed to fail. `Stage::Hula` stays and the reason is written down |
| **`TrainPanel.svelte` is 2675 lines with a branch per kind** | the grid renders from data the core declares; the pair keeps its branch; four become two |
| **Mobility > 1 in the UI is genuinely hard above 2 DOF** | the handoff is right: ground or drive down to 2 DOF and show that slice. No 3-D lever |
| **Scope** | Phases 0–3 deliver no new user-visible feature and are the majority of the risk. They are also what makes 4–6 cost nothing each. Stopping after 3 leaves the tool strictly better and strictly smaller |

**What would change this plan:** a measurement showing that a stage's assembly
relations are *not* per-arrangement — that one closure law covers every kind
including the pair's centre distance and the crossed pair's — would argue for
the handoff's flat netlist after all. §4.4 finds one law covering the epicyclics
and not the pair; if that turns out to cover the pair too, §5's decision to keep
`PairStage` should be reopened.

---

## 10. Decisions taken

Reviewed and settled before any code. Each is recorded here because it changes
what the phases do, not merely what they are called.

**1. Scope — Phases 0–4 first.** The kinematics unified, multiple inputs and
outputs, mobility. No new stage type in the first delivery: 5–6 are where the
Wolfrom and friends arrive, and they are nearly free once 0–4 are done. Most of
the risk is in 0–3, which deliver nothing user-visible, and that is the right
order for it.

**2. `Stage::Hula` is absorbed, vocabulary and all.** It is fundamentally a
slight variation of an epicyclic and belongs in the same kind. What carries over
with it is a *vocabulary and a set of inputs*, not a model — a far-side
clearance, an offset stated as a crank distance, the words a designer of one of
these uses — **gated by the arrangement that makes them meaningful**, which for
most of them is a planet count of 1. This is the worm's treatment exactly: a
preset, a vocabulary, and a choice of which inputs to put in front of a
designer, over one primitive. The gate is a property of the arrangement and is
asked of it, never a `match` on a kind tag.

**3. Loss — per-mesh direction is the intended end state, and the numbers are
allowed to move.** But no change is accepted on sight. Two things are required
of each moved figure:

- a reason, traced to the per-mesh direction rather than to an assembler bug;
- **and a check that does not rely on the diff.** *A failure to change is not
  evidence of success* — `docs/corrections.md` records both halves of this
  (*a figure that changed is not evidence that today's change moved it*, and
  *a check built from the thing under test measures nothing*). So the per-mesh
  loss model is verified against `tools/train_kinematics.py`'s independent power
  balance in both directions: a figure that moved must be shown right, and a
  figure that did not must be shown to have been *reached*, not skipped.

**4. `LoadCase::reacted` — the rowspace law is the ideal, the input stays for
now.** "Nothing holds this" is useful vocabulary for a designer, and whether the
law can replace it without loss is a *usability* question that cannot be
answered until the UI meets the new core. So: derive it, report it, keep the
input, and revisit once Phase 4's panel exists and the two can be compared on
screen. *Revisited in 4e: the law restates the input, and the input stays.*

**5. The lever diagram** — deferred, and scoped separately. The tool draws no
epicyclic today and `state.md` records that as deliberate.

**6. Branch and cadence.** `train-as-graph` off `main`, one commit per phase,
each green against everything `CLAUDE.md` marks "yes". Nothing merged without
explicit permission.
