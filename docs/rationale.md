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
and a shaft with `T ω ≤ 0` is not an input. Both are refused.

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
cut, a module of zero, a shaft with `T ω ≤ 0`, two gears with different racks.

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

### Say what is not modelled, next to the number

A worm stage shows no bending stress and says why on screen. A ring shows the
smallest tooth count its design could have had. A planetary set states in its own
notes that it assumes the planets share load equally.

**And say nothing where nothing is known — but check that "nothing" is not
itself a discontinuity.** Refusing to answer looked like caution when the bending
correction hit a flank tangency. It was a cliff: a number becoming no number,
with nothing physical happening at that tooth count.

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

Nine scalar solves, each monotone, each bracketed, none an optimiser, none with
a tuning parameter. Everything else in the crate is algebraic.

| # | Solve | Method |
|---|---|---|
| 1 | `inv⁻¹` | series seed + safeguarded Newton, with a domain guard |
| 2 | Tip radius for a given tip width | Newton, analytic `ds/dr` |
| 3 | Flank/fillet junction when undercut | Brent, bracketed by construction |
| 4 | Planet shift for a common centre distance | Newton, closed-form bracket |
| 5 | 30° tangent critical section | Brent on the trochoid parameter |
| 5b | Inscribed-parabola critical section | Brent on the fillet, then on the flank's roll |
| 6 | Contact ellipse aspect ratio `κ` | Brent in `ln κ` |
| 7 | Cutter travel at a ring's flank/fillet junction | Brent on the trochoid's radius |
| 8 | Cutter travel where a ring's fillet reaches mid-space | Brent on the trochoid's angle |

The involute function is not algebraically invertible, and that single fact
causes #1, #2 and #4.

**#6 had a tempting alternative and refusing it is the same decision as the
rest.** Hertz's aspect-ratio relation has widely used closed forms —
Hamrock–Dowson's `κ ≈ 1.0339(R_y/R_x)^0.636` and its relatives — but they are
*fits* to the relation, not solutions of it. Taking one would put a fitted
exponent underneath every crossed-axis contact stress the tool reports. The solve
costs about fifty function evaluations and is exact.

**Guards matter as much as solvers.** Ordinary planetary inputs routinely request
a centre distance outside the involute domain, and the difference between a
guarded and unguarded `inv⁻¹` there is the difference between "this ring tooth
count is impossible" and a NaN silently reaching a stress figure.

**What would change this:** nothing in prospect. A published closed form that is
a *solution* rather than a fit would retire one of the nine; none is known.

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

**What would change this:** a calibration of `Y_S` against the parabola rather
than against the 30° construction. `Y_S` is an ISO fit written in terms of the
30° section, which is the one thing tying the two together.

### No ISO/AGMA correction factors

`Y_β`, `K_A`, `K_v`, `K_Fβ`/`K_Hβ`, `K_Fα`/`K_Hα`, `Z_ε`, `Z_β` and their
relatives are not used, and will not be added on request without revisiting this.

**Three reasons, in order of weight.**

1. **Their validated band is narrow relative to modern designs.** Each carries
   hard caps — `ε_β ≤ 1`, `β ≤ 30°`, a floor at `Y_β = 0.75`. Those are not
   physical thresholds; nothing changes in the mechanics at exactly `ε_β = 1`.
   They are the edges of the data. Outside the band the formula does not fail —
   it quietly returns the boundary value, which is the worst failure mode
   available.
2. **They are only balanced as a set.** `Y_β` reduces stress and `K_Fβ` raises
   it, and they describe the *same* face-width physics from opposite directions.
   Both were calibrated against `σ_Flim` values themselves back-derived using the
   whole set. Adopting one is taking the favourable half of a calibration.
3. **It trades accuracy for precision.** A number that is exactly right about a
   simpler question beats one that is approximately right about a harder one
   while hiding which question it answered.

**The evidence that these are empirical rather than derivable is direct:**
published comparisons against finite-element analysis find methods that disagree
on whether root stress *rises or falls* with helix angle — the sign of the trend,
not merely its size. There would be nothing to disagree about if it were
geometry.

**Where this leaves the numbers.** Bending here is conservative against a
published ISO rating by up to roughly 25 % at high helix and overlap. That is the
deliberate direction, and the tool should not be compared to an ISO rating
without saying so.

**`Y_S` is the deliberate exception**, and the distinction is worth stating
because it is easy to lump together. `Y_S ≥ 1`: it is the ratio of peak fillet
stress to nominal section stress, a *local* effect computed from `s_Fn`, `h_Fe`
and `ρ_F`, all measured off our own exact profile rather than looked up against a
gear population. Dropping it would not be conservative — it would report a
nominal stress roughly 1.6–2.1× below the real peak. Its notch parameter is
clamped into the fit's stated range and **reported raw**, because `Y_S` rises
with `q_s` and clamping a sharper-than-stated notch under-predicts stress.

**The policy is about factors that multiply a stress, not about conventions as
such.** A worm's length and a wormwheel's face width are shipped as
recommendations from DIN/ČSN and BS 721 with their sources on screen, because
neither enters any stress here — a point contact's peak pressure is bit-identical
at 4 mm and 40 mm of face width. A convention that cannot move an answer informs
a choice; one that multiplies a stress silently moves a number a part is sized
against.

**The axial compression term is omitted** from bending, internal and external
alike, following ISO rather than AGMA. It relieves stress by order 10 %, so
leaving it out is the conservative direction.

**What would change this:** having ISO's `σ_Flim` values, so the complete set
would have something consistent to be measured against. They are paywalled, and
transcribing them into an open repository is a licensing question the JGMA
precedent does not settle.

### Load sharing is deferred, and the reason is structural

`ContactPath::load_fraction` takes a `LoadSharing` model and
`LoadSharing::LinearRamp` exists as an explicitly uncalibrated 1/3→2/3 ramp. It
is labelled a placeholder, not a substitute for a stiffness model; its purpose
was to size the effect, and it has.

**Measured:** once sharing is allowed the governing point *becomes* the HPSTC. At
the tip a tooth carries roughly a third of the load, so tip loading stops
governing — and the worst surviving point of the cycle is exactly where the
model already places it. Sharing therefore lands within **0.0–0.2 %** across
every mesh tried.

So the expensive part — a calibrated mesh-stiffness model — buys almost nothing
for a worst-case number, while dragging in tooth and rim stiffness, deflection
under load and manufacturing deviation. Those inputs are not available to a
high-level design tool, and an uncalibrated stiffness model produces confident
numbers that are *worse* than the conservative bound, because they look
authoritative.

**Two conditions would change this, and both are worth naming:**
- **A duty-cycle or transmission-error calculation**, where the whole mesh cycle
  matters rather than its worst instant. Sharing is essential there and the
  0.2 % figure does not apply.
- **High contact ratio (`ε ≥ 2`)**, where two pairs are always engaged and the
  single-pair zone this argument rests on does not exist.

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

A crossed-axis helical pair and a worm drive are both crossed-axis screw
gearing. A worm is a screw gear with very few starts and a high lead angle.

**A crossed gear pair is the spur stage with its shafts turned**, which is the
specification's own arrangement: `β₁ = Σ/2 + β_add`, `β₂ = Σ/2 − β_add`. Three
stage kinds, not four. What still differs between a worm drive and a gear pair is
**one input**: whether the first member's diameter is given or derived from a
helix angle.

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
`σ_F = F_t/(b·m)·Y_F·Y_S` is a cantilever loaded across its whole **face**, and a
crossed pair's load is a point. Choosing an effective width is exactly the sort
of convention that multiplies a stress. A concentrated load on a wide tooth is a
plate problem and the beam formula has no honest reading of it — which is why no
standard rates it analytically either.

What the stage reports instead is what a worm drive is actually limited by:
contact stress, sliding velocity, and mesh power loss in both directions. Worm
drives fail by wear and heat far more often than by tooth breakage.

### Two friction coefficients, because there are two questions

Whether a drive turns at all is decided at rest against a **static** coefficient;
how well it turns once moving is decided against the **sliding** one.
`Directional::once_moving` is the whole rule, and the static figure is never
itself reported — its only job is the sign.

Applied to every stage kind although only a worm is ever near its threshold, for
the same reason `PARALLEL_AXES` is a named zero: the rule is general and the
geometry decides whether it bites. The default worm drive is self-locking, which
is the answer a handbook gives.

**A number quoted in a warning is the number the reader will go and change**, so
the self-locking note names the *static* coefficient — the one that actually
decides it.

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
3.55–4.26 on a ring, so using the tool's `ρ` would inflate `q_s` — and `Y_S` with
it — by that factor.

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
about the line of centres, and a change in centre distance is a displacement
along that mirror axis — so whatever gap it opens on one flank it opens equally
on the other.

**Gated twice, because half of an arithmetic identity is not evidence.** The
obvious acceptance test is met by construction, so it proves nothing. The real
one places the two **drawn outlines** at a centre distance and closes them until
they touch, once on each flank: the seated placement holds to 3.4e-16 rad across
`Δa` = 0.1, 0.3, 0.6 mm, and the play the drawn teeth leave converges on the law
from below.

**λ reaches none of the commanded centre distance.** The indexing offset moves a
tooth *rigidly*, so it decides when a tooth arrives and not how thick it is; zero
backlash is set by the thickness, which is the shift the tooth was cut at.
Asserted **exactly** — the profile is bit-identical at every λ — because "λ does
not reach this" is an invariant rather than a trend.

**A close tooth-count internal mate genuinely limits the eccentricity**, and the
model is right to refuse it: the shift term carries `1/Σz`, and for an internal
pair `Σz` is the tooth-count *difference*, so a 24-in-26 pair runs at a 1 mm
centre distance and even `Δx = 0.05` is 5 % of it.

**What would change this:** knowing what the mechanism can physically follow. If
it is a simple eccentric, `x(θ)` should be optimised against *that* constraint
rather than the ideal profile being reported with its residual. The residual is
reported because the mechanism is not yet chosen.

### A stage is rated where it runs

The zero-backlash centre distance is where the profile shifts put the pair; a
real one runs at that plus its assembly clearance. Every contact quantity belongs
to the second, and only **backlash** keeps the design mesh, because it measures
play *against* the zero-backlash reference.

Rating at `a_w` was rating a pair nobody assembles, and the clearance is not a
detail to round away: it is the reason there is any backlash to report at all.

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

### Two load cases, because there are two allowables

A rating is a stress against an allowable, and the crate held two allowables from
the start — `ultimate_allowable` and `fatigue_allowable` — while rating
everything against the second. That is the wrong question asked confidently: a
peak load has to be survived *once*, and judging it against what the part must
survive forever fails gears that are fine and passes none that are not.

So a rating is `LoadCase<T>`. What separates the two cases is exactly two things
— which torque, and which allowable — and nothing else about a stage knows which
one it is looking at. Both scale in closed form (bending linear in torque,
contact as its square root), so the second case costs a multiply rather than a
second solve, and `LoadCase::of` is the only constructor for the same reason
`Directional::of` is: there is no path by which a stage reports one case and not
the other.

**Why the operating load is a torque and a speed, not a percentage.** The
obvious control is one "duty" slider at 80 %. It asserts that torque and speed
fall together, which an electric motor roughly obeys, efficiencies bend, and
another power source need not obey at all. This crate has no basis for that
relation, so it declines to assert it: the user states each absolutely, each is
clamped to its own peak, and the *ratio between them* is reported as an output.
Zero is admissible — a train that only ever sees its peak has no cyclic case.

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

### A load exists only where it is reacted

A back-driving torque is not a sign on the input. It enters at the far end, and
the question it raises is not "how big is it" but "what holds it".

Walking upstream, each stage passes the load on attenuated by its **backward**
efficiency until one cannot be driven backward at all. That stage reacts it, and
everything above it carries none of it — which is the whole reason a designer
puts a worm in a lifting drive. If the walk reaches the input with the load still
turning something, then nothing reacted it: the train is back-drivable, the load
drives it, and the case is zero at every gear.

That last outcome is an input reaching no number, which the interface rule below
says must be **said** rather than silently ignored — so the train reports which
stage held the load, or that none did.

**Why a two-pass solve.** A stage's backward torque depends on every efficiency
downstream of it, which is not known until those stages are solved. Ratio and
efficiency do not depend on torque, so the train is solved once for the shaft
line and again for the ratings. The second pass is not a refinement of the
first — it is the same arithmetic with the load it was missing.

### Reversing changes the count, and only the count

A reversing drive changes no stress. What it changes is how many times each
thing is loaded, in two ways that pull opposite directions:

- **Bending rounds within one actuation, not once over all of them.** A tooth
  three quarters of the way through a sweep has still been loaded by that sweep,
  and every tooth must meet the worst actuation, not the average one.
- **Contact halves.** The two flanks share the engagements; the root takes all
  of them.

It is offered only for an intermittent drive, because only there is there an
actuation to reverse between — an input that would mean nothing in the other
mode is not offered in it.

**It does not stack with the planet's derate.** A planet's bending is fully
reversed whatever the drive does: the sun loads one flank and the ring the other.
Applying a reversing-drive derate on top would be counting one fact twice.

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
more than it does. Each material carries a peak and a cyclic allowable instead,
which pair with the peak and cyclic input torques the geartrain already takes.

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
the kind refused above. It is stated in every planetary result's notes rather
than left in a document.

### A planetary needs the held shaft named

Three shafts means naming two. The specification names one, which does not
determine an answer: a sun-driven set behaves quite differently with the ring
held than with the carrier held, and the two are not variants of one answer. This
is an **input** the specification omitted, and inventing a default for it would
be choosing a machine on the user's behalf.

### Each stage kind keeps its own result type

A worm stage has no bending stress; a planetary has three shafts, two meshes and
a planet that is neither. Forcing those into one shape would mean a row of
`Option`s and a comment apologising for each. What the kinds share is the
vocabulary — `Backlash`, `TrainError`, the duty cycle — not the shape of their
answers.

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

**Checked by measurement**, because screenshots are not pixel-deterministic here:
every control reports the same `getBoundingClientRect().top` with and without an
error note.

**The gap is what pairs a note to its field.** With one spacing above and below,
a note sits as near the next field as its own and reads as a heading for what
follows. Two values, `--note-gap` and `--field-gap`, are the whole of the fix,
and they live in the shared stylesheet because the pairing has to mean the same
thing wherever a note appears.

### A control that exposes an assumption must not default to it

A stage's `working_depth` — the depth the undercut question is asked at — follows
its own dedendum rather than the classical 1 module. The whole point of the field
is that "17 teeth at 20°" answers *is it undercut within a module?* and not *is
it undercut at all?*, and it shipped defaulting to the first.

Following the dedendum also makes the automatic shift agree with the profile
generator's own `undercut` flag **by construction** rather than by coincidence.

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

### Additions to the specification's field list

Three things the specification does not list. Two are read-only outputs; the
third is an **input**, added because without it the specification does not
determine an answer.

| Where | What | Why |
|---|---|---|
| Stage, beside `Ratio` | contact ratios `ε_α`, `ε_β`, `ε_γ` | the spec has helix-angle inputs but no way to see whether they bought full axial overlap |
| Stage, per gear | provenance marker on each material property | the library ships estimates as well as measurements and must not present them alike |
| Planetary stage | **Held** — which shaft is grounded | the spec names only the driven shaft, which picks one of three and leaves the arrangement undetermined |

**One more input has two faces:** an eccentric gear's eccentricity can be entered
as the angular-shift amplitude or as the centre-distance throw. The second is the
first read backwards, and `Δx` stays the single field everything is built from —
the boundary resolves it once, so nothing downstream knows which face was shown.

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
2. **Ask what property the answer must have.** The ring's flank disagreement was
   located by noticing the simulated envelope was *not an involute of the ring's
   base circle* — a property conjugate action guarantees, checkable without
   knowing the answer.
3. **Prefer laws to numbers.** "An internal mesh is less curved than the external
   pair of the same teeth", "a ring's tooth is the stronger", "friction never
   pays", "efficiency never exceeds one", "every length scales with the module".
   Each is checkable without knowing the answer, and each has caught something.
4. **Analytic cross-checks** against textbook special cases.
5. **Invariants**: thickness modification does not move the centre distance;
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
