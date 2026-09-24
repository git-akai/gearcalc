# Rationale

Why each model here is the one chosen, on what evidence, and what would reopen
it. Read this before changing anything in `gear-core`.

Every entry has the same three parts, and the third is what makes this a
rationale rather than a list of opinions: **the decision**, **the measurement
that settled it**, and **the condition that would change it**. Where there is no
such condition, the entry says so.

What the tool computes is [`reference.md`](reference.md). What was once wrong and
how it surfaced is [`corrections.md`](corrections.md). What is built and what is
not is [`state.md`](state.md).

---

## The standing rules

These are not preferences. Each was arrived at by something going wrong, and
each is enforced by something other than good intentions.

### No engineering calculation in TypeScript

If a number appears in the UI, Rust computed it. TypeScript formats it and
nothing else.

**Why:** it is what keeps the Rust test suite meaningful. Logic that migrates to
the view layer lands where nothing tests it.

**A default is one of those numbers**, and so is a diameter, and so is an angle
in degrees. The gear tab's cutter default was written down in both languages and
the two drifted — TypeScript kept the *rack's* 0.38 tip round, which no 20-tooth
shaper can hold, so every ring the UI drew was cut by a tool that generates no
fillet. Rust was right throughout and could not see it, because the wrong number
lived only on the side with no tests.

**Enforced by:** defaults, bounds and strings all crossing from Rust, and the
wire types being generated rather than written twice. If you find yourself typing
an engineering number into a `.ts` file, that is the bug.

### No English in `gear-core`, and no engineering in the catalogue

The core emits a `Note` — a stable key and the values a sentence needs — and
every word lives in `crates/gear-io/data/strings_en.toml`, one file per language.

The division is **not** "text on one side". **Rounding stays in the core**,
because how many decimals a quantity deserves is a judgement about the quantity:
a translator may decide how `1.234` is written but not whether the fourth digit
is worth printing. `Note::number` takes the decimals explicitly so that choice is
made every time and readable at the call site.

**Why a key and not a sentence:** a consumer that wants to *act* on a note has
nothing to match on but its text otherwise. Four places once branched on a
message's wording, every one of which would have gone quietly false on a
rewording and all of them on a translation.

This covers **errors** too. `MeshError`, `MeasurementError`, `TrainError` and
`ScrewError` each carry `Explain::note`, and `Maybe::Unavailable` carries a
`Note` rather than a string, so one typed channel holds every reason a user
reads. Their `Display` impls remain, for the CLI and for `Debug`; what changed is
which of the two the browser sees.

### Inputs are the only state

Outputs are recomputed, never stored, so nothing can go stale. A full solve is
microseconds. In exchange: no cache invalidation, no dependency graph, no
field-updates-field wiring, and outputs that cannot disagree with inputs.

Shared-within-a-stage values live once on the stage, so `k₂ = 2 − k₁` is derived
and the invariant is unwritable rather than merely tested. A planetary set makes
the point harder: its two meshes want *different* invariants and share the
planet, so one stored `k` fixes all three.

### Find the parameter, not the branch

Where two cases look different, look for the value of one construction that
reproduces the other.

- Line contact is the zero of the lengthwise curvature.
- The rack is the shaper at `z_c → ∞`.
- A spur gear is a helical one at `β = 0`.
- **A ring is a gear with a negative tooth count.**
- A concentric gear is an eccentric one at `Δx = 0`.
- A parallel pair is a crossed one at `Σ = 0` — though see the caveat below.
- An unshared bending rating is a shared one at `LoadSharing::None`, which
  sweeps nothing and takes the point the unshared reading always took.
- A member in one mesh is a member in a list of one, so a planet in two needs no
  arm of its own ([the rating](reference.md#load-cases)).
- A load case is the worst one a mesh carries, scaled — wherever the stage's
  power split does not depend on the magnitude passing through it, which is a
  property of the shape's flow — linear in the torque through it — rather than
  one assumed of gearing.
- A load from the far end is a load from the near end with the direction
  reversed: one flow carries either, each mesh's driver whichever side the
  power comes from, and a mesh that cannot be driven that way holds either
  ([load cases](reference.md#load-cases)).

This is not tidiness. **Every surviving `match kind` is a place where two answers
can silently disagree**, and the corrections log is largely a record of exactly
that. The internal relative curvature was wrong in two independent ways at once
while it was written out as its own branch, and both faults were unreachable —
so both would have gone live together the moment anything reached them.

**The caveat, and it matters:** a degeneracy is not the same as a value. At
`Σ = 0` a crossed pair's two conditions on the contact normal collapse into one
and the line of action becomes a plane. `Screw::new` refuses it rather than
pretending, because the freed rotation *is* the operating pressure angle and the
parallel model has an exact law the crossed one cannot express.

### An input limit means "could this gear exist?"

Not "would anyone want it?". A guard that refuses a legal shape will one day
refuse a legitimate design.

**Measured:** a one-tooth gear, an 85° helix, a 2° pressure angle and a negative
addendum all produce finite, closed, correctly ordered cross-sections. None of
those is a limit of the mathematics; all four were convention.
`tests/extremes.rs` is the standing evidence.

The converse also holds: a *cutter* whose corner rounds overlap is not a tool,
and a body with `T ω ≤ 0` is not an input. Both are refused.

**And it cuts against convenience.** The buildable shift range runs past the
depth the dedendum asked for, because a deeper hob cuts that gear perfectly well
— refusing it would be refusing a part that can be made. What the designer is
owed there is to be *told* the tool is no longer the one specified, which is what
`ShiftRange::shallow_cut` is.

### Clamp rather than refuse, and say so

Where an input describes something the geometry cannot honour exactly, the
answer is the nearest thing that *can* be cut, reported — not a refusal.

**The distinction that makes this coherent** is which question the input is
answering. A **clearance** is secondary: it says how much room to leave, and
leaving slightly less is a part rather than a contradiction. So a dedendum, a
tip round, a cutter depth are all clamped and noted. What is refused is input
that describes *no shape at all*: a pinion cutter larger than the ring it is to
cut, a module of zero, a body with `T ω ≤ 0`, two gears with different racks.

This is the reading that settles the shift ceiling above — the dedendum is
clearance, so treating it as secondary to unify the model is a fair conceit —
and it is what makes the tool's own round a clamp rather than a refusal.

**Measured, and it was a live inconsistency.** The identical guard — the tip
round does not fit the tool's tip — was clamped on an external gear
(`clamp.fillet_capped`, backed off to 95 % of what fits) and **refused** on a
shaper, so `Ring` fell back to `fillet: None` and the part had no fillet at all.
One input, two answers, and the jump was in kind rather than in degree at a point
where nothing physical happens. The shaper caps now, by the same rule and with
the same note.

**And a clamp has to be continuous too.** Backing off by the 5 % margin only once
the ask crosses the boundary drops the realised round *at* that instant, which is
a step in the quantity the clamp exists to keep smooth. `min(asked, 0.95 × max)`
is monotone in what was asked for; `if asked > max { 0.95 × max }` is not. Gated
as that law rather than against a threshold.

### An arrangement is a "stage"

A worm stage, a hula stage; never a worm drive or an eccentric drive.

**Why:** one arrangement carried two names — the hula stage in the core and the
CLI, *the eccentric drive* in comments and half the documents — which put
"eccentric" on two unrelated features, this and the angularly varying profile
shift, and left a reader to work out that they were not related. A name that has
to be disambiguated in the reader's head is a name that will be
disambiguated wrongly.

**"Drive" survives in exactly two senses**, and in both it is the right word and
nothing else is:

- **which way power flows** — `Drive::Forward`, driven forward, back-driving, a
  drive flank against a coast flank;
- **how a load is applied** — a reversing duty, whether the duty reverses.

Neither names a part of a geartrain, so neither collides with the rule. And
where the thing being named is not a stage, it is not called one:
`train/arrangements.rs` describes **arrangements** — what sits where, knowing
nothing about loads — and `screw.rs` a **pair**.

**What would change it:** nothing about the word. If a future arrangement is
genuinely not a stage of a geartrain, it needs its own noun rather than this
one stretched.

### An orbiting output is a coupling, and a coupling is the stage's to lose

A planocentric reducer's output was its planet's own body — an orbiting
port — and whatever followed it in a train was joined to that body. A stage
kept its axes to itself, so the next stage's gear sat on a fixed axis of
its own while being the same body as a planet going round the eccentric,
and the model said nothing about how. **A shaft is straight**: the train is
becoming one graph in which a body turns about one axis, and on it that
join puts the next gear on the eccentric.

The machine answers it. A cycloidal disc's turn is taken off by pins in
holes, or an Oldham coupling, or a Schmidt coupling, to a shaft on the
centre line — so the output is **a second body, coaxial with the input,
turning with the disc**. That is an offset coupling: `ω_a = ω_b` between
bodies on parallel axes, lossless, with no geometry and no play. The
kinematics had the row already (`System::couple`, a coaxial coupling's),
and the flow passes it as a mesh of one tooth and minus one, whose frame
carries nothing.

**It is not what a planocentric is.** A preset is a starting point: a
planocentric with a step on its planet, a ring on the step and the coupling
taken away is a hula stage, and a hula with its wobble body coupled and its
second step taken away is a planocentric. So the coupling is an element a
stage has, added and removed like a step, and a law walks both ways ratio
for ratio. A preset the edits could not reach, or leave, would be a hole in
the edits rather than a property of the preset.

**What would change it:** an output mechanism with a ratio of its own — a
pin-and-roller output that is not 1:1 — would be a mesh, not a coupling.

### A train is one graph, and a stage is a part of it

A train was a list of stages, each with axes of its own, and a body two
stages listed was what joined them. That let one shaft stand on two axes —
the stage's input on its own axis, the stage before's output on another,
one body — which nothing could see as wrong, and it made a train two
readings of every question, the stage's and the train's. **Stored as one
graph, a body has one axis**: a train is one `Shape`, every preset laid
into it, and a join makes the two axes one line.

**A stage is what the graph falls apart into**, read off it and never
stored: its parts, the pieces that close, search and rate apart
(`Shape::parts`) — gears joined by meshes, meshes by the distances they
share. Nothing a part closes, sizes, searches or rates reads another, so a
part's result is its stage's result with no slicing, and a train built stage
by stage falls apart into exactly its stages, each solving as the preset
alone does; laws hold both, and a file written as stages is converted to
the graph a chain of the same stages builds now (`graph_of`, `gear-cli
convert`). Parts that are derived follow the graph without being told: a
mesh removed that leaves two parts is two.

**What it costs, and where it is paid:**

- **Nothing on screen is a part.** The panel shows the graph's pieces — a
  mesh, a body, an axis — by the graph's own indices, and a stage is what a
  preset laid in and nothing after it: an edit is the graph's (`Edit`),
  offered at a piece and never asked of a stage, so there is no second
  numbering to read an edit through, and a box bound to a piece writes the
  train. The part stays in the core, the unit a result is closed,
  searched and rated in, and the harness prints the corpus part by part.
- **A part lists what it has something on.** A layshaft's output with no
  gear engaged is the next stage's input and nothing of the layshaft's
  part, so in neutral that part has no end of it. The shaft stays — the
  next stage's gear names it — and a gear engaged onto it brings the end
  back.
- **A stage asked alone is one part**, whatever it falls into
  (`Part::whole`): two pairs through a compound shaft, asked as a stage,
  are the stage the harness and the tests mean.
- **One file format, and a one-off converter** rather than a second reader,
  for the reason the format gives: two readers for one format is two to
  test for ever.

**What would change it:** nothing about the one graph. How it is shown has
been revisited once — a card a part gave way to one list of the graph's
pieces and one workspace, since a part is the right unit for a result and
not for a screen.

### Degrees where a designer states a number, radians in the mathematics

Both units are right and the tool needs both: a drawing says 20°, and every
trigonometric expression wants 0.349. So a *stage input*, a *gear input* and
anything crossing the boundary are in **degrees**, and everything from
`plane.rs` inwards is in **radians**, converted once at the edge —
`BasicRack::new(module, pressure_angle_deg, helix_angle_deg)` is where that
happens for a rack, and `PairStage::screw_at` for a screw pair.

**What is not right is a name that means both.** This crate had four —
`shaft_angle`, `lead_angle`, `wheel_helix_angle` and `pressure_angle` each
denoted degrees in one module and radians in the next — and eight further angles
that stated no unit at all.

It cost a real bug, and the shape of it is the argument.
`Screw::least_distance_lead_angle` was written to take a shaft angle in radians;
`PairStage::shaft_angle` holds degrees; the call site read perfectly well to its
author and was wrong. The field was **documented**. Documentation is not what a
reader checks — the *name* is.

So the radian one carries `_rad` wherever the name would otherwise mean both
(`lead_angle_rad`, `shaft_angle_rad`, `pressure_angle_rad`), a mismatched call
site reads wrong instead of reading fine, and `tools/check_units.py` keeps it
true: every angular field states its unit, and no name is used in two.

Greek names — `alpha_n`, `alpha_t`, `alpha_w`, `beta` — are radians by
mathematical convention, and still say so. A reader who does not know that
convention is exactly the reader the sentence is for.

**Why not a newtype.** `Deg(f64)` and `Rad(f64)` would make the confusion
impossible rather than merely visible, which is stronger. It is declined for now
on the same footing as Q3's closed form: it touches forty-seven fields, most of
which cross the boundary through `ts-rs`, to remove a fault that a suffix and a
check already remove. If a third unit ever appears — gradians, or turns — that
arithmetic changes.

### Say what is not modelled, next to the number

A worm stage shows no bending stress and says why on screen. A ring shows the
smallest tooth count its design could have had. A planetary set states in its own
notes that it assumes the planets share load equally.

**And say nothing where nothing is known — but check that "nothing" is not
itself a discontinuity.** Refusing to answer looked like caution when the bending
correction hit a flank tangency. It was a cliff: a number becoming no number,
with nothing physical happening at that tooth count.

### A conservative answer is not a free one

**A conservative result is no less wrong than an aggressive one.** Both are
errors, both carry engineering risk, and only one of them is habitually excused.
That habit is the more dangerous of the two, because a bias that is always
forgiven is a bias nobody measures, and because the errors *stack*: a tool that
rounds every judgement toward safety, five judgements deep, does not produce a
safe answer — it produces an unknown one, wearing a reassuring sign. The gear it
sizes is heavier, larger and more expensive than the design required, and the
margin it claims to hold is not the margin it holds.

So "the safe direction" is **not a reason** to omit a factor, keep an
approximation, or prefer a model. Where this document says a choice is
conservative, that is a statement of its *direction* and never of its
justification, and it should be read as a debt: an error of known sign and
unknown size, still to be paid off. The justification always has to be
somewhere else — that the model is the right one, that the input is the one that
was measured, that the fit is the one calibrated to the geometry it is fed.

This project has been caught by the opposite mistake and by this one in the same
session. `Y_β` was excluded for years on a reading that made the omission look
conservative, and adopting it turned out to move the tool *further* from the
standard it was taken from ([the helix factors are a
pair](#the-helix-factors-are-a-pair-and-this-tool-can-take-neither)). The notch
factor was fed a fillet radius from the flattest point of the fillet — which
under-predicted, and would have been defended the other way round had it
over-predicted. Neither was found by asking which way the error pointed. Both
were found by asking which model the number belonged to.

**How to apply it.** Every known bias is written down with its size and sign, in
`state.md`'s "Known-approximate" list, not merely with its direction. A factor
is applied or declined on whether it belongs to the model in use, and the
argument is made on that ground alone. An unmeasured conservative bias is a
defect with a note attached, and it stays on the list until somebody measures it.

### Prefer a law to a threshold

When a model's limit has to be disclosed, find the property that makes the
disclosure checkable rather than picking a number.

- Crossing shafts can only *add* sliding, so a crossed pair beating the same
  teeth run parallel is the model admitting it lost some.
- Two models that meet at a boundary are gated by watching the disagreement
  *vanish with the parameter* — halve the clearance and the gap halves — so no
  tolerance gets chosen.
- A step and a coarsely sampled curve look identical at one sampling. What
  separates them is that refining the sampling shrinks a curve's largest jump and
  leaves a step exactly where it was.
- When an invariant is exact, test it exactly. Sampling an outline and watching a
  trend is the wrong instrument for two angles that either meet or do not.

**A disclosure can expire.** "This efficiency omits profile sliding" was true of
the old model and became wrong of the new one — it then fired on the *more* exact
number. Deleted, not reworded.

---

## The mathematics

### Where closed form is impossible

Ten scalar solves, each monotone, each bracketed, none an optimiser, none with
a tuning parameter. Everything else in the crate is algebraic — **except one
search**, which is named below rather than left out of the count.

| # | Solve | Method |
|---|---|---|
| 1 | `inv⁻¹` | series seed + safeguarded Newton, with a domain guard |
| 2 | Tip radius for a given tip width | Newton, analytic `ds/dr` |
| 3 | Flank/fillet junction when undercut | Brent, bracketed by construction |
| 4 | Planet shift for a common axis distance | Newton, closed-form bracket |
| 5 | 30° tangent critical section | Brent on the trochoid parameter |
| 5b | Inscribed-parabola critical section | Brent on the fillet, then on the flank's roll |
| 6 | Contact ellipse aspect ratio `κ` | Brent in `ln κ` |
| 7 | Cutter travel at a ring's flank/fillet junction | Brent on the trochoid's radius |
| 8 | Cutter travel where a ring's fillet reaches mid-space | Brent on the trochoid's angle |
| 9b | Where a severed tooth's trochoid is least | Brent on `dθ/ds`, analytic |
| 10 | The undercut minimum, where the tool's round is capped by the depth it reaches | Brent on the shift, the tool settled at each trial |

The involute function is not algebraically invertible, and that single fact
causes #1, #2 and #4.

**#10 is reached only where the closed form stops being exact.** The undercut
indicator is linear in the shift *for a given tool*, and ordinarily the tool is
the same at every shift, so one closed-form step lands on the minimum. But the
tool's round is capped at a fraction of the depth it reaches, and that depth
moves with the shift — so on a three-tooth gear cut with a full ISO round the
minimum is a fixed point of *shift → tool → shift*, not a line. The residual is
continuous and non-decreasing, every regime of the cap being so, and it is
bracketed and solved. Derived for the round *asked for* instead, the minimum
sat 0.03 mm into undercut wherever the cap reached, and the gate that should
have seen it stood ±1e-4 from the edge with the root-radius axis untouched
([corrections](corrections.md)).

**#9b replaced a scan, and the reason is the general one.** Severing is the
trochoid reaching the tooth's centreline, and it used to be found by sampling
`θ` at two thousand points and taking the least. A scan has a resolution and the
thing it looks for does not: the negative excursion narrows to nothing as a
tooth approaches severing — measured at a twentieth of a thousandth of the
interval where the scan resolved half a thousandth — so the count was ten times
too coarse for the case that matters and **no larger count fixes it**, because
the window closes to zero. What a boolean flipping costs is also different in
kind from a figure moving.

`θ` is unimodal along that interval, measured over three thousand undercut teeth
and gated as `the_trochoid_turns_once`, so its minimum is a root of the slope
and the slope is closed form. The answer moved by 1.8e-15 mm across the whole
shipped population and no tooth changed classification, which is what a
refinement rather than a correction looks like.

**#6 had a tempting alternative and refusing it is the same decision as the
rest.** Hertz's aspect-ratio relation has widely used closed forms —
Hamrock–Dowson's `κ ≈ 1.0339(R_y/R_x)^0.636` and its relatives — but they are
*fits* to the relation, not solutions of it. Taking one would put a fitted
exponent underneath every crossed-axis contact stress the tool reports. The solve
costs about fifty function evaluations and is exact.

**Guards matter as much as solvers.** Ordinary planetary inputs routinely request
a axis distance outside the involute domain, and the difference between a
guarded and unguarded `inv⁻¹` there is the difference between "this ring tooth
count is impossible" and a NaN silently reaching a stress figure.

**What would change this:** nothing in prospect. A published closed form that is
a *solution* rather than a fit would retire one of the ten; none is known.

### ...and the one thing in the crate that is none of the above

**`auto::maximise` is a search, and it has tuning parameters.** The paragraph
above says "each monotone, each bracketed, none an optimiser, none with a tuning
parameter", and that is true of the ten solves and was read for a while as
though it were true of the crate. It is not. Every stage that chooses profile
shifts for efficiency calls a bounded box sweep followed by a multi-start pattern
walk, carrying a search span, a scan step, a stopping resolution, a work budget,
a start count and a first step size. The tables in
[reference.md](reference.md#efficiency-parallel-axes) that report what a shift
division is worth are its answers.

It is named here because an inventory that quietly omits its one exception is
worse than no inventory: the exception is exactly what a reader of this section
would want to know about.

**Why it is a search rather than a solve.** The objective is smooth but its
optimum sits *against a constraint* rather than in a bowl, and which constraint
binds changes across the domain — undercut, a pointed tip, a root round that no
longer fits, a contact ratio floor, bottom clearance. A bracketed scalar solve
needs a monotone residual with a sign change, and a constraint boundary is where
the admissible set ends rather than where a derivative vanishes.

**Half of it has a closed form and does not use it.** At a fixed shift *sum*,
the stationary condition for the **division** is derived and checked
([reference.md](reference.md#efficiency-parallel-axes)) — and the search chooses
the division numerically all the same. This section said otherwise for as long as
it has existed, which a grep for the function's callers settles in a second and
nobody had run.

Two things stand in the way, and the second is the reason it stays that way. As
derived, the condition assumes each tip moves at `m` per unit of shift, so the
factor cancels between the members; `no_sharp_tip` is **on by default** and holds
a capped tip at the radius where the tooth is `min_tip_width` wide, which moves
at about *half* that and differently on each member. And the corrected condition
would still only supply the *interior* candidates: the optimum is as often at an
end of the admissible interval, so using it means bracketing that interval's two
ends, the cap's onset, and a stationary point per smooth piece — five solves
where a bounded one-dimensional search is one, for an answer already right to a
thousandth of a module. The audit's record (`docs/history/audit.md`, Phase 4)
carries the derivation, verified, for whoever weighs it again.

Every row of every table this project prints reports the sum landing on a bound
rather than at a stationary point, which is the other half and the reason the
whole thing is a search.

**And the search is converged only where its coordinates are the problem's.**
The six numbers are an input now (`auto::Search`) rather than constants in the
loop, because "these are enough" is a claim about them and a claim nothing can
raise is a claim nothing can check. Raised, it holds for a pair — fourteen times
the work moves its efficiency by at most 4.1e-7 and no shift by more than one
step of the stopping resolution — and fails for an epicyclic set, where it moves
`η₀` by up to 2.4e-4 at a sun shift 0.30 modules away, in **both** directions.
More effort finding a worse answer is not a budget that is too small; it is a
walk whose result depends on where its sweep happened to put it.

A pair is searched in the pair's own two directions: the shift sum, which sets
the operating pressure angle and so the whole length of the path, and the
division, which only moves the path's two ends against each other. One of those
is flat and it is an axis. A set is handed two of its three raw shifts, and its
admissible region is bounded by a curve the optimum lies against — so the walk
slides along the bound rather than climbing to it.

**What would change this:** establishing that the loss is monotone in the sum up
to whichever constraint binds first. If it is, the sum becomes a one-dimensional
bracketed search against the active bound and the crate has no optimiser left —
and the same question asked of a set is the same repair, since *finding the
coordinate the constraint is flat in* is what both need. That is being attempted
rather than assumed, and until it lands, this section describes eleven things
and not ten.

### The Lewis parabola over the 30° tangent

A cantilever whose outline is a parabola with its vertex at the load carries
uniform bending stress, so the largest such parabola inscribed in the tooth
touches where the real tooth is weakest.

**This diverges from ISO 6336 and AGMA 2101**, which specify a fixed 30° tangent.

**Measured:**
- The 30° tangent is *independent of where the load acts*, which is the one
  property the cantilever model is meant to have. Its tangents cross the
  centreline 11.8 % below the load point at z = 9 and 0.04 % above it at z = 60.
- It is the more conservative construction everywhere: +2.9 % to +13.7 % on `Y_F`.
- It changes rankings very little — Spearman ρ = 0.993 over 1521 designs — so the
  choice is principled rather than consequential.
- NASA TM-107012 makes the same choice for internal teeth, independently.
- Experimental single-tooth-bending work reports measured critical locations
  *above* the 30° prediction, which is the direction the parabola moves it. The
  authors attribute part of the divergence to large test deformations, so this is
  support rather than proof.

`CriticalSection::TangentAngle` is retained, unused by default, for a number
comparable with a published rating.

**And the notch factor now matches it.** For a long time the parabola section
was paired with ISO's `Y_S`, which is fitted to the 30°/60° tangent section —
half of one calibration against half of another. The section's own source
(Savage, Rubadeux & Coe) carries Dolan and Broghamer's `K_f`, and that is what
is applied. See [the notch factor is not a free
choice](#no-isoagma-correction-factors).

### No ISO/AGMA correction factors

`Y_β`, `f_ε`, `Y_DT`, `K_A`, `K_v`, `K_Fβ`/`K_Hβ`, `K_Fα`/`K_Hα`, `Z_ε`, `Z_β`
and their relatives are not used, and will not be added on request without
revisiting this.

**Three reasons, in order of weight.**

1. **Their validated band is narrow relative to modern designs.** Each carries
   hard caps that are not physical thresholds; nothing changes in the mechanics
   at exactly the edge of the data. Outside the band the formula does not fail —
   it quietly returns the boundary value, which is the worst failure mode
   available.
2. **They are only balanced as a set.** `K_Fβ` raises a stress where the
   geometry factors lower it, over the *same* face-width physics from opposite
   directions, and all were calibrated against `σ_Flim` values themselves
   back-derived using the whole set. Adopting one is taking the favourable half
   of a calibration — which this project then went and did, once, and had to
   measure its way back out of: see
   [the helix factors are a pair](#the-helix-factors-are-a-pair-and-this-tool-can-take-neither).
3. **It trades accuracy for precision.** A number that is exactly right about a
   simpler question beats one that is approximately right about a harder one
   while hiding which question it answered.

**The evidence that these are empirical rather than derivable is direct:**
published comparisons against finite-element analysis find methods that disagree
on whether root stress *rises or falls* with helix angle — the sign of the trend,
not merely its size. There would be nothing to disagree about if it were
geometry.

**Two of ISO 6336-3's factors are the deliberate exceptions**, and the same
three tests separate them from the list above rather than a different one: they
are computed from *this* gear's own geometry rather than looked up against a
population, dropping them is **unconservative** rather than safe, and neither is
half of a balanced pair — nothing else in the model pushes back against either.

A notch factor is the ratio of peak fillet stress to nominal section stress — a
*local* effect computed from the tooth's own `s_Fn`, `h_Fe` and fillet radius,
all measured off our exact profile. Dropping it would report a nominal stress
roughly 1.6–2.1× below the real peak.

**Which notch factor is not a free choice**, and that is the whole lesson of
this section applied one level down. The critical section here is the inscribed
Lewis parabola searched over both curves, which is Savage, Rubadeux & Coe's
construction; the notch factor that model carries is Dolan and Broghamer's
`K_f`, read at `ρ_f`, the minimum radius of the fillet. `Y_S` is fitted to ISO's
tangent section and to `ρ_F` measured *at* that section. Pairing one with the
other is taking half of a calibration — the same error as `Y_β`, one level
smaller, and it was live for as long. `RootSection` carries both radii and each
fit reads its own; `CriticalSection::TangentAngle` with `Y_S` is the ISO set,
whole, for a comparable number.

`Y_S`'s notch parameter is clamped into the fit's stated range and **reported
raw**, because `Y_S` rises with `q_s` and clamping a sharper-than-stated notch
under-predicts stress. `K_f` states no band and clamps nothing.

**A mechanics-derived factor was looked for and does not exist for this
geometry.** Neuber's notch theory is genuinely elasticity-derived, but every
form of it needs a notch depth and a net section, and a gear fillet is a
transition from tooth to rim rather than a notch cut into a prismatic bar —
choosing that depth reintroduces exactly the convention the search was meant to
remove. Heywood is semi-empirical; critical-distance methods are
material-dependent, which is refused elsewhere in this document. What `K_f` has
instead is corroboration across methods and decades — photoelastic (Jacobson,
1955), analytical (Chabert, Dang Tran and Mathis, 1972) and finite-element
(Wilcox and Coleman, 1973, "only a few percent different"). See
[`state.md`](state.md) for the full record, including that Dolan and Broghamer's
specimens contained no undercut teeth and this tool rates them.

`Y_B` de-rates a rim too thin to support its own tooth root, moving the failure
out of the fillet and through the rim — a failure mode nothing else here models
at all, so there is nothing for it to be balanced against. It is only read where
a designer gives a rim thickness, and a rim nobody described rates at 1.

### The helix factors are a pair, and this tool can take neither

ISO 6336-3:2019's Foreword lists what changed from the 2006 edition. The first
two entries are:

> — modification of the `Y_β` factor in Clause 8;
> — modification of the `Y_F` factor in 6.2.

**Together.** The revised `Y_F` gained an internal factor `f_ε` over the contact
and overlap ratios, and the revised `Y_β` gained a `1/cos³β`. One is `≤ 1` and
the other `≥ 1`, and they are two halves of one revision. Their product is the
net helical adjustment the standard actually applies:

| `ε_β` | `f_ε` | `Y_β` | product |
|---|---|---|---|
| 0,1 (β = 25°) | 0,975 | 1,315 | **1,283** |
| 0,6 (β = 25°) | 0,840 | 1,175 | **0,988** |
| 1,0 (β = 25°) | 0,715 | 1,063 | **0,760** |
| 1,0 (β = 30°) | 0,687 | 1,155 | **0,794** |

At **full axial overlap — what a helical gear is proportioned for** — ISO 2019
*lowers* a root stress by about 21–27 %. So this tool, applying neither, sits
1,26–1,36× a published ISO 2019 rating: conservative, by close to the "~25 %"
the documentation had claimed all along for a reason that turned out to be the
2006 formula.

**Applying `Y_β` alone was tried and reverted.** It looked like a correction —
the factor exceeds 1 over most of its own figure, so omitting it *reads* as
unconservative. Measured against the pair rather than against the half, it moved
the tool from 1,26–1,36× ISO to 1,29–1,46×: further from the standard, in the
name of getting closer to it. Three mixings, not one:

1. **Half a revised pair**, as above.
2. **`Y_F` here is not the `Y_F` `Y_β` corrects.** It is measured off the exact
   generated profile at an inscribed-parabola section; ISO's is a closed form at
   a 30° tangent section. A correction fitted to one base does not transfer to
   another.
3. **The virtual gear follows the other edition.** This crate uses
   `z_n = z/cos³β`; ISO 2019 uses `z_n = z/(cos²β_b · cos β)`, a 3,9 % different
   tooth count at β = 30°.

**Where this leaves the numbers, measured rather than asserted.** Against ISO
6336-3:2019, at `ε_α ≈ 1,65`:

- **`ε_β ≥ 1`**: this tool reads 1,26–1,36× ISO. Conservative, and that is the
  designed-for case.
- **`ε_β ≤ 0,3` with `β ≥ 20°`**: 0,78–0,94×. **Below** ISO — the one regime
  where the derived model is unconservative against the standard. It is already
  flagged: a helical stage without full axial overlap raises
  `mesh.overlap_below_one`, for a reason arrived at independently (a gear
  helical in form that still transfers load like a spur gear). That note is now
  also the marker for this.

**The rule this taught, which is the general one.** *A factor's direction is not
a property of the factor. It is a property of the set it was calibrated in.*
`Y_β > 1` is a true statement about `Y_β` and tells you nothing about whether
omitting it is safe. The only way to know is to multiply out every factor the
standard applies and compare against what this tool reports — which is a
measurement, and is now in `tools/` rather than in anyone's head.

**The policy is about factors that multiply a stress, not about conventions as
such.** A worm's length and a wormwheel's face width are shipped as
recommendations from DIN/ČSN and BS 721 with their sources on screen, because
neither enters any stress here — a point contact's peak pressure is bit-identical
at 4 mm and 40 mm of face width. A convention that cannot move an answer informs
a choice; one that multiplies a stress silently moves a number a part is sized
against.

**The axial compression term is applied**, internal and external alike. It is
the second term of the `J` whose first term is `Y_F` — Savage, Rubadeux & Coe's
`6h/t_c² − tan φ_C/t_c` — so it belongs to the model in use, and it relieved the
canary by 10.1 % and 12.2 %. ISO omits it, and the ISO comparison set omits it
too, so a number from that set is not an AGMA `J`. This paragraph said the
opposite for as long as the term was outstanding: its omission had been
defended as "the conservative direction", which is exactly the defence
[a conservative answer is not a free one](#a-conservative-answer-is-not-a-free-one)
refuses, and it was the debt `state.md` was carrying under that name.

**What would change this:** having ISO's `σ_Flim` values, so the complete set
would have something consistent to be measured against. They are paywalled, and
transcribing them into an open repository is a licensing question the JGMA
precedent does not settle.

### Load sharing is offered rather than assumed

`ContactPath::load_fraction` takes a `LoadSharing` model, and
`LoadSharing::LinearRamp` is an explicitly uncalibrated 1/3→2/3 ramp. It is a
**mesh input, off by default**, on every mesh that reports a bending stress,
and it reaches bending alone. It was the stage's, on the argument that a stage
running two meshes under two models would report a comparison rather than a
design; the default answers that better — every mesh starts at none, and a
mesh an edit adds takes the first mesh's — and a designer who sets two
differently has said so where they can see it.

**Off by default, because it is an estimate.** A calibrated mesh-stiffness model
would drag in tooth and rim stiffness, deflection under load and manufacturing
deviation — none of them available to a high-level design tool — and an
uncalibrated one produces confident numbers that are *worse* than a conservative
bound, because they look authoritative. So nothing is applied on a designer's
behalf.

**Offered rather than hidden, because that is what this project does with an
estimate.** Rules of thumb are kept as features a user switches on, not as
machinery buried where a number quietly depends on them. It had been reachable
only from a CLI diagnostic, which is neither.

**Bending only, and that is a property of the model rather than a scoping
decision.** A contact rating is already taken at the pitch point and the two
single-pair boundaries — precisely the places where one tooth carries
everything — so a sharing model cannot move a contact stress. Bending's worst
point is a *product*: the form factor grows toward the tip while the share falls
away there, so allowing sharing means sweeping the cycle for the maximum of the
two together instead of evaluating one point.

**Measured: 0.0–0.2 %** across every ordinary mesh tried. Once sharing is
allowed the governing point *becomes* the highest point of single-pair contact —
where the share is exactly 1 — so the answer is the one already reported, and the
expensive model buys almost nothing for a worst-case number. That is not an
approximate agreement below the band but an exact one: the single-pair boundary
is itself a candidate in the sweep, so where the maximum sits there the answer is
the unshared one to the bit. **A hula stage never leaves that regime** — its
meshes run just above continuous contact by construction — so the control is
offered there and provably cannot bite.

**One sweep, both kinds of member.** A ring's load point travels *up* in roll
away from its tip where an external tooth's travels down, and its flank stops at
the generation limit; both are the mesh kind's sign again rather than a second
construction ([a ring is a gear with a negative tooth
count](#a-ring-is-a-gear-with-a-negative-tooth-count)). A ring had no shared
section at all until they were one sweep, so a set that switched the model on
rated one member of an internal mesh under it and the other without — one mesh,
two answers, which is the fault this whole document is mostly about.

**Where it is not 0.2 %, and the disclosure that costs.** At a **virtual contact
ratio of 2 or more** there is no single-pair zone at all: two pairs are always
engaged, the ramp never reaches a full share, and it relieves the tooth by about
a third. That is a large number from an uncalibrated model in the
unconservative direction, so the stage reports `mesh.load_sharing_out_of_band`
beside the figure. It is not a hypothetical regime — a standard tooth cannot
reach it at any helix angle, but an ordinary **high-contact-ratio** design
(addendum 1.35) reaches it immediately, and that is exactly the design a user
would switch sharing on for.

**What would change this:** a calibrated stiffness model, which would replace the
ramp rather than the control. The two conditions that would make one worth having
are unchanged — a duty-cycle or transmission-error calculation, where the whole
mesh cycle matters rather than its worst instant, and the high-contact-ratio case
above, where the single-pair argument this rests on does not exist.

### Contact is one formula

`contact_stress` takes a lengthwise curvature; `PARALLEL_AXES` is a named zero,
and at it the elliptical patch's peak pressure is *exactly* zero. Line contact is
a degenerate value, not a branch.

**The formulation that stays unbranched is Carlson's.** The classical solution is
written with `K(e)` and `E(e)` and requires knowing which semi-axis is major —
itself a branch, and ill-conditioned as the ellipse degenerates. Carlson's
symmetric integrals make no distinction between the axes and are well conditioned
in exactly the limit that matters.

Two conditions on how the aspect ratio is posed, both consequences of the
degenerate limit being the case that matters: solve in `κ ∈ [0,1]` so parallel
axes is the *endpoint* rather than an infinity, and return zero pressure rather
than `NaN` there.

**The one genuine discontinuity is geometric, not elastic.** A real tooth has
finite face width, so an ellipse longer than the face is truncated by the tooth
rather than by elasticity. `σ_H = max(σ_elliptical, σ_line)` is exact at both
ends and the two cross once. Near that crossing the truth sits slightly above
both, since a truncated ellipse concentrates load more than a uniform line does.

**Acceptance was the existing model, unchanged**: `σ_H` against the
contact-half-width route to 1e-12, `ρ₁ + ρ₂` constant along the path,
independence from labelling, `σ_H ∝ √E*`, the helical ratio of exactly
`√(cos β_b)`, and `gear-cli strength 17 43 2.0` unchanged to the last digit.

### Torque, not force, is what a `Load` stores

Every force in a mesh is a projection, and a projection means nothing until you
say of what, onto which plane, at which radius. Four are in play and they differ
by `cos α_t`, `cos α_w` and `cos β_b`.

Storing any one of them bakes a choice of plane and radius into a bare number
that no longer records which it made. Torque does not: it is a property of the
shaft, invariant under every redefinition of a radius, and it is what the
specification takes in and reports out.

### Crossed axes are one model

A crossed-axis helical pair and a worm stage are both crossed-axis screw
gearing. A worm is a screw gear with very few starts and a high lead angle.

**A crossed gear pair is the spur stage with its shafts turned**, which is the
specification's own arrangement: `β₁ = Σ/2 + β_add`, `β₂ = Σ/2 − β_add`. And a
worm is that stage with its first member's size stated as a pitch diameter
rather than a helix angle — the same freedom read another way, `d = z m_n /
cos β`. One stage; what the worm preset adds is the words *starts* and
*wheel*, and the worm's conventional proportions
([one stage, one result](#one-stage-one-result)).

**Verified:** `sin γ = z m_n/d` holds on both members, so writing the wheel's
diameter as `z₂ m_n / sin γ₂` removes the axial module from the chain entirely
and the transmission ratio `z₂/z₁` *falls out* of the two diameters and two lead
angles rather than being imposed. Reproduced to 1e-9 over three tooth pairs ×
four shaft angles × three helix angles, with the worm canary bit-identical.

**Efficiency is one force balance, not a formula per axis angle.** It *contains*
both formulas it replaced: the classical screw one at the pitch point to 1e-12
including its exact self-locking threshold, and the parallel-axis loss integral
at `Σ → 0` to a hundredth of a point. That residual is the parallel formula's own
`O(μ²)` linearisation, identified by watching the gap fall linearly with `μ`
(0.0153 → 0.00047), not a defect in the balance.

**The worm is a ZI (involute helicoid).** ZA, ZN and ZI are one family — a
straight line under screw motion, differing only in where the line sits — so
supporting all three would be a parameter rather than a branch. What it would
cost is not the branch but the derivative: ZN and ZA are saddles with principal
directions rotated 58–77° from the ruling, and the analytic shortcut this crate
uses is available only for the developable one.

**Measured:** ZN comes out 1–15 % *below* ZI as the lead angle rises, so ZI is
the conservative reading. The type touches contact stress and nothing else — the
flank normal at the pitch point is exactly `α_n` for all three.

**The argument that settles it is conjugacy.** The wheel is a true involute
helical gear, and the involute helical gear's conjugate partner is the ZI worm.
A ZN worm meshing with an involute helical wheel is not exactly conjugate, so
adopting ZN would immediately raise "then what is the wheel?", which a crate
built entirely from the involute has no clean answer to.

**What would change this:** a user machining ZN on a conventional lathe wanting
the number for what they will actually make. `hertz::relative_curvatures`
already takes arbitrary per-body principal curvatures including negative ones, so
ZN would arrive as a different `(κ₁, κ₂, direction)` triple rather than a rework.

### A worm stage reports no bending stress

Not a gap. Three reasons that are differences in kind, not factors:

- **The tooth measured would not be the tooth loaded.** The bending method
  inscribes a parabola in *the profile this crate generates*. A worm wheel's
  tooth is the envelope of the worm thread: throated, curved along its length,
  with a section that changes across the face.
- **The load case differs in kind.** Parallel-axis bending puts the whole load at
  one tooth's HPSTC in the transverse plane. A worm mesh carries a point contact
  tracking diagonally across the flank with several pairs engaged.
- **There would be nothing to check it against.** Worm gearing's rating standards
  rate *durability*, not bending, so no published bending allowable exists for
  these materials and a user could not audit the number even in principle.

**And the contact path does not unblock it**, which an earlier audit predicted it
would. The path gives the load's position along the *profile*;
`σ_F = F_t/(b·m)·Y_F·K_f` is a cantilever loaded across its whole **face**, and a
crossed pair's load is a point. Choosing an effective width is exactly the sort
of convention that multiplies a stress. A concentrated load on a wide tooth is a
plate problem and the beam formula has no honest reading of it — which is why no
standard rates it analytically either.

What the stage reports instead is what a worm stage is actually limited by:
contact stress, sliding velocity, and mesh power loss in both directions. Worm
stages of this kind fail by wear and heat far more often than by tooth breakage.

### Two friction coefficients, because there are two questions

Whether a stage turns at all is decided at rest against a **static** coefficient;
how well it turns once moving is decided against the **sliding** one.
`Directional::once_moving` is the whole rule, and the static figure is never
itself reported — its only job is the sign.

Applied to every mesh although only a worm's is ever near its threshold, for
the same reason `PARALLEL_AXES` is a named zero: the rule is general and the
geometry decides whether it bites. The default worm stage is self-locking, which
is the answer a handbook gives.

**And applied to a path's whole flow, not mesh by mesh**, because a train can
hold at rest where none of its meshes does. Power that circulates multiplies
every mesh's loss: the compound preset back-driven passes fifty times its input
through its teeth, which leaves 48.5 % once it turns and −2.6 % at rest, where
static friction doubles every loss. So it cannot start, and its figure is
nought. The Wolfrom preset cannot be back-driven even running.

**The sign is the part that is easy to lose.** The flow counts only the power
that leaves, so a drive that cannot start reads as nought rather than below it,
and a test of whether nought is positive is decided by rounding. The compound
broke away by three parts in 10¹⁵ alone and after a spur, and did not after an
idler, until the flow read a power within its own zero as nought.
`tools/breakaway.py` keeps the sign, from a moment balance on every body, and
holds the crate's figures to it.

**A number quoted in a warning is the number the reader will go and change**, so
the self-locking note names the *static* coefficient — the one that actually
decides it.

### A constraint belongs to the mesh, not to the arrangement around it

A mesh whose teeth reach past the root circle they run into bottoms out whether
a carrier is turning about it or not. So **what is asked of a mesh is asked
once**, in one place, and every arrangement that builds one gets it —
`auto::MeshTrial`. What the shape owns is what it genuinely does own: which
meshes a candidate has, and how each is *assembled* — at a clearance-opened
axis distance, from a shaper cut, around a crank. Those are its mechanics.
What is asked of the result is not.

**Written per stage type, they disagreed.** Three types each spelled the
question out and between them answered it three ways: the parallel pair asked
whether its teeth bottom out and neither epicyclic type did; the pair and the
set asked whether each member could be cut as asked, and a ring was asked by
nobody. Each gap was invisible from inside the type that had it, because the
type that got it right was somewhere else.

**The tool is the parameter, not the branch.** A member arrives as a `Cut`: a
rack-generated one answers the four questions
[`auto::member_is_buildable`](#a-ring-is-asked-of-its-cutter-not-of-a-rack) asks,
a shaper-generated one answers of its cutter, and nothing above needs to know
which it is holding. The same discipline runs through the geometry it reads:
`Mesh::bottom_clearance` is one expression for an external and an internal mesh
with the sign doing the work, which is
[a ring is a gear with a negative tooth count](#a-ring-is-a-gear-with-a-negative-tooth-count)
read on radii.

**What this buys is extension without duplication.** A fourth arrangement adds
kinematics and the meshes those kinematics assemble; it inherits every question,
and a question added later reaches it without anybody remembering to go and look.

**What would change this:** a constraint that is genuinely the arrangement's
rather than the mesh's — a hula pair's tip margin at a *held* crank is one, and
it stays where it is for that reason. The test is whether the question can be
put to a mesh that does not know what is turning around it.

### A ring is asked of its cutter, not of a rack

A ring's flank, root and fillet are its **shaper's** rather than inputs of its
own, which is why it has no dedendum input and why
[`auto::member_is_buildable`] declines to ask it a rack's four questions. That
declining was read for a while as *a ring is not asked anything*, and it left the
one thing a search needs unasked.

**Two of the questions do carry over, unchanged.** A ring's *space* is where the
mating pinion's tooth goes and is generated the way a tooth is, so it takes the
identical expression — `m_t(π/2 + 2(x + x_s) tan α)` — and the same two guards on
it. So `admissible_profile_shift` already bounds a ring's shift and there was
never a second range to write. Reading a *rack's* bound onto a ring instead caps
it near 1.2 modules where an epicyclic set wants 1.9, which is the shift bound of
a tool that is not cutting it.

**What does not carry over is the round**, and that is the one to ask the cutter.
A rack-cut member is asked whether the fillet *it specifies* still fits; a ring
specifies none, so the question is whether the **tool left the shape the shift
asked for** — which the ring has already answered by the time any candidate
exists, in the clamps it records. Any of them means it did not.

**Measured, and it is the whole of why this matters.** Nothing asked, an
epicyclic set's search walked past the shift where its ring's space stops being
the space asked for: **26 of 30 sets** returned a ring the cutter had capped, one
of them at 2.35 modules against a cap of 1.94. The efficiency reported for those
is the efficiency of a part nobody makes.

**What would change this:** a second shaper-cut member. There is one rule and
one place, and the split between "asked of a rack" and "asked of a tool" is a
`match` on which tool cuts the member — the tool being the parameter rather than
the branch, which is what [find the parameter](#find-the-parameter-not-the-branch)
asks. A third kind would be the moment to make the round question itself take a
tool, so that the rack is the `z → ∞` case of it, as it already is in
[`crate::shaper`].

### A ring is a gear with a negative tooth count

Every internal *meshing* relation is the external one under that sign: the tooth
sums, the efficiency term, the operating radii, the relative curvature, the
contact path. `MeshKind` appears in arithmetic nowhere.

That is the same convention that lets Hertzian contact treat a concave body as a
negative radius, which is why `hertz.rs` never needed an internal case.

**But generation is genuinely a different construction** — a corner going round a
circle rather than along a line — and no amount of sign-juggling turns one into
the other. That is why `ring.rs` and `shaper.rs` exist rather than a flag on
`Gear`. The two halves pull opposite ways, and stating which is which is the
whole content of the distinction.

**On a ring it is the *space* that the external formulas describe**, not the
tooth. **Measured:** against tooth thicknesses at the operating circles, the space
reading gives exactly zero backlash at every `k` while the tooth reading is
0.63 mm out at `k = 1.2`.

**A shift is *where the tool sits*, for a shaper.** A rack's pitch line is a
machine setting so shifting it leaves the rolling alone; two pinions have their
ratio fixed by their tooth counts, so the pitch point moves with the centre
distance and the rolling circles with it. One factor `a/a_ref` carries all of it
and is exactly 1 at zero shift.

**A ring has no dedendum input and no root-radius coefficient.** Both are its
cutter's — the root circle is where the tool reaches, `a_cut + r_tip` exactly.
The linearised `r + m(h_f + x)` differs by 17 µm at `x = 0.25` and 57 µm at
`x = 0.5`, both well above the 3.6 µm the cut simulation resolves.

**`ρ_F` is a fillet property at any tooth size.** When the critical section climbs
onto the involute flank the notch is still the fillet, read at the junction.
Reading the involute's own curvature there is not a notch radius — it jumps
0.61 → 22.9 mm across one tooth.

**The fillet is not a radius.** Measured against a least-squares circle, the
best-fit `R/ρ` is 3.11…1.24 for a rack-cut external gear and 1.71…2.12 for a ring
on a 20-tooth shaper, departing from any circle by 26 µm down to 0.7 µm. At the
critical section the ratio to the tool's own round is 1.47–2.52 external and
3.55–4.26 on a ring, so using the tool's `ρ` in place of the generated fillet's
would inflate any notch factor by that factor. **Which point on the fillet is
read matters as much**, and for the same reason: junction against minimum is
another 1.4–6.3× (see [`state.md`](state.md)).

### An eccentric gear is an ordinary gear with `Δx = 0`

`eccentric.rs` assembles a gear tooth by tooth, and **every** gear in the crate
is drawn through it, on screen and in the DXF alike. A concentric one comes out
bit-identical to the z-fold replication it replaced and still generates one
tooth, not `z`.

**Per-tooth constant `x` is the specification, not an approximation of it.**
Constant ratio *requires* each driving flank to be a pure involute at a single
seat, which is exactly what the generator produces for one scalar `x`. So the
profile generator is unchanged; what this adds is assembly.

**The governing constraint** is eccentric body motion with a genuinely constant
transmission ratio. That single requirement determines the geometry: every
driving flank must be an involute of one base circle concentric with the rotation
axis, and those flanks must sit at exactly equal angular spacing. Nothing else is
constrained — tip radius only decides where the involute is truncated, tooth
thickness only decides backlash.

**Varying the addendum alone cannot work**, and that is measured, not argued:
addendum modification does not move the flanks at all. Both flank seats come out
identical to a standard gear — drive and coast pitch error exactly 0.000 µm — for
any `e`. It produces an eccentric *outer surface* on a mesh that is entirely
concentric.

**The unavoidable error, and why λ = 0 is optimal for a reversing drive.**
Profile shift moves a tooth's two flanks in opposite directions, so if both flank
sets were uniformly spaced every tooth's angular thickness would be the
difference of two constants — the same for every tooth. Uniform spacing on both
flanks forces uniform thickness. Two lines of algebra, and there is no clever
indexing that escapes it.

The drive-flank error then scales as `|1 − λ|` and the coast as `|1 + λ|`, so
minimising the worse of the two gives `min_λ max(|1−λ|, |1+λ|) = 1` at **λ = 0**.
Any compensation that improves one direction degrades the other by more than it
gains. **Measured** at z = 17, α = 20°, e = 0.25 mm: 62.6 µm both ways at λ = 0;
0.000 forward and 125.2 µm reversed at λ = 1.

**λ = 1 is not producible by radial hob motion alone** — it needs that motion
synchronised with a once-per-revolution differential rotation of the workpiece.
λ = 0 is exactly what the plain radial oscillation gives.

**Why the naive process fails, quantitatively.** One flank is generated over a
36.9° sweep of gear rotation — 1.7 tooth pitches — and `x(θ)` is itself changing
across it. Near the quadrature positions **63 % of the whole shift range occurs
within one tooth**. That is why E2 cannot be rescued by refining it: a faithful
simulation of the naive process would converge on flanks that are not involutes
at all.

**One hob, one setting.** An eccentric gear is assembled out of `Gear`s and a
`Gear` is a whole gear, so every guard in `Tooth::new` is a gear-level decision
being taken per tooth. Whatever is a property of the *tool* must be settled once,
by the tooth that demands most; whatever is a property of *one tooth* cannot be
shared away and must be reported.

**And the summary is one of the pieces.** `Gear::mean` is the gear every
scalar is quoted from and the one the root envelope is built on, so it is rebuilt
with the same tool.

**The mesh-phase coefficient is exactly half the backlash**, and being half of an
exact law it is exact. The drive and coast lines of action are mirror images
about the line of centres, and a change in axis distance is a displacement
along that mirror axis — so whatever gap it opens on one flank it opens equally
on the other.

**Gated twice, because half of an arithmetic identity is not evidence.** The
obvious acceptance test is met by construction, so it proves nothing. The real
one places the two **drawn outlines** at a axis distance and closes them until
they touch, once on each flank: the seated placement holds to 3.4e-16 rad across
`Δa` = 0.1, 0.3, 0.6 mm, and the play the drawn teeth leave converges on the law
from below.

**λ reaches none of the commanded axis distance.** The indexing offset moves a
tooth *rigidly*, so it decides when a tooth arrives and not how thick it is; zero
backlash is set by the thickness, which is the shift the tooth was cut at.
Asserted **exactly** — the profile is bit-identical at every λ — because "λ does
not reach this" is an invariant rather than a trend.

**A close tooth-count internal mate genuinely limits the eccentricity**, and the
model is right to refuse it: the shift term carries `1/Σz`, and for an internal
pair `Σz` is the tooth-count *difference*, so a 24-in-26 pair runs at a 1 mm
axis distance and even `Δx = 0.05` is 5 % of it.

**What would change this:** knowing what the mechanism can physically follow. If
it is a simple eccentric, `x(θ)` should be optimised against *that* constraint
rather than the ideal profile being reported with its residual. The residual is
reported because the mechanism is not yet chosen.

### A stage is rated where it runs

The zero-backlash axis distance is where the profile shifts put the pair; a
real one runs at that opened by its assembly clearance. Every contact quantity
belongs to the second, and only **backlash** keeps the design mesh, because it
measures play *against* the zero-backlash reference.

Rating at `a_w` was rating a pair nobody assembles, and the clearance is not a
detail to round away: it is the reason there is any backlash to report at all.

**"Opened" has a direction, and it is the mesh kind's.** An external pair's
flanks part as its centres separate; an internal pair's part as they come
together, the pinion moving out of the ring's teeth toward its centre. So a
clearance is `+c` on one kind's centres and `−c` on the other's —
`MeshKind::run_at`, [a ring is a gear with a negative tooth
count](#a-ring-is-a-gear-with-a-negative-tooth-count) read on the assembly —
and the backlash law carries the same sign, so play is positive on either kind
where the flanks have parted. **Measured, and it was live on every internal
mesh:** the law reported the interference a separation caused as play of the
same size, the two epicyclic stage types assembled their internal meshes a clearance
*tighter* than zero backlash and took the magnitude of that overlap as the play
they had, and the planetary set — whose two meshes share one physical distance
and so need their zero-backlash distances to differ by `2c` — solved them to be
equal, which is why its clearance never reached a shift. It was found from the
outside: a designer turning the clearance and watching no shift move, where the
arrangement says one must.

**And the rule was swept to the end this time.** A pair rated where it runs
from the day this entry was written; the epicyclic set and the hula stage went
on rating their meshes — path, stresses, efficiency, the interference verdicts
and the tip room — at zero backlash, which the shipped hula stage turned into a
crank held open until its tip margin was exactly nought and then run a clearance
inside it. Every stage rates where it runs now, and the shape sizes a distance
for the far-side gap *as built*, each distance's running clearance being
geometry the stage has to know (`DistanceReport::clearance`).

**What would change this:** nothing about the direction. If a fourth mesh kind
arrives whose flanks part some other way, it is a third value of the sign and
not a branch.

**Measured, all in the same direction because separating the centres can only
shorten the path:** spur `ε` 1.6211 → 1.6013, its `σ_F` 81.1 → 82.1 MPa, worm `η`
68.430 → 68.369 %, crossed `ε` at Σ = 90° 1.8506 → 1.8307. Bending rises because
a shorter path is less load sharing. Efficiency has **no fixed direction** — a
shorter zone is less sliding to pay for, but which end it loses decides whether
that helps — so only `ε` is asserted as a direction.

**The design fact this exposed:** at `Σ = 0.5°` with a 12 mm face and 20 µm of
clearance, `ε` falls from 1.664 to **0.860** and the zone is `Face`-limited. A
designer now gets *"these teeth will not touch as built"* where the model used to
report a healthy contact ratio for a mesh that was not happening.

### A load case is a torque, a port and a kind — and a train carries a list of them

A rating is a stress against an allowable, and the crate held two allowables from
the start — `ultimate_allowable` and `fatigue_allowable` — while rating
everything against the second. That is the wrong question asked confidently: a
peak load has to be survived *once*, and judging it against what the part must
survive forever fails gears that are fine and passes none that are not. So a
load case has a **kind**, and the kind is the allowable it is judged against —
plus the one thing that follows from it: an ultimate load is survived once and
counts no cycles, a fatigue load is spent over a duty and may reverse the roots.
Nothing else about a stage knows which kind it is looking at.

**The cases are a list, as the stages are.** The train used to hold exactly
two loads, and their ports and directions were written into the field names:
`input_torque` entered at the input and drove forward, `back_driving_torque`
entered at the output and drove backward, and the rating folded the two into a
"peak" that was the worse of them. That is the same idea written down twice
with a direction baked into each copy, and it is why a third load — a brake at
the motor holding an output load, a second motor duty — had nowhere to go. A
case is now a torque, a **port** it enters at, and whether the far end holds
it; direction is derived from the port and never stored, so a train with a
third entry point some day is a third value of `Port` and not a branch anywhere.

**What went with the pair.** The rule that "the peak is taken *after* each
direction's own distribution, never before it" — which
[direction is the reader's](#direction-is-the-readers-not-the-mechanisms)
records being got wrong in two stage types — is not generalised but **removed**:
two directions are two cases, each rated at its own torque in its own
direction, and there is no maximum left to take in the wrong order. The only
maxima that remain are in sizing, where an automatic face width answers to the
largest ask of every enabled case of a kind that is switched on — the highest
case sizes the part, however many overlap. The clamps went with it too: an
operating torque was held to the peak and a note said so, and there is no peak
to hold a case to now, cases being absolute and free to exceed one another.

**Every case is scaled at its own torque.** The stage solves its power flow
once at unit torque in each direction and scales it — a power flow being
linear in the torque through it — and every rating once at the largest torque
a mesh carries in any case, each case being that scaled. The latter is the same "second case costs a
multiply" the two-case model had, with the reference chosen so the shipped
trains reproduce the figures they had to the bit; and it is what lets a case
carrying nothing be a scale of zero where a flow solved at nothing would have
refused (`planetary::power` wants a driving input). A fresh train carries three
cases — the two loads and the operating duty it used to hold as fields, with
the same defaults — so nothing a designer had moves.

**Why a load is a torque and a speed, not a percentage.** The obvious control
is one "duty" slider at 80 %. It asserts that torque and speed fall together,
which an electric motor roughly obeys, efficiencies bend, and another power
source need not obey at all. This crate has no basis for that relation, so it
declines to assert it: each case states both, absolutely. Zero is admissible —
a load held still is a torque at no speed, and a train that only ever sees its
peak has no fatigue case.

**Why the sweep of an intermittent duty is measured at a named port.** The
alternative — at the load's own port — keeps one field fewer, but makes a 25°
sweep stated on a case at the start mean 25° of *motor* rotation, which is not
what anyone designing a 25° output mechanism types. The sweep is a fact about
the mechanism's motion and not about where its load enters, so it says which
port it is measured at, and defaults to the end — where it was always measured.

### A contact pressure is not a tensile stress

Peak contact is the one of the four ratings that is **off** by default. The other
three compare like with like: a root bending stress against a tensile allowable,
and a flank pressure against a fatigue figure that was derived for flanks.
Comparing a Hertzian pressure with the library's `ultimate_allowable` — a tensile
number — is arithmetic with no mechanism behind it. A flank under a single
overload fails by *subsurface shear*, which arrives at a contact pressure well
above the tensile ultimate, so the comparison is not merely unfounded, it is
unfounded in the conservative direction and would dominate a face width for no
reason a designer could defend.

It is offered rather than removed, because a designer who *has* a
contact-pressure limit can put it in the override and switch the rating on. What
the tool declines to do is assume one.

### One pressure, two ratings — and the curvature is not what separates them

Two teeth in mesh have very different flank curvatures, so it is natural to
expect them to carry different contact stresses for that reason. They do not, and
the reason is worth being exact about because the wrong mechanism leads to the
wrong fix.

Hertz reaches the contact through the **gap** between the surfaces, and the gap
depends on the individual radii only as `1/ρ = 1/ρ₁ + 1/ρ₂`. Each body is then
treated as an elastic half-space carrying that shared pressure, and a half-space
does not know its own curvature. So `ρ₁ = ρ₂ = 10` and `ρ₁ = 5.5, ρ₂ = 55` are
the same contact at the same pressure in both bodies — gated, because it is the
claim the whole reporting decision turns on. A small pinion tooth and a large
wheel tooth touching each other do not see different pressures for being
different sizes.

What genuinely separates them is **when each is rated**. Along the path `ρ₁ + ρ₂`
is constant, so `ρ` peaks where the two are equal and falls toward both ends —
and the two ends are not symmetric about that peak, so the two single-pair
boundaries carry different pressures. Pitting initiates in the dedendum, where
sliding opposes rolling; each gear's flank is at its root at one end of the path
and its tip at the other. So each gear is rated at the worse of the pitch point
and *its own* end. That is ISO 6336-2's `Z_B`/`Z_D`, arrived at by evaluating the
two points instead of quoting the factor — [no correction
factors](#no-isoagma-correction-factors) applies here as everywhere.

On the reference 17/43 pair the difference is not decorative: the pinion is rated
at 692.7 MPa and the wheel at 629.9 — the wheel's own boundary is *milder* than
the pitch point, so the pitch point governs it.

**This was reported wrong twice.** First as one figure duplicated onto both gears
— true of the pressure at an instant, but not what a gear is rated on. Then, on
being told the curvatures differ, briefly as a mesh-only figure with the gears
carrying none. The number that belongs to a gear was there the whole time; it was
the evaluation point, not the curvature.

### The narrower face carries the pair, so the automatic width answers to the mesh

Sizing each gear's automatic width from its own requirement is wrong in a way
that only shows once the two gears' requirements differ — which is exactly what
rating them at their own points, or giving them different materials, produces.
The mesh is carried at `min(b₁, b₂)`. A gear sized to its own smaller figure
therefore pulls the effective width, and the *other* gear with it, under what
that other gear required.

So an automatic width resolves to the largest ask any member of its mesh has, and
a member in two meshes answers to both. Each gear's four toggles still choose
which of *its own* ratings count; what they do not get to do is decide the width
on behalf of a gear that needs more. Gated as the invariant the control claims:
at an automatic width, every enabled rating is met.

### Direction is the reader's, not the mechanism's

A geartrain has no forward. It has bodies, teeth and losses, and which end a
designer calls the input is a label they bring to it. So **the reverse case is
never written a second time**: it is the same construction with the roles
swapped, evaluated again and *asserted*, and where the two answers differ — a
worm that self-locks, a set whose losses land on a different member — that
difference is an output rather than a branch.

`Directional::of(|d| …)` is the shape this takes almost everywhere: one
expression, evaluated at both directions, so a change reaches both by
construction. `Directional::once_moving` is the same idea one level up — every
kind asks whether the stage breaks away at all, and the geometry decides whether
it bites.

**Measured, and it is not a tidiness argument.** An epicyclic set solves its
power flow twice, once each way, because which body drives decides where `η₀`
multiplies. It then reported its members' back-driving torques by scaling the
*forward* distribution — the reverse solve's own torques were computed for their
efficiency and discarded — and the ring came out **6 % low**. Scaling is exact
wherever the forward torque is a geometric projection or the two directional
efficiencies agree, which is true of every parallel-axis pair and false of a set
and of a worm.

The check that separates them is the degenerate one: **at zero friction the two
directions distribute torque identically**, because there is no loss for a
direction to place. So a model in which they still differ is wrong, and a model
in which they *never* differ has thrown the direction away.

**A load case is a torque *and a direction*.** It follows, and it is where the
rule was hardest to see. While the train held two loads and rated a "peak"
that was the worse of driving and being driven, the maximum had to be taken
**after** each direction's own distribution rather than before it: collapsing
the two bodies' torques to one magnitude first and pushing that through the
forward construction is the same answer only where the distribution is
direction-independent — a parallel-axis mesh carries one tangential force
whichever way it turns — and every stage type for which it is *not* had this fault
in its ratings after the same fault had been corrected in its reports. A
back-driven worm was rated at `η_forward` of the load it was holding, and a
back-driven set's ring 6 % low in bending, which is the same 6 % the reported
torques had been. The rule is now unwritable rather than obeyed: a load from
each end is its own case, carrying its direction beside its torque
([a load case is a torque, a port and a kind](#a-load-case-is-a-torque-a-port-and-a-kind--and-a-train-carries-a-list-of-them)),
and there is no maximum across directions left to take in either order.

**And what a mesh carries belongs to the mesh, not the stage.** Two meshes of
one stage need not agree about which case loads them hardest, so a set's sun
mesh and ring mesh each scale from their own worst.

**Zero is a torque, as it is a speed.** A mechanism that is held rather than
driven runs at no load and still has to be rated for the peak it sees, and a
train may legitimately be non-forward drivable and only back-drivable. So no
construction here may assume a direction carries something: a rating at zero is
zero, not a refusal. The Hertz point solution refused a zero force where its own
limit is closed form, and took a worm stage's whole solve with it.

**What would change this:** nothing. Where a mechanism genuinely has a preferred
direction, that is a fact about its geometry — a worm's lead angle against its
friction — and shows up as an answer, not as an arm of a branch.

**This used to be one-sided, and the fix is what the rule asks for.**
`Directional::self_locking` read `backward <= 0.0`, so a stage that could not be
driven *forward* had no flag and no note — it was described only by a mesh
efficiency reading `0.0 %`, which reads as arithmetic rather than as a statement
about the mechanism. The case is reachable: `gear-cli crossed 17 23 90` at a
9°/81° helix split is exactly it.

It is `Directional::locked() -> Directional<bool>` now, and the *threshold* went
with it — [`Screw::locking_friction`] returns both, because both are one
construction with the members swapped: **the tangential force reaching the member
the power leaves by has fallen to zero**. Forwards that member is the wheel and
backwards it is the worm, and setting the relevant component of the flank balance
to zero gives each in closed form, with no bracketing. A *negative* threshold is
a value and not an absence — it says no friction locks the pair that way, which
is the ordinary case forwards.

The word "self-locking" survives where it belongs, in the catalogue: it is what
English calls `locked().backward` on a worm, and `mesh.forward_locking` is the
sentence for the other end. Naming a direction is the reader's business, which is
this section's whole point.

### A load exists only where it is reacted

A load entering at a port is not a sign on the other port's torque. It enters
at its own end, and the question it raises is not "how big is it" but "what
holds it".

The flow that answers it is the one a path's efficiency is read from
(`train::flow`): every mesh's driver is whichever side the power comes across
it from, its driven side carries the driver's torque under that direction's
`η`, and a mesh that cannot be driven that way — a self-locking worm from its
wheel, a crossed pair whose helix split cannot drive forward — **holds**: its
driver presses the flanks and nothing beyond it sees any, which is the whole
reason a designer puts a worm in a lifting drive. A crossed pair locked
forward used to be pushed through at an efficiency of nought or less and
arrive downstream as a torque of nothing with no word about why. The flow is
asked with the given torques known and everything else — the derived loads,
the reacted ends, ground — unknown, and where the given torques contradict
the statics (a load stated at each end that no mesh can hold between them,
the ordinary case of a load nothing reacts) the case reaches no number, which
the interface rule below says must be **said** rather than silently ignored
— so the case reports it by name and rates nothing.

**Why what reacts a load is the designer's and not the model's.** Whether a
port holds a load is a fact about what is connected there — a brake, a motor
with holding torque, a free body — and nothing in the geometry can know it.
The train used to decide it by direction: a load from the input was always
held (the output was assumed to be a load) and a load from the output never
was (the input was assumed free), which is the ordinary case written down as
the only case. A case says it now by what it loads: the chain's two ends are
reacted where the case does not load them, and every other open port — a
released ring, a layshaft's idler, a hula's wobble body — is free unless it
is loaded, since a reaction there is a thing a designer attaches and says so
by loading it. A port loaded with a torque of nought is a port turning and
carrying nothing, which is how the core is asked whether a stage locks; the
panel does not offer it, because relief takes a torque given past the statics
back, and the same question is on the stage card as its backward efficiency.

The graph refactor asked whether a reaction could be *derived* — "nothing
reacts it" as "no torque in the rowspace of the shaft line puts that load on
that body" — and the answer is that the law restates the declaration. With
a port allowed a torque, the rowspace supplies one; with it allowed none,
there is none; and whether it is allowed one is exactly what loading it says.
What *is* derivable is where the reaction lands once the ports are declared
— a self-locking stage holding it first, the reacted ends otherwise — and
every body's torque is reported per case so it can be read off.

**Why a two-pass solve.** A stage's torque depends on the ratio and efficiency
of every stage between it and the port, which are not known until those stages
are solved. Ratio and efficiency do not depend on torque, so the train is solved
once for the shaft line and again for the ratings. The second pass is not a
refinement of the first — it is the same arithmetic with the loads it was
missing. The first runs at a unit load rather than at none: an automatic face
width is sized from a rating, and a stage asked to rate nothing on a face of no
width has a `0/0` to refuse where the shaft line was all that was wanted.

### A reversed root is disclosed, and corrected only on request

A root loaded on **both** flanks endures less than one loaded on a single flank.
The usual allowance is a fraction on the allowable —
`REVERSED_BENDING_FRACTION`, 0.7 — and that is a convention which multiplies a
number a part is sized against, which is exactly what
[no ISO/AGMA correction factors](#no-isoagma-correction-factors) refuses to
apply on a designer's behalf. So it is a train-wide switch, **off by default**,
and where it is off the stage says which members the reversal reaches.

**Two things reverse a root, and they do not stack.** A planet always is loaded
both ways — the sun drives one flank and the ring the other, whatever the load
does — and every gear is in a fatigue case whose *duty* reverses, which is the
same flag that already splits that case's contact cycles between the two
flanks. `Reversal::reverses` takes the member's own answer or the case's, never
both, so one rule decides where a note can appear and where a derate can land;
the derate lands on that case's fatigue bending alone, and the note is raised
once on the member if any case reverses it.

**It was applied to the planet alone, silently.** A planetary set derated its
planet whether or not anyone had asked, while a reversing drive — an explicit
input, on a control that says it reverses the load — derated nothing anywhere.
One convention, applied in the place nobody chose it and absent from the place
they did.

**Bending only.** Pitting is compressive on whichever flank carries it, so a
contact rating keeps the material's own figure. The planet's contact width was
being sized against the derated allowable too, which is a bending allowance
reaching a rating it has nothing to say about.

**What would change this:** an allowable measured under fully reversed loading,
which would replace the fraction rather than the switch. The library has no such
column, and inventing one is the thing
[material data](#material-data-ships-estimates-deliberately) declines to do.

### Reversing changes the count, and which roots are reversed

A reversing duty changes no stress. What it changes is how many times each
thing is loaded, in two ways that pull opposite directions:

- **Bending rounds within one actuation, not once over all of them.** A tooth
  three quarters of the way through a sweep has still been loaded by that sweep,
  and every tooth must meet the worst actuation, not the average one.
- **Contact halves.** The two flanks share the engagements; the root takes all
  of them.

It is offered only for an intermittent duty, because only there is there an
actuation to reverse between — an input that would mean nothing in the other
mode is not offered in it — and only on a fatigue case, an ultimate load being
survived once.

**And it marks every root as reversed**, which is the one thing beyond the count
it decides — see
[a reversed root is disclosed](#a-reversed-root-is-disclosed-and-corrected-only-on-request).
It does not stack with a planet's own reversal: a planet's bending is fully
reversed whatever the duty does, since the sun loads one flank and the ring the
other, and counting that twice would be counting one fact twice.

### The tolerance table has two grade scales, not one

The natural rule — "the band with the smaller value wins, regardless of page" —
rests on a premise the data refuses: that these are overlapping bands of *one*
grade scale.

**Measured**, at module 1.0–1.6 and a 12 mm pitch diameter where both tables
apply: page 2's grade 4 is 7/20 µm against page 1's grade 4 at 22/71. Taking the
smaller value at each grade produces a ladder that **drops between grade 3 and
grade 4**. No rule for choosing between overlapping entries avoids this, because
the grade numbers do not denote the same thing on the two tables.

The standard's own annotation supports the reading: page 1's `1.0~1.6` column is
marked 選用 (*optional*) while the finer columns are 適用 (*applicable*).

So: two named scales, never compared, and the default is decided on scale and
grade ordering alone rather than on which entry yields the smaller value — which
keeps it predictable and independent of the table contents, so it survives the
addition of other standards.

**The standard itself is not in the repository, and the transcription is.** They
are different acts: a table of numbers read out of a document is not that
document, and this project needs the numbers to compute a tolerance while it
needs nothing from the pages but the reading. So `data/jgma_116_02.csv` ships
with its three transcription checks — row counts per grade, every value a
preferred number, monotone in grade within a band — and the copyrighted PDF that
was once beside it does not. This is the same line drawn in
[no ISO/AGMA correction factors](#no-isoagma-correction-factors): what stops
those being adopted is that the *values* are unavailable, not that the documents
are.

### Material data ships estimates, deliberately

This is a departure from the no-magic-numbers bar the rest of the project holds
to, and it is confined to material data — no geometry or solver takes an
estimated constant.

**The survey shaped the model, not the other way round.** Density, elastic
modulus and tensile strength are published for all eight materials. Poisson's
ratio is published for the steels and POM and by **no polyamide datasheet**.
Fatigue is published for the steels, is a printed *graph* for POM, and does not
exist at all for the polyamides.

**Two structural findings, not merely missing numbers:** glass-filled grades have
no yield point, so their datasheets report stress at break — `ultimate_measure`
records which. And `1215 Hardened Steel` is not metallurgically coherent: 1215 is
a ~0.09 %C resulphurised free-machining steel that cannot be through-hardened,
only carburised, giving a hard case over a soft core that one scalar cannot
represent. Both 1215 entries were dropped.

**A calculator with empty fields cannot produce a ballpark number**, and ballpark
numbers before refinement are the point of the tool. Three things keep it honest:
every value carries a `basis`; anything that is not a plain datasheet reading
must carry a note saying what it is, enforced by a test; and estimates are
**class-uniform**, so entries stay comparable even where the absolute value is a
guess.

**Measured, on how much the estimates cost:** `ν` enters Hertz only through
`(1−ν²)/E` and `σ_H ∝ √E*`, so the entire plausible polymer range `ν ∈ [0.33, 0.44]`
moves contact stress by **±2.5 %**. Fatigue is the opposite case — the
uncertainty is order-of-magnitude, and it is flagged as the weakest column.

**The S-N curve was withdrawn.** Fitting Basquin needs two points on a fatigue
curve, and those do not exist for six of the eight materials. A curve fitted to
invented points is worse than an honest scalar, because it looks like it knows
more than it does. Each material carries an ultimate and a fatigue allowable instead,
which are what an ultimate and a fatigue load case are judged against.

**No glass-filled POM, and that is a finding.** Delrin 570 is glass *filled* —
fibres added without effective coupling — so it is **25 % weaker** in tension than
unfilled Delrin while being 63 % stiffer, which for a tooth in bending is the
wrong trade. Glass *coupled* acetals are genuinely stronger, so one can be added
later provided it is a coupled grade. The distinction reverses the sign of the
strength change.

**What would change this:** ISO 6336-5 or VDI 2736-2, both paywalled, and the
latter covering only unfilled grades in any case.

### Equal planet load sharing is assumed

Real sets need a floating member, and the remedy is a mesh-load factor of exactly
the kind refused above. It is stated in the notes of every stage with a
replicated axis rather than left in a document.

### A rating that cannot be taken costs the rating, not the stage

A ring whose cut leaves no fillet has no notch, so no `ρ_f`, so no bending
number. Its geometry is not in doubt — it draws, it exports and it meshes — and
neither is anything else reported: the ratios, both contact stresses,
the efficiencies, the cycles, and the other members' bending are all still
answerable. So the missing input costs the one figure that needed it.

**The reachable case is ordinary, which is what settles it.** A planetary set
gives its ring `k = 2 − k_stage`, so a stage thickness modification of 1.4 puts
the ring at 0.6 — thick enough that the cutter which would leave its space comes
to a point before its own tip. Refusing the whole set there would throw away
nine sound figures over one absent one.

**The blank is readable**, which is the condition on doing this at all: the
member carries its own clamps, so `clamp.cutter_no_tip_corner` sits beside the
dash and says what is missing and that a cutter with more teeth relieves it. A
number becoming no number is a jump the interface has to explain, not one it may
leave to be guessed at.

**What this is not** is a licence to answer where the answer would be wrong.
Input that describes no shape at all is still refused — a cutter larger than the
ring it cuts, a module of zero, a body absorbing power. The distinction is the
one [clamp rather than refuse](#clamp-rather-than-refuse-and-say-so) already
draws: a rating is a question *about* a part, and a part can be perfectly real
while one question about it has no answer.

**Where the line falls, on the same stage.** A hula pair whose ring has no
fillet reports every figure but that ring's bending, exactly as above. A hula
pair with **no path of contact** is refused outright, and the difference is not
severity — it is that the second describes no mesh. A path is what a contact
stress, an efficiency and a contact ratio are all taken *on*; without one there
is no pair for the questions to be about, and the fixed-carrier efficiency that
would go into the power flow is a zero standing in for a number that does not
exist. The stage then reports an efficiency, a backlash and a set of speeds
computed from it, all of them meaningless and none of them saying so. That is
the failure this rule exists to prevent, not an example of it.

### A planetary needs the held body named

Three central bodies means naming two. The specification names one, which does not
determine an answer: a sun-driven set behaves quite differently with the ring
held than with the carrier held, and the two are not variants of one answer. This
is an **input** the specification omitted, and inventing a default for it would
be choosing a machine on the user's behalf.

It is the **train's** input now, not the set's: which body is held is a
constraint on a port, laid over what the stage holds *by convention* — the
ring, for a set, and the panel's select shows that hold as the choice it
is, so the default is named rather than invented. A convention is the
weakest statement there is and gives way to any statement of its kind about
the same stage: holding the carrier releases the ring without a word about
it. A statement the designer made does not give way — two holds on one set
lock it, and the train says which hold closed it rather than quietly
dropping one. That asymmetry is the whole of "relief over constraints": the
machinery that relieves a *number* exists because an automatic value has
nothing to say for itself, and a constraint a designer wrote has. Which
body is *driven* is no constraint at all: what drives a set is a load on
one of its open ports, and a load case says so.

### One stage, one result

A stage is a `Shape` — axes, the train's bodies on them, members on the
bodies, meshes between members, one distance per pair of axes that mesh — and its result is
one `ShapeResult`: a `GearResult` per member, a `MeshReport` per mesh, a
`DistanceReport` per distance, `MemberRating` over the meshes each member is
in. A spur pair, a worm and a planetary set are **lists of what sits
where** like every other arrangement — `arrangements::pair`, `::worm`,
`::planetary` — and every menu entry is a **preset** (`StagePreset`): one
of those lists at sensible teeth, under the family the shape reads as; a
worm's conventional proportions are one thing set on a distance
(`Distance::worm`). Each was a *vocabulary* first — a struct of the words a
designer uses, converted into the shape — and each carried a stage-level
module, pressure angle, overlap and contact-ratio floor that the shape had
already moved onto its members and its meshes, which is what decided it:
the second spelling could state what the first no longer could. The hula
stage went the same way, `arrangements::hula`, its corpus unmoved. None of them is a type in the core: a stage *is* a `Shape`. The
enum over it went too — one variant, forty-eight match arms and an
`as_shape()` that could not be `None`, buying a `kind = "shape"` a file
could only ever write one value of.

**It was not built this way, and the reasons it was not are the lessons.**
Each arrangement was a stage *type* with a result of its own, on the reading
that a crossed mesh has no bending stress and a set has a body that is not a
gear, so one shape would be a row of `Option`s. Three things were wrong with
that reading, and each is a rule now.

**A type is not a shape.** The worm was a stage type of its own — members that
were not gears, no shift, no addendum, a result unlike a pair's — on the
reading that a worm is a thread. In the model this crate actually runs both
flanks are involute helicoids on cylinders: a worm is a helical gear with a few
starts at a steep helix, its wheel a helical gear at the complementary one, and
everything a gear can be asked, both can be asked. Folding it into the pair
bought the worm a shift, an addendum, an interference check and a mode that
moves the wheel's shift as DIN 3975 has it — none of which the separate type
could carry — and a **crossed gear pair**, which had inherited the thread's
poverty, could say its flank had been eaten into where the same pinion with
parallel shafts already could ([corrections](corrections.md#the-log)). *That
is a gear* is not a qualification here: every member of every stage is a
`GearResult`, and `StageResult::members()` is the walk over them.

**A walk that names the arrangements forgets one.** A sweep over "every number
every member reports", written field-path by field-path across five types, can
only miss the type nobody named — which is how one of four expressions for a
back-driving torque came to be unbounded and stay so (F30, a self-locking
worm's wheel reporting 2.2e307 N·m). `members()` and `meshes()` are the
accessors that sweep needs, and the laws in `train/mod.rs`'s tests run over
every preset through them. The walk also found that **no quantitative law
crosses the arrangements**: a parallel-axis member's forward torque is a
projection with no efficiency in it, so its backward share is the same fraction
for both members; a crossed pair's output carries a forward efficiency the
backward load does not share, so its two members differ by exactly
`1/η_forward`. The invariant that does hold everywhere is the weaker one —
every member of a stage that reacts a load reports a share of it, finite, and
signed like its own torque — and it is the one asserted.

**A new arrangement is new kinematics and nothing else.** That was the claim
the division was to be judged by: a new arrangement should be how its bodies
relate, where its meshes sit and what carries what, with `MemberRating` and
`MeshReport` keyed on members and meshes rather than on named roles. It was
tested by building the shape and running a pair, a worm and every arrangement
of a set through it beside the types' own solves. Every figure but four
agreed, and each of the four was a fault in a type
([corrections](corrections.md#the-log)) rather than a difference of model — so
the types went. An epicyclic set is more reference frames than a pair, not a
different thing; a axis distance shared by several meshes is a layshaft's
question as much as a set's; what a type kept apart — three bodies here, two
there, a planet's own row — the shape reads off its graph, and a layshaft, a
Wolfrom, a stepped planet, a planocentric, a Ravigneaux and a hula stage are
lists of what sits where (`train/arrangements.rs`) with no code of their own.

**What would change it:** an arrangement the shape cannot lay out — a member
on two axes, a mesh that is not two members — is a change to `shape.rs`, and
the rule is that it is still not a type: the six questions a stage answers
are the whole of what it owes, and the shape answers them. (They were a
trait, `Constrained`, while there might have been a second implementor.
There was never one, so they are the shape's own methods.)

### Helical is not a lesser case

If a spur gear gets a number, the helical one does too — rated on its virtual
spur section, not refused and not rated transversely, which mixes planes and
under-predicts by about `cos β`. This is why `Ring::virtual_spur` and a
fractional-tooth-count `Ring` exist.

---

## The stack

Rust for all mathematics, compiled to WebAssembly; Svelte and TypeScript for
layout and event handling only.

The `[inputs are the only state](#inputs-are-the-only-state)` architecture makes the UI boundary almost nothing: because outputs are a
pure function of inputs, there is no state to synchronise, no lifecycle and no
callbacks. Given that, an intermediate native GUI would have meant writing the UI
twice and throwing one away — and the argument for it was native debugging, which
does not survive this architecture, since `gear-core` is a plain Rust library
debuggable through `cargo nextest` and a small CLI whatever sits on top.

A full geartrain solve is microseconds, so it runs on the main thread and
recomputes on input change. No worker, no async, no loading states.

Numerics are hand-rolled — about 120 lines of root finding — so they stay
auditable.

---

## The interface

### The language list is Rust's, and so is the tag matching

The catalogue crosses the boundary whole because a message split across two
repositories of text will disagree with itself. The same argument reaches one
step further than it first appears: the *list* of languages, and the rule
deciding which one a browser's `zh-TW` means, are facts about the catalogue —
so a front end that held either would have to be told separately every time a
file was added, and the half nobody tests is the half that forgets.

So `languages()` and `resolve_language()` cross too, and the front end asks
rather than knows.

**Names are in their own language, with English beside them.** A picker that
says only "German" and "Chinese (Traditional)" is a picker for people who
already read English, which is the one audience that does not need it. But a
reader who has landed in a script they cannot read needs a way *back*, and four
names none of which they recognise is not one — so the native name leads and the
English name follows in brackets, as the common index. English itself shows once,
not twice.

**A missing translation shows English, not a key.** `t()` renders an unknown key
as the key itself — deliberately, so a half-translated catalogue shows a reader
something they can report rather than swallowing the sentence that was warning
them. That is the right failure for a message nobody has written; it is the
wrong one for a message that exists in English and has not been translated yet,
where the English sentence carries the whole meaning. So a translated catalogue
is layered over English rather than replacing it. The safety net is not the
plan: a test holds every shipped file to English's exact key set, so a
translation that falls behind fails CI rather than quietly reverting.

**A fresh tab's name is a word the application chose, so it comes from the
catalogue too.** It stops being one the moment a reader types over it — after
that it is their document's name, travels in the exported TOML as whatever it
says, and does not follow a later change of language. Read once at creation for
exactly that reason.

**The preference outlives the session, and nothing else does.** "Inputs are the
only state" is about the model: everything that changes an answer is an input,
and every output is recomputed. A language is not an input — it changes no
number — so it is not bound by that rule, and a reader should not have to pick
their language again on every reload. It is stored in `localStorage`, with both
the read and the write guarded: a browser with site data blocked throws on the
accessor itself, and a language picker is not worth a blank page.

### Notes must not move the controls

Every field's note is rendered into a slot holding **all** the notes that field
could show, stacked in one grid cell with the inactive ones hidden. The slot is
then as tall as the tallest candidate at the current width, so a note appearing
shifts nothing below it, and a blank candidate reserves the space on a field that
has none.

Two properties make this worth the indirection over a written-down line count:
the browser does the measuring, so it stays right at any window width and cannot
be made stale by editing a note's text; and nothing is ever clipped.

**One builder and one component**, because two copies of a convention are a
convention that will eventually disagree with itself — and these had. A panel
decides *which* sentences a field has to offer; `notes.ts` and `FieldNote` decide
how they are shown. Where a note ends comes with the component for the same
reason: it is a property of the note rather than of whichever row it lands in,
and `grid-column: 1 / -1` is "the row's full width" in every template at once
rather than a column count each panel counts out for itself and has to recount
when a row grows a column.

**A readout's annotation is not a field's note.** The slot exists so that typing
does not move the page; a figure that cannot change while you look at it needs
none of that, and asking for it right-aligns the annotation away from the value
it belongs to and reserves a line for nothing.

**A mesh reports what a mesh has, and a stage what a stage has.** Efficiency and
backlash exist at both levels and are different quantities there — a pair's own
loss against what the arrangement does with it, a pair's own play against what
that play comes to at a body — so each is a row where it belongs and neither is
mentioned in the other's annotation. A stage whose efficiency note quoted the
meshes' product was answering, in small type beside the wrong number, a question
the mesh rows answer in full: the epicyclic set and the hula stage both did
it, and the stage's is the case that shows why it misleads, since the two figures
are 99 % and 27 % and only one of them is the stage's. Both directions are given at
both levels for the same reason they are given anywhere here — a stage that
cannot be back-driven says so by reporting the zero, not by omitting the column.

**Checked by measurement**, because screenshots are not pixel-deterministic here:
every control reports the same `getBoundingClientRect().top` with and without an
error note.

**The gap is what pairs a note to its field.** With one spacing above and below,
a note sits as near the next field as its own and reads as a heading for what
follows. Two values, `--note-gap` and `--field-gap`, are the whole of the fix,
and they live in the shared stylesheet because the pairing has to mean the same
thing wherever a note appears.

### A note lives with the thing it is about

Three things a solve can remark on, and the catalogue's sections are those
three: a **gear** (`[gear]` — a bound that moved its number, a root loaded
both ways, a rim too thin to rate, a face nothing sizes), a **mesh** (`[mesh]`
— contact that does not stay continuous, a helical pair short of full overlap,
a sharing model extrapolating, a screw pair that locks), and the **stage**
(`[stage]` — its distance, its search, its planets). Each is carried on the
result of the thing it names and drawn there: a gear's under the field it is
about or on its card, a mesh's beside the figure it is about, the stage's in
the stage's list. Nothing names what it is about, because where it is drawn
says so.

**Why it was worth a rename.** Nine of these were filed under `[stage]` and
carried a tooth count or a member name to be matched back up by — on a card
that already had the gear's name at the top. The contact-ratio finding was
three sentences: one stage note for a line contact, another for a point, and a
third, hand-written in the front end, drawn beside the row — so a reader was
told twice, in words that did not agree. The mesh notes are on `MeshReport`
now, which is what lets a set say *which* of its two meshes extrapolates, and
what made the two epicyclic stage types raise the contact-ratio findings at all: the
pair had asked both questions and neither of them had asked either.

**The self-locking sentence names no preset**, because the note is raised on
any crossed-axis mesh — a spur pair at a shaft angle as much as a worm — and
"the wheel cannot back-drive the worm" named parts that stage has none of.
"The second member cannot drive the first" is what `Directional::locked` says.

**What would change this:** a fourth thing to remark on — a body, say — which
would be a fourth section rather than a note on the nearest of these three.

### A readout does not repeat an input

An automatic input shows the value it resolved to, so a readout printing the
same number is the same figure twice on one page — and it was three times on
a pair, whose axis-distance row carried the running distance with the
nominal folded into an annotation, both of which the two inputs above it
already said between them. A set's row hung a "residual" off it that a solve
that closes always reports as nought and a solve that does not reports as a
failure. Both rows are gone.

**What is not a repeat** is a figure the input does not show: a worm's lead
angle, a hula mesh's far-side gap *as built* against the minimum it was asked
to keep. And **which figure the box shows is the same on every preset**: the
distance the stage runs at, the nominal in the annotation. The hula's was the
one exception, showing the zero-backlash offset and annotating the running one,
which read as a different kind of number from the box beside it.

**And turning automatic off keeps the number the box was showing.** `Auto`
holds its `manual` while `auto` is on so the field has something to fall back
to, and seeding it from the solved value is the front end's job — which the
geartrain panel was not doing, so a axis distance turned manual dropped to
the zero it was created with and the stage fell over. It is seeded to the
digits shown, so what the reader saw is what they now hold, the way the gear
tab's throw and amplitude already were.

**A control that is always an input says so by snapping back.** A set's
clearance and a hula stage's running clearance cannot be derived — the one is
what the two nominal distances differ by, the other is absorbed by shifts that
are all either given or the crank's — so their `auto` toggles are relieved
back to given at once, by the same relation the solve enforces, rather than
offered and silently disregarded. And a control that is never read says so
the same way: a crossed pair has no axial overlap, so its ratio's toggle is
relieved back to automatic — by the core, on any change to the stage, which
is why relief can be asked with nothing *just* touched. The panel used to
reset that toggle itself when the shaft angle moved, and it was the one
relief rule left written in TypeScript.

### What a stage owes relief

The freedoms machinery was measured by what one change cost it — the helix
becoming three readings on the members and the axial contact ratio arriving
as a fourth — and the answer was eight arms in a `match` over kind × freedom,
three copies of the same precedence chain, a count in one group hand-derived
from a toggle another group moves, and a walk that settled only because its
groups were written in a lucky order. None of it wrong, all of it the shape
that made the next stage type cost the same again.

So a stage answers six questions — the shape's own methods, once a trait
while a second implementor was still imaginable — and nothing else: which members it has; every input relief may turn, by name; how its
helix may be *stated* — the readings, in relief order; which of its inputs
argue with each other; where its bodies and meshes sit; and which of them a
train may address. Everything that walks those — counting, relieving, seeding
a box from what it showed, reading the helix the readings state, lining a
stage's inputs up against its result — is written once above the shape. A
member's inputs are resolved once for every member by `Freedom::Member(i, _)`,
so a per-member input that arrives is one line, not one per preset.

**The readings are one list, read from both ends.** Relief turns them
automatic least precious first, and the solve honours the *last* one given —
so the reading relief leaves standing is the reading the solve reads, by
construction, with no second chain of `if`s to keep in step. Before this the
two were stated separately, and on a pair they disagreed: relief kept the
second member's helix over the first's, and the solve read the first's.

**And a group's entry is one input, stated one or more ways.** A pair's size
is one freedom with three boxes, and the relation among distance, clearance,
shifts and size counts it once — given while any reading is. That replaces a
count trick (four of seven, held to one by a second group) and the
toggle-dependent limit it needed, and it is why the walk no longer depends on
the order its groups come in: it repeats until a pass moves nothing, and a
test holds that relief is idempotent from every start.

**The gate is asked of the solve, not of the declaration.** Everything
pinned, relief decides what may stand, and then each input left given is
nudged and the solved stage has to move. An input that stands given and
moves nothing is one the solve disregards — the very thing relief was written
to prevent — and only a test that runs both ends can see it. It is the test
that found the crossed pair's ratio standing given and read by nothing.

### Deleting the last of anything leaves a fresh one

A gear tab, a geartrain, and a stage of one: removing the last replaces it
with a default rather than refusing. The stage's button used to grey out at
one, which is a rule the reader has to infer from a control that stopped
working; a train with no stages is one the core refuses, and the honest thing
to do with that is not to arrive there.

**A load case is the one exception, because none is a state the core answers.**
A train with no load case is a shaft line — ratios, efficiencies and backlash
stand, every rating row stands empty, and an automatic face width with nothing
to ask stands at its box as it does with no source switched on. Nothing is
refused, so nothing has to be replaced, and the two buttons under the list are
how a case comes back.

### A control that exposes an assumption must not default to it

A stage's `working_depth` — the depth the undercut question is asked at — follows
its own dedendum rather than the classical 1 module. The whole point of the field
is that "17 teeth at 20°" answers *is it undercut within a module?* and not *is
it undercut at all?*, and it shipped defaulting to the first.

Following the dedendum also makes the automatic shift agree with the profile
generator's own `undercut` flag **by construction** rather than by coincidence.

### A member is adopted, not imported

The gear tab can take one member of an open geartrain as a new tab. It is the
same part described twice — a stage member is a `GearParams` with a rating
around it — so the tab should show the tooth the stage rated, and the way to
guarantee that is to hand over the parameters the stage *built* rather than
the ones it was *given*: `GearResult::params`, filled once where every member's
result is made, carrying the shift the stage chose, the addendum a tip
width held down, the helix shared out of a shaft angle with this member's
hand, a planet's `2 − k`. The tab solves those with no guard left to fire —
a test holds that nothing clamps — and quotes the stage's own pitch diameter.

Which members are rings is the stage's to say, not the tab's to infer from a
tooth count: `Shape::member_cutter` names the pinion cutter where there is
one, and a member with a cutter is internal and takes it. A worm is refused —
a thread's proportions are its own and the tab has no model of them — and the
list shows it greyed rather than omitting it, so a reader sees why.

The word is *adopt*. `import_train` reads a document this tool wrote, and the
two must not be confused in the code or the catalogue: one crosses a file
boundary and is checked for what a file can say that no stage honours; the
other reads a train that is open and cannot say anything of the kind.

One native `<select>`, grouped by geartrain in the sidebar's order with a
member per option, was chosen over a two-step menu because it holds no state
of its own: choosing acts at once and the control returns to its label. The
members are numbered as their cards are — and the cards' numbering used to
be `2 × stage + member`, right only while every earlier stage was a pair;
it counts the members the earlier stages have now, in one place both panels
read.

### A file is adjusted to what the tool can honour

A file can say what the panel cannot. The panel relieves a stage on every
change, so a crossed pair with its axial contact ratio given, or a pair with
its distance, both shifts and a helix all pinned, cannot be built there — but
a hand-edited document can say either, and reading it faithfully would put a
box on screen that stands given and is read by nothing, the exact state
relief exists to prevent.

So every stage a file describes is relieved on the way in, by the same
`relieved(None)` the panel asks after a shaft angle moves, and the reader is
told in one sentence — *the imported file has been adjusted to meet the
requirements of the tool* — where anything moved. Two things are fixed by
that sentence being the whole of it. **No value is changed**: a toggle the
stage cannot honour goes back to automatic and its number stays in the box,
so nothing the person typed is lost, only the claim that it was being read.
And **the adjustment is once, on the way in**: the document in the tab is one
the panel would have produced, and exporting it and reading it again adjusts
nothing, which the test holds. The precedent is for whatever else a file may
one day say that a stage has no use for: relieve it into what can be honoured,
keep the numbers, say so once, and never refuse a whole train over one toggle.

### An input that moves no number needs saying so

...and the fix is not always to hide it. A ring's profile shift box was connected
to nothing at all for a long time. A crossed pair looks like the same case — it is
solved at its pitch point, so shift, addendum, dedendum and root radius reach
none of its figures — but they are not meaningless: they are the tooth that will
be **cut**, and a designer specifying a crossed pair is specifying those parts.

So they are offered, and the panel says once, plainly, what they do and do not
reach. **The fault is the silence, not the field.**

### A hidden input is still an input

The gear tab's type-specific fields kept their values when the type changed, so
an eccentric gear switched back to external stayed eccentric with no control on
screen to say so. Two halves of one mistake: state that outlives its control, and
a readout that asks "is this non-zero?" where it meant "is this that kind of
thing?". Changing type returns every field the incoming type does not use to
its default.

### An input does not wait on an answer

A geartrain halfway through an edit is regularly one that cannot be built, so
the panel has to go on showing every control that produced it. It did not: the
readouts were guarded on the answer, and the guards had swallowed far more than
readouts — a stage that failed hid its material override boxes, its bound notes
and every figure's **label**, and the other stages went with it, because a train
is a chain and one failure leaves none of them solved.

Two things were being confused. What a control **is** — an input, and never
conditional on an answer — and what it **shows**, which may not exist yet. The
material properties are the clearest case: they were read off the result and are
in the library, so a stage that would not build withheld the very boxes a
designer would reach for to make it build.

So the rows stand and their figures are blank rather than absent, which is also
what stops the page changing shape at the moment it most needs to hold still —
the same reason a note's slot is reserved whether or not it has anything in it.
Every formatter answers `undefined` with a blank, so a readout is written once
and reads either way rather than being written twice.

**And a refusal is data.** `TrainError` has carried a `note()` since it was
written, whose keys the string tests check in both directions, but the boundary
handed over its `Display` — English prose written in Rust, which made the
failure the one thing the application said in a language nobody chose. It
crosses as a note and the stage it happened in, like every other message, and
only a boundary that actually broke is still thrown.

### A view preference belongs to the tab, not to the panel

The panel, and with it the canvas, is rebuilt whenever a reader looks at another
gear — so anything the drawing remembers about *being looked at* has to live
where the tab does, or a glance erases it. The zoom, the pan and the reference
circles on screen did not, and coming back to a gear found it framed from
scratch with the circles switched back on. Which stage of a train is expanded
had already been moved for exactly this reason; this is the same fact a second
time, now in one shape: a `GearView` beside `params`.

**Beside, not in.** A view is not an input. Nothing derived is stored and
nothing that leaves may carry one — a DXF is a drawing of a part and a train's
TOML is a description of a gearbox, and neither has an opinion about zoom. It
dies with the session, like every setting here that is not the language.

**Two boxes can carry one phrase and ask different questions.** "Reference
circles" under the canvas and "Include reference circles" in the export panel
are deliberately not wired together: one is what a reader is looking at, the
other is what a file contains. Answering both with one box would mean clearing
the screen quietly changed what an export contained.

### Unfinished work is knocked for, not switched on

The eccentric gear is derived, gated and drawn like everything else here, and
still no part cut from it has been measured — that is what "experimental" in its
own name has been carrying. But a type picker is read as a list of what the tool
does, and a reader opening it has no way to know that the third entry asks for
more trust than the other two. Naming the doubt in the option is asking the
reader to weigh it before they know what any of the words mean.

So it is not in the list until it is asked for: ten knocks on the application's
title in the sidebar, inside four seconds. Three properties, each deliberate.

**The knock has no answer.** No hover, no cursor change, no message, nothing in
the tab order, nothing announced. A control that says "you found something" is a
control being offered, and this is not on offer — what changes is that the
picker has a third entry. The two defaults that would have leaked it on their
own, the text caret and the selection ten clicks put on a word, are turned off in
the one stylesheet rule this costs.

**It only opens.** Knocking it shut again would leave a tab holding a kind the
picker no longer lists, which is the same fault as "A hidden input is still an
input" one level up: a value still doing something with no control on screen to
say so. A reload is the way out, and a reload already returns everything else
here to its default.

**It is not stored.** The mode is a session's, like every setting that is not
the language, so it cannot arrive with a link and cannot cross to the other copy
of the application in the same browser — the language is still the only thing
that does.

The kinds became a table to hold this, in the same shape and for the same reason
as the field list beside it: a kind's name, its note and whether it is offered
are one row rather than three places to keep in step, and the picker renders
whatever the row says.

**The geartrain's presets are the core's list, not a table of their own.** A
stage preset was three hand-written buttons and four hand-written accessors
for a default, then a row each in a TypeScript table; it is now a variant of
`StagePreset` in `gear-core`, which knows its family, the catalogue key of its
name and the shape it starts as, and `defaults()` crosses the whole list under
its three families — parallel axes, skew shafts, epicyclic — so the "add
stage" menu renders from the list and a preset added in the core is on the
menu by being on the list. The families are what a shape *reads as*
(`Shape::family`), never a stored kind: a spur pair is the epicyclic family
with its carrier held and no ring, and a crossed pair turned to nought is a
parallel one afterwards. A crossed pair *is* an entry now, and so is a worm,
for the menu's sake alone: each is a spur stage with something set on its
distance, and neither is an obvious thing to build from a pair, which is what
a preset is for. None is behind the developer knock; the knock keeps the
eccentric gear only.

### Additions to the specification's field list

Three things the specification does not list. Two are read-only outputs; the
third is an **input**, added because without it the specification does not
determine an answer.

| Where | What | Why |
|---|---|---|
| Stage, beside `Ratio` | contact ratios `ε_α`, `ε_β`, `ε_γ` | the spec has helix-angle inputs but no way to see whether they bought full axial overlap |
| Stage, per gear | provenance marker on each material property | the library ships estimates as well as measurements and must not present them alike |
| Planetary stage | **Held** — which body is grounded | the spec names only the driven one, which picks one of three and leaves the arrangement undetermined |

**One more input has two faces:** an eccentric gear's eccentricity can be entered
as the angular-shift amplitude or as the axis-distance offset. The second is the
first read backwards, and `Δx` stays the single field everything is built from —
the boundary resolves it once, so nothing downstream knows which face was shown.

Offered as the two fields they are, each with the `auto` toggle every derived
number in the application carries, and turning one on turns the other off. They
were a mode select and a field that appeared beneath it, which said the same
thing in two controls neither of which looked like the field it replaced, and
reported the solved amplitude a third time among the results.

---

## Testing

The prior work's central idea carries over intact:

> **Bound the profile from both sides.** Penetration alone is insufficient — an
> arbitrarily undersized profile passes it trivially. Only penetration *and*
> deviation together pin the profile down uniquely.

On top of that, in rough order of what has actually caught things:

1. **Verify against something that shares no code.** The rack simulation, the
   pin-tangency measurement, `ezdxf`, the contact-half-width route, the numerical
   average of instantaneous loss, the crossed path differentiated off the flanks
   rather than constructed, the ring cut simulated from the cutter alone. Every
   one of those caught something self-consistent tests had passed.

   **...and that does not repair what you got wrong.** A reader forgiving
   enough to be worth checking against is often forgiving enough to hide the
   fault: `ezdxf` builds a document, supplying from its own template every
   structure the file omitted, and then agreed the file was sound. What it
   agreed to, SOLIDWORKS refused. Ask what the independent tool *does* with a
   defect before trusting that it would show you one.
2. **Ask what property the answer must have.** The ring's flank disagreement was
   located by noticing the simulated envelope was *not an involute of the ring's
   base circle* — a property conjugate action guarantees, checkable without
   knowing the answer.
3. **Prefer laws to numbers.** "An internal mesh is less curved than the external
   pair of the same teeth", "a ring's tooth is the stronger", "friction never
   pays", "efficiency never exceeds one", "every length scales with the module".
   Each is checkable without knowing the answer, and each has caught something.
4. **Analytic cross-checks** against textbook special cases.
5. **Invariants**: thickness modification does not move the axis distance;
   `b_min` is independent of the `b` used; backlash is zero at nominal centres;
   `z_r = z_s + 2z_p` ⟹ `x_p = 0`.
6. **Regression fixtures**, pinned so refactors fail loudly — with the old bug
   retained as a *negative* fixture, proving the suite still detects it.
7. **Property tests**: random valid parameters must give a simple closed curve
   with monotone **radius**. Not monotone angle — undercut profiles are
   legitimately re-entrant, a misconception that cost the prior work 161 false
   failures.

**One grid, shared.** Three test files each had their own, and between them they
left six of a gear's eleven inputs at their defaults. An axis nobody turns is an
axis nobody tests.

**Before trusting a new gate, run it against the broken code.** `git worktree
add` a detached HEAD, copy the test in, and watch it fail. A gate that cannot
fail is not a gate.
