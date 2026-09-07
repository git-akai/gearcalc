# Reference

What the tool computes, and by what relation. One entry per quantity: the
symbol, the formula, its units, its domain, and what comes back when there is no
answer.

**This document states; it does not argue.** Why a model was chosen over its
alternatives is [`rationale.md`](rationale.md); what was once wrong and how it
surfaced is [`corrections.md`](corrections.md); what is built and what is not is
[`state.md`](state.md). Nothing appears in two of them.

Conventions throughout: **degrees at the UI boundary, radians everywhere
inside**; lengths in mm, stresses in MPa, angles as stated. `m` is the **normal**
module unless subscripted `m_t`; subscript `n` is the normal plane and `t` the
transverse. A quantity that cannot exist is an `Option` or a `Result`, never a
NaN.

---

## Primitives

### The involute function

```text
inv α = tan α − α                    involute.rs
```

`inv⁻¹` has no closed form. Seeded from the series `tan α − α = α³/3 + 2α⁵/15 + …`
inverted to `α ≈ (3v)^⅓ − (2/5)v`, then refined by Newton with `d(inv α)/dα = tan²α`.
The refinement is **bracketed**: the bare scheme diverges above roughly 60°, which
is inside the pressure-angle range this tool allows.

Domain: `inv α ≥ 0` for `α ≥ 0`, so `inv⁻¹(v)` for `v < 0` returns `None`. That is
not a numerical failure — it means the requested centre distance is below what
the base circles permit, and planetary ring searches request it constantly.

### Root finding

Two solvers, both bracketed, so neither can diverge: a Newton step that leaves
the bracket is replaced by a bisection step. Convergence is tested against
`2ε|x| + x_tol`, relative so it means the same thing at every magnitude.
Exhausting the iteration bound returns `None` — a solve that did not converge is
not an answer.

Every transcendental step in the crate routes through these two. The complete
list of them is in [rationale.md](rationale.md#where-closed-form-is-impossible).

---

## The gear

### Rack and reference geometry

```text
m_t = m / cos β              tan α_t = tan α_n / cos β      plane.rs
r   = m_t z / 2              r_b     = r cos α_t
sin β_b = sin β · cos α_n                                   plane.rs
```

The two that carry an angle between the normal and transverse planes are
`crate::plane`'s, in one place: each was written out nine or ten times across
the crate, which is the shape of a defect this project has recorded more than
once. A spur gear is `β = 0`, where both reduce exactly.

### Tooth thickness and its equivalent shift

Thickness modification `k` is defined on the rack: tooth width `(π m/2)k`, space
width `(π m/2)(2−k)`, preserving the pitch. It is **exactly** an extra
thickness-only profile shift:

```text
x_s = π (k − 1) / (4 tan α_n)
s_n = m ( (π/2) k + 2 x tan α_n ) = m ( π/2 + 2(x + x_s) tan α_n )
```

identically, to 4e-16 over `α_n ∈ {14.5, 20, 25, 30}°`, `x ∈ {−0.5, 0, 0.7}`,
`k ∈ {0.6, 1.0, 1.45}`.

> **The rule this creates, used everywhere:** *radial* quantities — root radius,
> tip radius, cutter depth — take `x`. *Thickness* quantities take `x + x_s`.

Because a meshing pair requires `k₁ + k₂ = 2`, the `x_s` terms cancel and
thickness modification provably cannot move the centre distance.

**Cutter tip width**, in the normal plane so it is helix-independent:

```text
w_tip,n = π m − s_n − 2 m (h_f − x) tan α_n
```

### The generated profile

One half-tooth, from the tooth tip centre outward to mid tooth-space:

```text
tip arc  ->  involute flank  ->  trochoid fillet  ->  root arc
```

`θ` is measured from the **tooth centreline**: 0 at the centre, `π/z` at mid
space.

```text
flank      (r, θ) = ( r_b √(1+u²) , ψ_b − inv_from_roll(u) )      u = tan α_r
fillet     d = √(s² + b_c²)   k = 1 + ρ/d
           (x_f, y_f) = ( k s , r − k b_c )
           θ = atan2(x_f, y_f) − (s − a_c)/r
undercut   L = r sin α_t − b_c/sin α_t − ρ      undercut ⟺ L < 0
fillet cap ρ_max = w_tip cos α_t / (2(1 − sin α_t))
```

Two properties that must not be "simplified" away: the flank continues **below
the base circle** to its true intersection with the trochoid, and the fillet cap
is the expression above rather than the plausible `w_tip/(2 cos α_t)`.

**Severed teeth.** Where the fillet reaches the centreline the cutter has removed
the whole tooth. The profile is truncated there so it stays a simple closed
curve, `severed` is set, and `u_j` and `u_tip` become NaN — any code touching the
flank must check `severed` first.

**Homogeneity.** Every length is homogeneous of degree one in the module and
every angle is invariant. This is exact and is asserted over the parameter grid.
Roll parameters are *not* a good instrument for it: recovering `u` from a radius
near `u → 0` amplifies by `1/u²`, because the involute is tangent to its base
circle there.

### Sampling

Point spacing follows from a stated **chord tolerance** in mm rather than a
chosen count: a segment is split until its measured sagitta is inside tolerance.
The default is 1 µm, which is finer than the tightest tolerance JGMA 116-02
specifies for any gear.

Tip and root arcs are emitted as **true arcs** — a polyline vertex carries a
bulge of `tan(θ/4)` — so only the involute and the trochoid are approximated.
On an eccentric gear the root is not a circle and is subdivided like a flank.

### Input ranges

Every bound is closed form, and each sits exactly where the generator's own
guards begin to clamp.

| Input | Bound | From |
|---|---|---|
| Normal module | `m > 0` | every radius collapses at zero |
| Pressure angle | `0 < α < 90°` | `x_s → ∞` below, `r_b → 0` above |
| Tooth count | `z ≥ 1` | |
| Helix angle | `\|β\| < 90°` | `m_t → ∞` at the limit |
| Thickness mod. | `0 < k < 2` | a rack whose tooth or space has no width is not a rack |
| Profile shift | computed, below | |
| Addendum | `h_a > max(−h_f, (r_b − r)/m − x)` | tooth must have height; tip must clear the base circle |
| Dedendum | `−h_a < h_f < x + 0.9 r/m` | positive height; root circle off the axis |
| Root radius | `ρ ≤ 0.95 · min(b_d, ρ_max)/m_t` | the round must fit both the depth and the space |
| Angular shift | see below | one tool must reach every tooth |

**Profile shift.** The specification's `\|x\| ≤ 2` is wrong in three directions at
once, because every real bound depends on something a constant cannot see. Each
guard is linear in `x`, so the admissible interval is an intersection of
half-lines:

```text
thickness:  0.02 m ≤ s_t ≤ 0.95 π m_t     s_t = m(π/2 + 2(x + x_s) tan α_n)/cos β
depth:      m(h_shared − x_lo) ≤ 0.9 r    h_shared = max(h_f, 0.05 + x_hi)
```

where `[x_lo, x_hi]` is the interval the teeth are cut across — a single point
for a concentric gear. **Two tiers**, and they are not interchangeable: `min` and
`max` are degeneracy limits, and the design thresholds sit *inside* them:

| Threshold | Meaning |
|---|---|
| `undercut` | below this the flank is undercut at the stated working depth |
| `sharp_rack_undercut` | the same question asked of a sharp-cornered rack |
| `pointed` | above this the tip is capped and reported |
| `shallow_cut` | above this the cutter reaches deeper than the dedendum asked for |

**Angular shift.** The teeth share one hob, so its depth is set by the tooth that
needs most and then driven into the tooth cut at the smallest shift. Both demands
are linear in `Δx`, and the bound is the tighter of them:

```text
|Δx| ≤ min( (0.9 r/m − 0.05) / (1 − c) , (0.9 r/m − h_f + x̄) / (−c) )
c = cos(2π ⌊z/2⌋ / z)
```

What no per-gear range can express is `inv α_w ≥ 0`, a constraint on the **sum**
of both shifts in a pair. That is a mesh-level error.

### Automatic values

**Profile shift.** The smallest shift that avoids undercut down to a stated
working depth:

```text
x_min = h_w − [ ρ + sin α_t ( r sin α_t − ρ ) ] / m
```

Closed form and exactly invertible, because the undercut indicator is linear in
`x`. With `ρ = 0` it reduces to `x_min = h_w − z sin²α_t / 2`, so `x = 0` needs
`z ≥ 2 h_w / sin²α_t` — 18 teeth at one module of depth, 22 at a full standard
dedendum. The automatic value is `max(x_min, 0)`.

**Altered addendum**, from a minimum tip width. `s(r′) = 2r′(ψ_b − inv α_{r′})`
is monotone decreasing with `ds/dr′ = 2(ψ_b − inv α_{r′} − tan α_{r′})`, so it is
a bracketed Newton between `r_b` and the pointed-tooth radius.

---

## Meshing

### Centre distance and backlash

```text
inv α_w = inv α_t + 2 Σx tan α_n / Σz          Σx = x₁ + σ x₂,  Σz = z₁ + σ z₂
a_w     = a_ref cos α_t / cos α_w              a_ref = m_t |Σz| / 2
```

`σ = +1` external, `−1` internal. The sums are over the **thickness** shift
`x + x_s`, and only their ratio reaches `α_w`, so one expression covers both
kinds. `None` when `inv α_w < 0`: the base circles would have to overlap.

**Backlash is exact**, not the textbook first-order `j_t ≈ 2Δa tan α_w`:

```text
cos α′ = a_ref cos α_t / a′        j_t = 2 a′ ( inv α′ − inv α_w )
```

verified to 3e-16 mm against a direct computation of tooth thicknesses at the
operating pitch circles. It is zero at `a′ = a_w` by construction, and every
source of backlash — shift, thickness modification, clearance, tolerance —
enters through `α_w` and `α′` alone.

**One conversion, two numerators.** Angular play at a member is one sentence for
every mesh in the crate: a flank advances along the common normal by one normal
base pitch per tooth.

```text
j_θ,i = 2π j_n / (z_i p_bn)                    mesh::angular_play
```

The transverse form `j_t |Σz| / (a′ z_i)` is the same number in its own plane,
bridged by `cos α_t cos β_b = cos α_n cos β`. The **gap** is where the two mesh
kinds differ:

```text
parallel:  j_n = 2 a′ ( inv α′ − inv α_w ) cos α′ cos β_b
crossed:   j_n = j_axial sin β_b1 + 2 Δa sin α_n
```

A centre-distance error is a *separation* and opens both flanks, so it counts
twice; a worm's axial float is a rigid-body slide and counts once. `sin α_n` is
the contact normal's component along the line of centres at **every** shaft
angle — an identity, not a small-angle reading.

**Loaded flank phase.** How far a member turns when the centres move with one
flank kept in contact: exactly half the backlash, because a change in centre
distance is a displacement along the mirror axis of the two lines of action, so
it opens both flanks equally.

### The centre distance a pair runs at

`Mesh::a_w` is the **zero-backlash** distance. A real pair runs at that plus its
assembly clearance, and every contact quantity belongs to the second: the path,
the operating pressure angle, the operating radii, the relative curvature, the
stresses, the efficiency integral. Only **backlash** keeps the design mesh,
because it measures play against the zero-backlash reference.

| | how the distance enters |
|---|---|
| Parallel | `Mesh::at(a)` re-describes the pair: the line of action turns, `cos α′ = a_ref cos α_t / a′` |
| Crossed | `Screw::path_of_contact_at(…, a)` takes it: the line of action cannot turn, so it slides |

### Signed relations, both mesh kinds

Gear 2's tooth count, shift and radii carry the kind's sign, and that is the
whole of the difference:

```text
r_b2 = σ m_t z₂/2 cos α_t                      signed, hence concave
ρ₁   = r_b1 tan α_w + ξ      ρ₂ = r_b2 tan α_w − ξ
ρ₁ + ρ₂ = σ a_w sin α_w      1/ρ = 1/ρ₁ + 1/ρ₂
η term  = 1/z₁ + 1/(σ z₂)
```

`ξ` is measured from the pitch point, positive toward gear 1's tip.

### Path of contact and contact ratio

```text
recess    = T(r_a1, r_b1) − r′₁ sin α_w        T(r_a, r_b) = sgn(r_b)√(r_a² − r_b²)
approach  = T(r_a2, r_b2) − r′₂ sin α_w
ε_α       = (approach + recess) / p_bt         p_bt = π m_t cos α_t
ε_β       = b sin β / (π m)
ε_γ       = ε_α + ε_β
```

Each length is measured from the pitch point, so each subtracts its own gear's
share. Only the sum uses `a_w`, since `r′₁ + r′₂ = a_w`. With gear 2's radii
signed, one pair of expressions gives both kinds.

`ε_β` counts axial overlap the way `ε_α` counts profile overlap; spur gears have
`ε_β = 0` identically. It is a design check and enters no stress.

**Load point.** The highest point of single-pair contact, one base pitch along
from first contact. Measured from the tip it needs no mate:

```text
u_load = u_tip − (ε_α − 1) p_b / r_b
```

### Efficiency, parallel axes

At a contact point `ξ` from the pitch point the sliding velocity is `ξ(ω₁+ω₂)`
while the input power is `F_n v_b`, so the instantaneous fractional loss is
`μ|ξ|(1/r_b1 + 1/r_b2)`. Contact traverses the line of action at constant speed,
so the time average is uniform in `ξ`:

```text
η = 1 − μ π (1/z₁ ± 1/z₂) (ε₁² + ε₂²) / (ε_α cos β_b)
ε₁ = approach/p_bt      ε₂ = recess/p_bt      + external, − internal
```

Verified against a direct numerical average of the instantaneous loss over five
meshes at three helix angles each, to 1e-10 relative. `cos β_b` is exactly 1 at
zero helix, so the spur case is a value of this rather than a branch. The `/ε_α`
is load-sharing bookkeeping and holds the total transmitted force at `F_n`.

---

The loss integral is `∫|s| ds` along the path, `s` measured from the pitch point,
and it is written with the sign carried rather than squared away:

```text
loss ∝ (ε₁|ε₁| + ε₂|ε₂|) / ε_α          ε₁, ε₂ the two ends, in base pitches
```

For the familiar mesh the path straddles the pitch point, both ends are positive,
`ε|ε|` **is** `ε²`, and this is the classical expression to the last bit. A pair
can also touch entirely on one side — an internal pair at one tooth of difference
runs at an operating pressure angle high enough to put the pitch point outside
both tip circles — and there sliding never reverses along the path, so the
integral is the difference of the ends rather than their sum. One expression
covers both; the sign does the work.

**And the sum is not free for nothing either — an external pair loses least well
above its undercut floor.** The loss is an integral along the *whole* path, so a
longer path is a dearer one; positive shift shortens it, the contact ratio falls
toward unity, and the loss falls with it.

| pair | least loss | least shift that clears undercut |
|---|---|---|
| 9/37 | 97.62 % at `Σx = +1.20`, ε 1.36 | 96.83 % at `Σx = −0.65`, ε 1.65 |
| 17/43 | 98.47 % at `Σx = +1.25`, ε 1.48 | 96.87 % at `Σx = −1.20`, ε 3.04 |

The optimum is **interior** — stepping either shift further makes it worse — so
nothing holds it there but the loss turning over. This matters because the
automatic shift this crate has always offered is the *undercut* one, the least
that clears, and that is a **floor rather than an answer**: on 17/43 it gives up
1.6 points of mesh efficiency. A pinion small enough to need shift to exist does
not change the direction, only where the shift goes — at 9 teeth the floor pins
`x₁` at +0.50 and the optimum puts the rest on its wheel.

What it buys the efficiency with is contact ratio, 3.04 down to 1.48, and that is
a trade a designer may not want: fewer teeth sharing the load, and a noisier pair.
The tool reports both and decides neither.

**The division of a pair's shift is free, and the loss claims it.** Two shifts
reach the operating pressure angle only through their signed *sum*, so a centre
distance fixes the sum and leaves the division over. Nothing in the geometry
wants it — but moving shift from one member to the other lengthens one end of the
path and shortens the other, and the loss is an integral along that path.
Differentiating at fixed sum gives one equation:

```text
(2|ε₂|ε_α − N)/sin α_a1 = (2|ε₁|ε_α − N)/sin α_a2      N = ε₁|ε₁| + ε₂|ε₂|
```

Where the two tips sit at the same pressure angle it collapses to `ε₁ = ε₂` —
**balance approach against recess**, the rule the textbooks give, which is this
at equal tip angles rather than a separate law. The ends move at `1/sin α_a`,
which is why the tip angles are what it is written in: a tooth whose tip sits low
on its flank moves the path a long way for a little shift.

Gated against the loss rather than against the algebra: the condition changes
sign where a sweep of two hundred divisions finds the best one, on four pairs;
the division the solver returns beats every other tried; and a pair with nothing
to tell its members apart divides evenly. Both the textbook rule alone and the
same expression without its tip-angle weighting fail those.

**What it is worth depends entirely on what the mesh feeds.** On an ordinary
pair it is three to eight hundredths of a point — 98.32 % to 98.35 % on 17/43,
98.08 % to 98.16 % on 13/61 — which is real and small. On a drive whose reduction
comes from two meshes nearly cancelling it is worth thirty to fifty times that at
the output, because `η = 1/[R(1 − η₀) + η₀]` multiplies the mesh loss by the
reduction. That is the whole reason a high-ratio design bothers to optimise its
shifts, and why the same optimiser is unremarkable in a gearbox and decisive in a
Wolfrom.

---

**What a stage asks for, and what it may not do.** Every stage that has shifts to
choose carries one toggle. Off, the automatic shifts are the undercut minimum and
every answer is what it always was. On, they are chosen to lose least, and the
undercut shift becomes the floor.

What is already given constrains the search rather than being overruled by it:

| given | what it fixes |
|---|---|
| a profile shift | that gear's, exactly |
| a centre distance | the two shifts' signed *sum*, through `mesh::shift_sum_for` |
| a crank offset | the same, on each of the eccentric drive's two meshes |

A pair has two shifts to choose, so any two of `{a, x₁, x₂}` fix the third and
pinning all three is a contradiction rather than a tighter specification. The
front end relieves it visibly — the input furthest from what was just touched
returns to automatic — rather than accepting a number and disregarding it.

Two bounds have never had to bite before, because near zero shift they do not:

- **A contact ratio floor.** Loss falls monotonically with the length of the
  path, so the least-loss pair is always the one whose teeth barely reach and the
  floor is the answer rather than a guard. It is therefore a stage input. 1.2 is
  the usual design minimum for a pair; the eccentric drive defaults to continuous
  contact instead, because a mesh of one tooth of difference has so short a path
  that 1.2 would forbid the mechanism rather than constrain it.
- **Bottom clearance.** A tip that passes the mating root circle bottoms out. The
  dedendum already carries that gap — a standard 1.25 module against a 1 module
  addendum *is* the 0.25 of clearance — so the bound reads the clearance the
  designer specified rather than inventing an input for it.

Each stage differs only in what is free and what it is worth:

| stage | free | objective |
|---|---|---|
| spur | both shifts | the mesh's own efficiency |
| planetary | the sun's and the ring's, the planet's following | `η₀`, since `power` rises with it either way |
| eccentric | each mesh's division of its shift | the two meshes' product |
| worm | — | no profile shift exists to choose |

The searches share `auto::maximise`: what differs between stages is how many
numbers are free and what they are worth, not how to look for them, and
`auto::Freedoms` is the one mapping from what a search hands back onto the full
set. Each rates the geometry the stage would *build* — the automatic addendum
resolved, at the distance the pair runs at rather than its zero-backlash one —
so what is optimised is what is reported.

**The same four questions of every chosen shift** (`auto::member_is_buildable`):
the shift is at or above the least that clears undercut, the flank is not
undercut anyway, the tooth does not come to a point before its tip, and the root
round asked for still fits the space — which shrinks as the shift rises, since
the cutter bites less deep and the space narrows. A ring is not asked: its root
and fillet are its shaper's rather than inputs of its own, and an internal
mesh's bounds are the tip margin and the interference it reports, which belong
to the pair rather than to one member.

They were once asked stage by stage, which meant a bound reached the search it
was written in and no other: the root round bounded a pair and not an epicyclic
set, and the eccentric drive was choosing a pinion nobody could cut and taking
1.9 points of efficiency less for it.

**The first two of the four are a choice, and the other two are not.** Undercut
is a design decision — a designer entitled to an undercut tooth is entitled to
one — and being below the floor and being undercut are that single question
asked numerically and off the form, so they are relieved together. A severed
tooth and a root round that will not fit are shapes no cutter leaves, and
nothing relieves them. Which is why the bound is `Option`: `no undercut` off
passes `None` and the first two go unasked.

### Who decides a shift, and what it must satisfy

Every gear that is cut by a rack carries two controls, and they are different
kinds of thing. **`auto` is a source** — this shift is not given, the stage
decides it. **`no undercut` is a constraint** — whatever decides it, it may not
undercut. They combine rather than compete:

| `auto` | `no undercut` | the shift is |
|---|---|---|
| off | off | the number typed, exactly |
| off | on | that number, raised only if it genuinely undercuts |
| on | off | free: nothing asked of it, so zero unless a distance, an offset or the optimiser decides it |
| on | on | free and floored: the least that clears undercut where nothing else decides, which is what an automatic shift has always been |

Only shifts left automatic are the optimiser's to move; a given one constrains
it, as a given centre distance does.

**The bound is not one number, and that is not an inconsistency.** There are
three ways a shift arrives and each earns a different answer to the same
question (`train::undercut_bound`):

| how it arrived | bound | why |
|---|---|---|
| a search chose it | `max(x_min, 0)` | a chooser should not thin a tooth that needed no help |
| a designer gave it | none | it was held to `x_min` when it was read; re-judging it here rejects legal designs |
| a relation left it | `x_min` | nothing can move it, so the only honest question is whether it *does* undercut |

Choosing a shift and checking a given one want different answers to the same
question. The
true minimum is negative on any comfortable tooth count — −1.76 at `z = 43` —
so applying it to a *chooser* would thin a tooth that needed no help, for
nothing; a search is therefore floored at `max(x_min, 0)`, which is the
automatic value this crate has always used. A number a designer typed is held
to `x_min` itself, because a deliberate −0.3 on a 43-tooth wheel is a decision
about centre distance or balance and not a mistake about undercut. A shift that
*was* raised says so in a note, so the field and the gear never disagree in
silence.

That asymmetry is measured rather than argued: flooring the **search** at the
true minimum let the eccentric drive's split walk out to −1.79 and come back
with less drive efficiency than it started with.

A ring has neither control's second half: its flank is its shaper's rather than
a rack's, so it is given or it is the stage's to choose, and undercut is not a
question that can be asked of it.

**Where a clearance is read.** A clearance is taken by whatever is free to
absorb it, and each stage reports what it took rather than leaving a reader to
work it out. The centre distance absorbs it when the distance is automatic; the
shifts absorb it when they are being chosen, closing the pair to zero backlash a
clearance *inside* a given housing; and with neither free the input goes unread
and the answer says zero. The eccentric drive's minimum clearance is the same
question asked of its crank: it is what *sets* the offset, so a given offset
leaves it unread.

## Crossed axes

One model covers a worm drive and a crossed helical pair; they differ in **one
input**, whether the first member's diameter is given or derived from a helix
angle.

```text
sin γ = z m_n / d            exact, no iteration
γ = 90° − β    ⟹    sin γ = cos β
β₁ = Σ/2 + β_add      β₂ = Σ/2 − β_add      so β₁ + β₂ = Σ
```

Both ends of the range are refused and they are not symmetric: `sin γ ≥ 1` is a
member with no lead at all, and `β₁ = 90°` is a disc rather than a gear — caught
where the helix angle is still known, because `cos 90°` is 6e-17 and a derived
diameter would come out merely enormous.

### The path of contact

Built from two properties of an involute helicoid, both measured from the
surface's own parameterisation: its normal makes a fixed angle with its own axis
(`n̂·â = sin β_b`), and that normal is tangent to the base cylinder. At contact the
normal is shared, so those two conditions fix its **direction**; the contact
points then lie on the line with that direction tangent to both base cylinders.
Eight such lines exist and exactly two pass through the pitch point — the tooth's
two flanks, mirror images.

```text
r(s)   = √(r_b² + (ρ_n cos β_b)²)          ρ_n = |s − s_tangency|
zone   = both members with ρ_n ≤ √(r_a² − r_b²)/cos β_b
ε      = zone length / (π m_n cos α_n)     the NORMAL base pitch
travel = zone length · sin β_b             along each member's own axis
```

The parallel case is a **degeneracy**, not a value: at `Σ = 0` the two conditions
on `n̂` collapse into one, the line becomes a plane, and contact spreads from a
point to a line. `path_of_contact` returns `None` there.

Which of the eight lines is the mesh is settled once at the zero-backlash
distance and carried, since which flanks face each other is not a function of
centre distance.

### The friction balance

```text
F  = F_n (n̂ + μ v̂)              press along the normal, rub along the slip
T₁ = (r × F)·â₁                 moments about each axis; F_n cancels in the ratio
T₂ = ((r − o₂) × F)·â₂
η  = T₂ ω₂ / T₁ ω₁
```

Nothing about the kinematics is told to it: the speed ratio falls out of the
surfaces neither separating nor overlapping (`v₁·n̂ = v₂·n̂`) as `−z₁/z₂` to nine
digits at every point of the path. Efficiency is the average along the real path
of contact, in both directions.

At the pitch point this **is** the classical screw formula to 1e-12:

```text
η_forward  = (cos α_n − μ tan γ₁) / (cos α_n + μ cot γ₁)
η_backward = (cos α_n − μ cot γ₁) / (cos α_n + μ tan γ₁)
self-locking ⟺ μ ≥ cos α_n tan γ
```

**Two friction coefficients.** Whether a drive turns at all is decided at rest
against a **static** coefficient; how well it turns once moving is decided
against the **sliding** one. `Directional::once_moving` is the whole rule, and
the static figure is never itself reported.

### Face width from continuity

A crossed pair has no stress that depends on its face width, so nothing about
strength can size it. What it has instead is a contact point that runs off the
end of a face too narrow:

```text
ε = 1   ⟺   b = 2 B sin β_b            B the half-span the zone needs
```

closed form in three cases, because the zone grows with the half-span at slope 2
while both ends are the face's, at slope 1 once one end has reached the teeth,
and at slope 0 once both have. The face is centred on its **gear**, not on the
mesh: `CrossedPath::axial_centre` is where each member's mid-plane meets the
path.

### Worm proportions

Shipped as recommendations with their sources named, in the **axial** module:

```text
b₁ = (11 + c z₂) m_x,   c = 0.06 (z₁ < 4), 0.09 (z₁ ≥ 4)      DIN/ČSN practice
b₂ = 2 m_x √(q + 1),    capped at 0.67 d₁,   q = d₁/m_x       BS 721
```

Not offered for a crossed gear pair, which has nothing wrapped round anything.

The worm's flank is taken as an **involute helicoid (ZI)**, which makes it
developable: one principal curvature is exactly zero along the ruling, so each
flank is locally a cylinder. A ZN worm's contact stress comes out 1–15 % below
the reported figure as the lead angle rises.

---

## Contact stress

General Hertzian contact, of which line contact is a limit. Two bodies touching
at a point are two quadratic surfaces; their gap is `h = x²/(2R_x) + y²/(2R_y)`
in the principal directions.

```text
1/E* = (1−ν₁²)/E₁ + (1−ν₂²)/E₂
```

In Carlson form there is **no major-axis branch** — the two elastic conditions
are the same expression with the arguments exchanged:

```text
1/(2R_x) = (p₀ a b / 3E*) R_D(b², 0, a²)
1/(2R_y) = (p₀ a b / 3E*) R_D(a², 0, b²)
```

Dividing one by the other removes the load, the moduli and the size, leaving the
aspect ratio `κ = b/a` fixed implicitly by the ratio of curvatures alone. That is
solved in `ln κ`, so the tolerance is relative and the line-contact limit at
`κ → 0` stays reachable.

**Line contact is the degenerate value.** At `1/R_x = 0` the ellipse is
infinitely long and a finite load over it gives exactly zero peak pressure, so

```text
σ_H = max( σ_elliptical , σ_line )              hertz::peak_pressure
σ_line = √( (F_n/L) (1/ρ₁ ± 1/ρ₂) E* / π )      L the contact line the teeth have
```

collapses to the line term for every parallel-axis mesh, without a branch.
`PARALLEL_AXES` is the named zero.

**The `max` is where the bodies take over from the elasticity, and it belongs to
both mesh kinds.** The elliptical solution assumes half-spaces of unlimited
extent; a real tooth's contact line runs out at the face, `L = b / cos β_b`. So
once the ellipse is longer than `L` the load is carried on the length that
exists. The two cross exactly once and the larger is physical on each side of the
crossing — near it the truth sits slightly above both, since a truncated ellipse
concentrates load more than a uniform line does.

A crossed pair needs it as much as a parallel one and for the opposite reason:
its ellipse *lengthens* as the shafts come parallel, so at small shaft angles the
line term governs and at a worm's right angle the elliptical one does. One
function answers both, and a patch is reported no longer than the teeth carrying
it.

**Both degenerate ends are values.** A zero lengthwise curvature presses with
exactly zero; a zero contact line presses with an unbounded pressure, which is
what a face width of zero means. Only a zero load on a zero line has no answer.

**Which points are checked.** Since `ρ₁ + ρ₂` is constant along the path, the
relative radius peaks where the two are equal and falls away toward **both**
ends. That balance point swaps sides with the labelling, so **both** single-pair
boundaries are evaluated and the answer does not depend on which gear is called
gear 1.

**Helical: three plane changes, and they nearly cancel.**

```text
ρ_n = ρ_t / cos β_b        F_bn = F_bt / cos β_b        L = b / cos β_b
⟹ σ_H = √( (F_bt/b) cos β_b / ρ_t · E*/π )
```

so a helical mesh comes out below the same transverse geometry by exactly
`√(cos β_b)` — 3 % at β = 20°.

---

## Bending

The form factor is **measured off the profile this crate generates**, not looked
up. Undercut, profile shift and thickness modification are then handled because
they change the profile.

**Critical section: the Lewis parabola.** A cantilever whose outline is a
parabola with its vertex at the load carries uniform bending stress, so the
largest such parabola inscribed in the tooth touches where the tooth is weakest.
`CriticalSection::TangentAngle` retains the 30° tangent for a
standards-comparable number.

```text
σ_F = F_t / (b · m) · Y_F · Y_S
q_s = s_Fn / (2 ρ_F)
```

The `Y_S` fit is stated over `1 ≤ q_s < 8`. Outside it the correction is taken
at the boundary and the stage **says so**, naming the member and the value —
above the band that under-predicts, which is the unconservative direction.

`ρ_F` is a **fillet** property at any tooth size: when the critical section
climbs onto the involute flank the notch is still the fillet, read at the
junction. Its curvature is closed form —

```text
P′ = R ( q′ + φ′ J q )        P″ = R ( q″ + 2φ′ J q′ − φ′² q )
```

for a curve `q(s)` carried by a frame turning uniformly at `φ′`, which covers the
rack's corner running along a line and the shaper's running round a circle with
one expression. `R` is a rotation, so the curvature can be read entirely in the
moving frame.

**Helical: two corrections, pulling opposite ways.**

```text
z_n  = z / cos³β             the virtual spur gear's tooth count
ε_αn = ε_α / cos²β_b         where the load sits on it
```

One power of `cos β` from the oblique section and two from the curvature of the
pitch ellipse it cuts; then one from the base pitch and one from the path
length. At `β = 0` both reduce exactly and the virtual gear is rebuilt bit for
bit identical, so there is no spur branch anywhere in the strength path.

**Minimum face width**, closed form, since `σ_F ∝ 1/b` and `σ_H ∝ 1/√b`:

```text
b_min,bending = b σ_F / σ_allow          b_min,contact = b (σ_H / σ_allow)²
```

independent of the `b` it was evaluated at.

---

## Metrology

### Span over teeth

Derived rather than quoted: the span is a chord along the base tangent, so it is
`(k−1)` base pitches plus one base tooth thickness.

```text
W_k = cos β_b · r_b [ 2π(k−1)/z + s_t/r + 2 inv α_t ]
```

which reduces exactly to `W_k = m cos α_n [π(k−0.5) + z inv α_t] + 2 x_thick m sin α_n`
for the standard rack — note `x_thick`, since a span is a thickness measurement.
`k` is chosen from the exact admissible range (both contact points between form
and tip radius), picking the one nearest the pitch circle; it reports "no valid
span" rather than an unmeasurable number.

### Over pins, and between pins for a ring

One relation at two signs. `σ = +1` external, `−1` internal:

```text
inv φ_M = σ ( ψ_b + d_p / (2 r_b cos β_b) − π/z )
u_c     = tan φ_M − σ d_p / (2 r_b)
r_M     = r_b / cos φ_M
M       = across − σ d_p        across = 2 r_M, or 2 r_M cos(π/2z) if z is odd
```

Every sign says something physical: an external gear's space narrows outward so a
larger pin rides higher, a ring's narrows inward so a larger pin sits deeper; the
pin diameter adds outside and subtracts inside; and the same arithmetic failure
means "too small" outside and "too large" inside, so the error is chosen by `σ`
too.

| | **z even** | **z odd** |
|---|---|---|
| **2 pins** | `2 r_M + d_p` | `2 r_M cos(π/2z) + d_p` |
| **3 pins** | `2 r_M cos(π/z) + d_p` | `r_M (1 + cos(π/z)) + d_p` |

Validity, all closed form: `r_form < r_b/cos φ < r_a` and `r_M − d_p/2 > r_f`.
Three pins are external only — inside a bore neither the odd-count problem nor
the datum problem arises.

**The four cases above are one measurement over different seats.** The pins are
equal circles at known places and the caliper reads the distance between two
parallel planes touching them, so:

```text
2 pins    |P_a − P_b| + d_p                          b the space nearest half a turn
3 pins    |P_1·n - P_3·n| + d_p    n perpendicular to (P_2 − P_1)
```

Both parities are values of that, and the published forms come back bit for bit.
Written this way it needs no seat to equal any other, which is what lets it
measure a gear whose teeth differ.

### Inspection data around the revolution

Every measurement is made of **flank seats** — where a flank's involute begins on
the base circle:

```text
flank(k, ±1) = seat_k ± ψ_b,k        seat_k = 2πk/z + λ(ψ̄ − ψ_k)
```

A span reads the distance between two of them; a pin sits between two. An evenly
cut gear has one `ψ_b` and evenly spaced seats, so both collapse to a formula in
`z` and that one angle — which is what the published forms are.

```text
span      W  = r_b cos β_b [ 2π(k−1)/z + (1+λ) ψ_j + (1−λ) ψ_{j+k−1} ]
space     h_k = [ 2π/z + λ(ψ_k − ψ_{k+1}) − ψ_k − ψ_{k+1} ] / 2
```

**λ reaches a span**, where it reaches neither the flanks nor the commanded
centre distance: a span is measured between flanks of *different* teeth, and the
indexing offset is exactly what moves one relative to another.

Each is written from the **pitch and the ψ**, not as a difference of two
accumulated seats. The two are the same arithmetic and not the same floating
point, and the ulp between them reaches the screen as a range on a gear that has
none.

One `k` and one pin diameter serve the whole revolution — a caliper is set once
and carried round — so the admissible counts are intersected over every starting
position rather than chosen per tooth. Each measurement is reported at the datum
tooth with the `[smallest, largest]` it takes around the revolution; **an evenly
cut gear's two ends are the same bits**, so a caller reports a range
unconditionally and an ordinary gear reads as one number.

Only **nominal** values are produced. Min/max need a tooth thickness tolerance
that is not available; the result types carry the space for it.

### Tolerance tables

JGMA 116-02, a banded lookup — module band, diameter band, two values in µm. No
interpolation. **Two named scales, never compared:**

```text
JGMA 116-02 fine       grades 0–6,  modules 0.2–1.6
JGMA 116-02 standard   grades 4–12, modules 1–10
```

Default precedence is fine first, then lowest grade, decided on scale and grade
ordering alone rather than on which entry yields the smaller value.

---

## Internal gears

### The flank

Still an involute of the ring's own base circle — the involute is self-conjugate
— but used the other way round. A ring's **space** is what narrows outward,
because the space is where the mating pinion's tooth goes:

```text
Ring::involute_at    (r, θ) = ( r_b √(1+u²) , ψ_b + inv_from_roll(u) )
```

the **plus** being the whole difference. Tooth plus space must come to the
circular pitch at *every* radius across the flank.

### A shift is where the tool sits

The ring's space takes `Gear`'s thickness expression unchanged:

```text
e_ring = m_t (π/2 + 2(x + x_s) tan α_n)        s_ring = π m_t − e_ring
```

so a larger `k` or `x` makes a ring's tooth **thinner**, and an internal pair
wants `k₁ = k₂` where an external one needs `k₁ + k₂ = 2`.

**The two reach the cut by different roads, and that is the rule rather than a
detail.** `x` is radial and is delivered by *where the tool sits*; `k` is
thickness-only and is delivered by *the tool's own tooth*, which comes out as the
standard one scaled by `k` — the basic rack's definition read round a circle. So
the plunge, and with it the root circle, answers to `x` alone:

```text
s_cutter = π m_t k / 2                         the tool's tooth
e_ring − s_cutter = 2 m_t x tan α_n            what the plunge is left to deliver
a_cut = operating_geometry(…, −x)              so no k reaches a radius
```

A shaper cannot be displaced the way a rack can — two pinions have their ratio
fixed by their tooth counts, so the pitch point moves with the centre distance
and the rolling circles with it. Everything follows from one factor:

```text
scale = a_cut / a_ref        r′ = scale · r        phase′ = scale · phase
```

exactly 1 at zero shift. The ring's root circle is `a_cut + r_tip`, where the
tool actually reaches — so **a ring has no dedendum input**; it has a cutter.

### The shaper trochoid

`shaper.rs` is the general case and the rack is its `z_c → ∞` limit, measured:
the difference falls first order in `1/z_c`. Both share one line, because the
fillet is the envelope of the cutter's corner circle, so the fillet point lies on
the common normal — and a rolling pair's common normal passes through the pitch
point.

`σ = ±1` appears in exactly **two** places: the centre distance `a = r + σ r_c`,
and which side of the cutter's axis its tip points from. Deliberately not in the
rolling, where two reversals cancel.

**The corner's phase**, from the fact that the offset of an involute is another
involute of the same base circle:

```text
θ_g = s_c/2r_c + inv α_t − inv α_g − ρ/r_bc,      cos α_g = r_bc/r_g
```

A negative `θ_g` means the corner rounds would overlap: the tip is narrower than
the rounds asked for. The round is **capped** at 95 % of the largest that fits,
and the part reports it — the same rule and the same note an external gear's tip
round gets, since it is the same guard.

**The junction is a tangency, not a crossing**, so it is closed form from the
line of action — for an internal pair the ring's tangency point lies beyond the
cutter's, so the two **differ** by `a sin α_t` rather than summing to it:

```text
√(r_j² − r_bw²) = a sin α_t + √(r_tan² − r_bc²)
```

**No fillet is `None`, not a fillet of zero length.** A cut that generates none
gives `fillet: None` and every consumer answers it.

### Limits

```text
smallest ring       z > 2 h_a cos β / (1 − cos α_t)
generation limit    deepest generated radius = √(r_b² + (a sin α_t)²)
space closes        inv α = π/z − ψ_b          root truncated at r_b/cos α
tooth runs out      inv α = −ψ_b               tip raised to r_b/cos α
```

The last two are one guard at the two ends of a ring's tooth, and both are the
external gear's pointed-tooth cap read on a ring. Its **tooth** narrows inward,
so it can run out of thickness at the tip; its **space** narrows outward, so the
two flanks bounding it can meet before the cutter's tip circle — and where they
do, the root is where they meet rather than where the tool reached. Past that
point the flank would cross the space's own centreline and its mirror image
would come back through it, which is an outline no tool can leave.

Below the generation limit the ring's flank is not cut by an involute at all —
the internal analogue of undercut, and a property of the *pair*. It bites on
ordinary designs: 0.08 mm on a 43-tooth ring cut by a 20-tooth cutter.

Two mesh interference conditions come off the same conjugate relation, forwards
and backwards: the ring's tip cannot touch the pinion where that would fall
inside its base circle, and the pinion's tip cannot reach past where the ring's
flank ends.

**A third asks a different question, and neither of those can see it.** Both of
the above are about a tip reaching past a flank *where the teeth mesh*. The tips
can also foul somewhere else entirely — where the two tip circles cross, which
for a small tooth difference is far from the line of centres, and where the mesh
itself is perfectly conjugate.

```text
cos θ_pinion = (R_a² − r_a² − a²) / (2 a r_a)      the crossing, from each centre,
cos θ_ring   = (a² + R_a² − r_a²) / (2 a R_a)      measured from the line of centres
half width   ψ_pinion = ψ_b1 − inv α_a1            a tooth's angular half-thickness
             ψ_ring   = ψ_b2 + inv α_a2            at its own tip
```

A pinion's tip has to reach past a ring's for them to engage, so the circles of
any pair that meshes cross somewhere and the question is always live. What
decides it is whether a tooth from each is *at* the crossing. The two windows sit
on different wheels at different angles about different centres, and the rolling
is what makes them comparable: take the instant a pinion tooth is symmetric about
the line of centres — the ring space it fills is symmetric about it too, which
fixes both phases at once — and then as the pinion turns by `δ` its window slides
by `δ` and the ring's by `δ z_p/z_r`. Written as intervals of `δ` both have the
same period, one pinion pitch, so one period decides it, and the margin is the
gap between them: negative is the overlap.

It is the condition that decides a one-tooth difference. On a hula drive at
`z = 18` the tips foul at 134° from the line of centres until the far-side gap
reaches about a quarter of a module, while every other condition is content
throughout — verified by rolling the two outlines through a tooth and measuring
containment (`gear-cli hulasweep`, against `gear-cli meshsweep` as its control).

---

## Angularly varying profile shift

```text
x(θ) = x̄ + Δx cos θ                e = m Δx
```

Maximum at 0°, minimum at 180° — what a hob moving radially in and out once per
revolution produces. **The pitch and base circles stay on the axis** and the
angular tooth pitch stays `2π/z`, so the body moves eccentrically at a genuinely
constant ratio. What moves is the tip and root envelope, a limaçon whose
departure from a true displaced circle is `e²/2ρ`.

**Indexing.** Uniform spacing on both flanks would force uniform thickness, so a
gear with varying thickness cannot be exactly conjugate both ways. Tooth `k` is
seated at

```text
2πk/z + λ (ψ̄_b − ψ_b,k)
```

which scales the drive-flank error by `|1 − λ|` and the coast by `|1 + λ|`.
λ = 0 is the minimax optimum and what a plain radial hob oscillation gives; λ = 1
is exactly conjugate forward at twice the error in reverse.

**One hob, one setting.** Whatever is a property of the *tool* is settled once
for the whole gear, by the tooth that demands most of it; whatever is a property
of *one tooth* is reported instead. The tool is a value — `Rack { depth,
tip_round }`, both in millimetres — passed to each tooth, so a tooth handed one
has nothing to clamp. `b_d = depth − m x` is what makes a single tool leave a
moving root envelope.

| guard | whose property | treatment |
|---|---|---|
| cutter tip round | the tool's | shared: the smallest any tooth allows |
| cutter depth | the tool's | shared: the greatest any tooth needs, capped once so the shallowest-shift tooth's root clears the axis |
| tooth comes to a point | that tooth's | reported, with its position |
| tooth is undercut or severed | that tooth's | reported, with its position |

**The root belongs to the gear**, not to the tooth: it runs from each tooth's
fillet junction to the envelope `r − m(h_cut − x(θ))` at mid-space, continuous at
both. The correction is spread across the fillet as well as the flat root, with
`w = t²(3 − 2t)` so the displacement is stationary at both ends, and `t` is
parametrised on **radius** because the flank is re-entrant below the base circle.
The flank itself is untouched — constant ratio requires it to be one involute at
one shift.

A tooth reaches to the **midpoint between the two seats**, not half a pitch:
λ seats the teeth unevenly by construction.

### The commanded centre distance

```text
inv α_w(θ) = inv α_t + 2 ( x(θ) + x_mate ) tan α_n / Σz
a_w(θ)     = a_ref cos α_t / cos α_w(θ)
```

sampled at the **tooth positions** — one per tooth, which is where contact
actually is. `a_w(θ)` is not sinusoidal even though `x(θ)` is, because it passes
through `inv⁻¹` and a cosine, so the best-fit pure sinusoid and its residual are
reported too. The fit is exact rather than optimised: equally spaced samples make
the first Fourier coefficient *be* the least-squares sinusoid.

The eccentricity has two faces and only one is stored: `Δx`, or the
**centre-distance throw**, the second solved from the first by a bracketed
inversion since the throw rises monotonically in `Δx` from zero.

---

## Planetary sets

```text
common centre distance   g(x_p) = a_w,ext(x_s + x_p) − a_w,int(x_r − x_p) = 0
da_w/dΣx = [ a cos α_t sin α_w / cos²α_w ] · [ 2 tan α_n / (Σz tan²α_w) ]
```

`g` is strictly increasing, so the root is unique and Newton is safe from
`x_p = 0`. The bracket is closed form, from `inv α_w ≥ 0` on both meshes:

```text
x_p ≥ −inv(α_t)(z_s + z_p)/(2 tan α_n) − x_s          external
x_p ≤  x_r + inv(α_t)(z_r − z_p)/(2 tan α_n)          internal
```

Required planet shift is **strictly increasing in `z_ring`**, which is what makes
the ring search provably complete, and `z_r = z_s + 2z_p` gives exactly zero.

**Which of the three shifts closes the set is a choice, and only one of the
three is hard.** The equality above is one relation among `x_s`, `x_p` and
`x_r`: two are a design and the third is whatever they leave. The planet is in
*both* meshes, so its shift moves both distances at once and the residual has to
be driven to zero numerically — the Newton solve above. The sun is in one mesh
only, and so is the ring: fix the other two and the mesh the absorber is **not**
in gives the distance outright, leaving its own mesh a shift sum to reach at a
known distance. That is `mesh::shift_sum_for`, the same relation a spur pair
reads a given centre distance through, and a closed form rather than an
iteration.

Which member absorbs is read off the shift toggles rather than named by a
control of its own: **the member left automatic absorbs, and the planet is
preferred**, because it is the one no single mesh's operating angle is a
statement about and the one this tool has always used. Pinning the planet is
therefore how a designer asks the sun to close it instead — the same indirection
by which pinning one of an eccentric drive's two members names the other as the
one the crank supplies. Pinning all three over-specifies the set; the planet
gives way, and the front end relieves it as it is created.

An absorbed shift is **checked, not bounded** — nothing is free to move it, so
the only honest question is whether it actually undercuts, and the bound it
answers to is `x_min` rather than a chooser's `max(x_min, 0)`
([`train::undercut_bound`](#efficiency-parallel-axes)). The ring search keeps the
planet as its absorber whatever the set does, because its completeness argument
is about the planet's shift rising with the ring's count.

**Layout checks**, all closed form: equal spacing needs `(z_s + z_r) mod N = 0`;
simultaneous meshing needs `N | z_s` and `N | z_r`; planet clearance is
`2 a_w sin(π/N) − d_a,planet`.

**Efficiency** — Pennestrì–Freudenstein, all six arrangements from one piece of
algebra:

```text
i₀ = −z_ring/z_sun                          basic (carrier-fixed) ratio
η₀ = η_sun-planet · η_planet-ring           at relative speeds
w  = sgn(T_sun (ω_sun − ω_carrier))         direction of rolling power
T_ring/T_sun = −i₀ η₀^w      T_carrier = −(T_sun + T_ring)
η  = |T_out ω_out| / |T_in ω_in|
```

**`w` is not known in advance**, since it depends on a torque that is itself
being solved for, so both values are tried and the physical one kept. Two
conditions decide it, and the first alone is not enough:

1. the sign the branch assumed has to be the sign it produces;
2. **the output has to absorb what the input delivers** — `T_out ω_out ≤ 0`.

`k = i₀ η₀^w` sits either side of 1 as `w` flips, so where `i₀` is itself close
to 1 the two candidates straddle it, `1 − k` changes sign between them, the sun's
torque does, the rolling power does, and *both* branches confirm their own
assumption. One of the two then has the output's torque along its own rotation —
a shaft delivering power while the input delivers too, with friction making up
the difference — which is energy from nowhere and reads as an efficiency above
1. That is not a corner case: a set reducing by the square of a tooth count lives
there, and nothing about the arrangement warns of it, which is why the second
condition is on the energy rather than on the ratio.

Where **neither** branch has the output absorbing, there is no back-driven state
at all: the set is self-locking, and the refusal is the answer. It agrees with
the classical criterion — back-driving efficiency `2 − 1/η` is negative exactly
when `η < ½`.

**Backlash, referred to an output shaft.** The two meshes sit at the same centre
distance and `r′_p1 ≠ r′_p2` in general. Eliminating the planet leaves Willis at
zero play:

```text
z_s(θ_s − θ_c) + z_r(θ_r − θ_c) = Δ,    Δ = [(z_s+z_p) δ₁ − (z_r−z_p) δ₂] / a
```

Hold the two shafts that are not the output and the third moves by `|Δ| / Z`,
with `Z` its own coefficient: `z_s` at the sun, `z_s + z_r` at the carrier, `z_r`
at the ring.

---

## The hula drive

Four gears in two pairs, all on one crank. Gears 1 and 4 sit on the fixed axis —
1 grounded, 4 the output — while gears 2 and 3 ride a body carried on an
eccentric, so **both pairs are separated by the same distance**, the crank's
offset, and that shared number is what makes the arrangement a drive rather than
two independent meshes.

```text
ratio        R = z₂z₄ / D,      D = z₂z₄ − z₁z₃          (Willis, both meshes)
shift        Σx = Σz (inv α_w − inv α_t) / (2 tan α_n)   Σx = x_pinion − x_ring
offset       e  = a_ref cos α_t / cos α_w
far-side gap C  = a_ref − m(h_ring + h_pinion) − m·Σx + e
dC/dα_w      = −m Σz tan²α_w / (2 tan α_n) + e tan α_w
```

`D` is an **integer** and it is the whole design rule. `|D| = 1` reduces by order
`z²`; `D = 0` means the two meshes step by the same amount and cancel, so the
output cannot turn — a refusal rather than a large number, and four of the
sixteen arrangements of `z, z ± 1` are exactly that. A wobble body carrying one
external face and one internal one lands at `|D| ≈ 2z`, so it cannot exceed about
`z/2` however the counts are chosen; the high-ratio arrangements are the four
whose wobble carries two faces of the same kind. The ratio is reported as the two
products rather than as a float, because it *is* integers.

**Which member of a pair is its ring is not an input.** Two axes one crank offset
apart can only be an internal pair, so the ring is whichever member has more
teeth, and every arrangement describes itself — the sixteen are one code path.

**The offset answers to every bound, not just the gap.** The far-side gap is one
requirement and the room the tips have where their circles cross
([Limits](#limits)) is another; both rise with the offset, so the drive opens out
until the one that asked for most is met and the rest have more than they asked
for. The tips bind below about a quarter of a module of gap, and the gap above
it — so a design asking for less than the tips allow is answered with what can be
built rather than with what was requested, and the figure it actually got is
reported beside the one it asked for. The tip bound is **supplied to the solve**
rather than written inside it, because it belongs to the pair; a copy of it in
the arrangement would be a second answer to a question that already has one.

**One offset, and what is left over.** Only the *difference* of a pair's two
shifts reaches either quantity above: the operating pressure angle takes it
through [`operating_geometry`](#signed-relations-both-mesh-kinds), and in the gap
both tips move together with the sum, so it cancels. The offset therefore decides
the difference and **the sum is free** — one spare number per mesh that no
geometry claims. So the drive has exactly three unknowns, the offset and one
split per mesh, and each names what decides it; a system where every unknown
carries its own source cannot be over- or under-determined, which is why there is
no constraint count to check and no solve order to choose.

**Why the far-side gap is the constraint.** At one tooth of difference a ring and
its pinion very nearly fill each other, and with ordinary proportions their tip
circles *overlap* on the side away from the mesh — at `h = 0.8` and no shift the
gap is `−0.6 m`. A wobble body cannot orbit through that, so the gap is what the
shift is spent on, and it is why a drive of this kind runs at operating pressure
angles no ordinary pair would: 57° at `z = 18`, one tooth of difference and half
a millimetre of gap. Two teeth of difference is far kinder — 26° for the same gap
— at a quarter of the ratio, since `D = 4`.

**Two efficiencies, because one is not the other.** Each pair's own comes from
[`contact::efficiency`](#efficiency-parallel-axes) with the crank held, and the
two multiply — 99.15 % together on the shipped counts. The *drive's* comes from
the three-shaft power flow ([Planetary sets](#planetary-sets)) at
`i₀ = z₂z₄/(z₁z₃)`, and it is nowhere near the first, because power circulates:

| reduction | meshes, crank held | the drive |
|---|---|---|
| 144 | 98.74 % | 35.8 % |
| 324 | 99.15 % | 26.6 % |
| 900 | 99.48 % | 17.5 % |
| 2500 | 99.68 % | 11.1 % |

The nearer the two meshes come to cancelling — which is what buys the reduction —
the more power goes round between them before any reaches the output, so a
*better* pair of meshes at a *higher* ratio is a worse drive. Every one of these
is self-locking: below half efficiency forward, the reversed flow has no state
where the output absorbs.

**The figure is the mechanism, not the model**, and the same code says so. Of
the sixteen arrangements, those whose wobble body carries two faces of the *same*
kind cancel, reduce by `z²`, and keep about a quarter; those carrying one of each
cancel nothing, reduce by about `z/2`, and keep ninety-odd percent — on the same
two meshes, losing the same 0.85 % between them.

| arrangement | `D` | ratio | meshes | the drive |
|---|---|---|---|---|
| `N+1/N/N−1/N` | 1 | 324 | 99.15 % | 26.6 % |
| `N/N+1/N/N−1` | −1 | −323 | 99.15 % | 26.4 % |
| `N+1/N/N/N−1` | −36 | −8.5 | 99.15 % | 92.4 % |
| `N/N+1/N/N+1` | 37 | 9.8 | 99.17 % | 93.2 % |

**What the tool ships with is `N ± 4` about 61**, reducing 232.6:1 and keeping
73 % forward and 63 % back. Four teeth of difference cancels less than one does,
and that is the whole of the difference: the same code, the same two meshes, and
a drive that keeps two and a half times what the `N ± 1` arrangement does at a
comparable size. It carries a shorter tooth with it — 0.7 module rather than 0.8
— because the addendum belongs to the *difference* rather than to the drive: the
operating pressure angle at four teeth is far lower, so a tooth that cleared the
involute interference limit at one tooth of difference reaches past it here. That
coupling is reported rather than assumed, and a drive taken to another difference
will want its own proportion.

This is the published behaviour of a Wolfrom set — efficiency falls as the
reduction rises, because the reduction *is* the cancellation — and it is why such
drives are used where their efficiency does not matter. A **three-ring reducer**
reaches a high ratio at high efficiency by not doing this at all: its rings are
carried on a parallelogram of cranks and *translate without rotating*, so its
reduction comes from one mesh's tooth difference with nothing to cancel against,
and two of them in series multiply ratios rather than cancelling. It is a
different mechanism, not a better-built one, and it is not what these four counts
describe.

**The whole power flow collapses to one relation** for this arrangement — carrier
driving, one central member held, the other the output:

```text
η = 1 / [ R(1 − η₀) + η₀ ]
```

which is the most useful thing here, because it answers the design question
before anything is drawn: *what would the teeth have to be worth?* The loss term
carries `R`, so a reduction multiplies the mesh loss before it reaches the
output. At 324:1 a mesh pair losing 0.85 % keeps 27 %, and it would have to lose
under 0.04 % to keep 90 %. The solve agrees with it across three reductions and
four coefficients of friction, which is what makes it safe to design against.

**Checked against a gearbox somebody built.** The bilateral drive gear is a 3K of
this family, optimised for efficiency by choice of profile shift and tooth count,
reporting 89.0 % forward — against 68.5 % for *the same gearbox with uncorrected
teeth*. Read through the relation at a reduction near fifty those are meshes at
99.73 % and 99.04 %: an excellent pair and an ordinary one. What the comparison
establishes is the relation; the stage's own answer at that reduction is above
the published figure rather than at it, because the six- and seven-tooth pinions
the check is built on take the shift such counts need to exist at all, and so
mesh better than the gearbox being compared to.

**A reduction does not say how to get it, and the difference of one tooth is the
dearest way.** `R = z²/d²`, so `z = d·z₀` gives the same `z₀²` at any tooth
difference `d` — the same ratio, the same pitch diameters, the same crank offset,
reached with `d` times the teeth at a `d`th of the module. At 324:1, with the
addendum, the shaper and each mesh's shift division free (`gear-cli hulaband`):

| d | z | module | meshes | the drive | α_w | backlash out |
|---|---|---|---|---|---|---|
| 1 | 18 | 1.000 | 99.32 % | **31.3 %** | 48.0° | 0.396° |
| 2 | 36 | 0.500 | 99.77 % | **57.7 %** | 33.7° | 0.149° |
| 3 | 54 | 0.333 | 99.94 % | **83.7 %** | 26.2° | 0.080° |
| 4 | 72 | 0.250 | 99.98 % | **93.9 %** | 20.0° | 0.047° |
| 6 | 108 | 0.167 | 99.98 % | **95.1 %** | 16.1° | 0.025° |
| 9 | 162 | 0.111 | 99.92 % | **79.9 %** | 12.8° | 0.014° |

**These are optimised divisions, and they sit on a bound rather than at an
optimum.** The *sum* of a mesh's two shifts is never free — the crank offset is
what it is — so only the division is, and searching it (with the addendum and the
shaper) is what the rows above report. The stationary point of the loss is not
where they land: on these drives the mesh loses least at divisions of `+2.85`,
`+2.05` and `−2.55` for one, two and four teeth of difference, and none of the
three is admissible, because contact has gone discontinuous or the tips have
fouled well before. Every row sits at `ε ≈ 1.00` with the tip margin at zero
instead. The loss is still falling when the geometry runs out, so what a designer
wants to know is which bound stops it — and that is what these are.

Three times better at four teeth of difference than at one, on the same
reduction in the same envelope — and a sixteenth of the backlash. The mesh
figures explain it: a one-tooth pair has to be opened to 48° of operating
pressure angle to clear itself, and the loss carries `1/z₁ + 1/z₂`, which halves
as the counts double. It turns over past six, where the contact ratio has grown
and the path sits further from the pitch point again.

**A pair's loss is dimensionless, so the two meshes do not constrain each
other.** Every term is a ratio — the ends of the path in base pitches, the
reciprocal tooth counts — so a pair keeps the same fraction of what it is given
at any size. Two meshes sharing one offset can therefore be *chosen
independently* and reconciled afterwards by their modules,
`m = 2 a_w cos α_w / (|Σz| cos α_t)`; the offset itself is a scale that cancels.
What the far-side gap, the tips and the contact ratio do is **remove ranges of
each pair's own shifts**, not tie the two pairs together.

Optimised that way, each pair on its own at `z = 36`, `h_a = 0.6`, `μ = 0.08`:

| d | reduction | α_w | the pair keeps | the drive keeps | m₁/m₂ |
|---|---|---|---|---|---|
| 2 | 324 | 34.8° | 99.875 % | 54.7 % | 1.0000 |
| 3 | 144 | 27.2° | 99.949 % | 86.5 % | 1.0000 |
| 4 | 81 | 23.3° | 99.965 % | 94.0 % | 1.0000 |
| 5 | 51.8 | 22.7° | 99.956 % | 95.0 % | 1.0000 |

**The module ratio comes out exactly one** — not because the meshes are tied but
because a drive's two pairs are near twins, `(z+d, z)` against `(z, z−d)`, whose
independent optima land at the same operating pressure angle. Equal modules is a
coincidence of that near-symmetry, and the table below is what it costs to depart
from it.

**Against the lowest shifts that clear.** The default rule — take the least shift
that produces non-interfering geometry — gets the *sum* right and the *division*
wrong:

| d | least loss (Σx, x_ring, x_pinion) | least shift | drive, best | drive, least |
|---|---|---|---|---|
| 2 | −0.20, +0.40, +0.20 | −0.20, +0.05, −0.15 | 54.65 % | 49.83 % |
| 3 | −0.10, +0.55, +0.45 | −0.10, +0.15, +0.05 | 86.48 % | 78.50 % |
| 4 | −0.05, +0.55, +0.50 | −0.05, −0.10, −0.15 | 94.00 % | 87.49 % |
| 5 | −0.05, +0.50, +0.45 | 0.00, −0.10, −0.10 | 95.04 % | 93.63 % |

The **sum is identical** at `d = 2, 3, 4` — a constraint sets it, and the default
finds it. The whole difference is that the optimum raises *both* shifts together
by 0.4 to 0.65 while holding their difference, and the default has no reason to.
That is worth 1.4 to 8 points of drive efficiency, and it is the freedom
[`efficient_split`](#efficiency-parallel-axes) exists for: at `d = 4` and `d = 5`
it lands on the sweep's answer (0.510 against 0.500, 0.436 against 0.425, the
efficiencies agreeing to 1e−7). At `d = 2` and `d = 3` it reports no root,
correctly — the loss there is still falling when the contact ratio runs out, so
the answer is a bound and not a stationary point.

**The two modules are separate inputs and want to be equal.** Nothing in the
arithmetic ties them — a pair's module is its own — so it is worth knowing that
moving them apart only costs. Both meshes run at one offset and each needs
`e ≥ a_ref cos α_t`, so below equality the larger mesh still binds, the offset
does not move, and the smaller mesh's reference distance falls away from it;
above equality the enlarged mesh binds instead and drags the offset up, pushing
the *other* mesh's angle out by what the first one gained. At `z = 18`, one tooth
of difference, the worse of the two operating pressure angles reads:

| m₁/m₂ | 0.80 | 0.90 | **1.00** | 1.10 | 1.30 |
|---|---|---|---|---|---|
| offset, mm | 0.704 | 0.704 | **0.704** | 0.774 | 0.915 |
| α_w, mesh 1 | 57.7° | 53.1° | **48.1°** | 48.1° | 48.1° |
| α_w, mesh 2 | 48.1° | 48.1° | **48.1°** | 52.6° | 59.1° |

Equal modules is a corner where both bounds are active at once, and the drive
efficiency falls away either side of it — 30.5 % at equality against 26.6 % at
0.8 and 27.5 % at 1.1 on the same search. So the design space is
`(z, d, addendum, shaper, two divisions)` and nothing more: the tooth
differences are equal because the ratio demands it, the two base counts are equal
for the same reason, and the modules are equal because the offset is shared.

So the interesting design is **not** the one-tooth difference. It is the largest
difference whose teeth still clear each other, which on this reduction is four to
six — and the constraint that stops it going further is involute interference,
answered by shortening the teeth, which is why the addendum is part of the
search rather than a fixed 0.8.

Two things follow, and they are the practical content of the whole section.
**A reduction near fifty is where this family works**; the same optimised meshes
at 324:1 would keep 53 %, and no mesh a designer can cut reaches 90 % there.
And **profile shift is the lever**, worth more here than anywhere else in this
tool: a third of a percent of mesh loss is 20 points of drive efficiency at these
ratios, which is the published 68.5 % → 89.0 % and matches the sensitivity this
model shows — half a point of mesh efficiency at 324:1 takes the drive from
26.6 % to 46.8 %.

**The stage builds what the arrangement describes.** A drive that closes
algebraically can still be one whose teeth foul, so each ring is cut by its
shaper and each pair is asked what it thinks: the contact ratio, the two
interference conditions, and the gap measured on the tips as cut beside the one
the arrangement solved for — they part company exactly where a tip was clamped.
Backlash is the pair's own, through the same `Mesh` relation every stage here
uses rather than a second one written for this arrangement.

**A shaper has to be smaller than the ring it cuts**, and these rings are small.
A tool larger than its workpiece is clamped down to the ring's own tooth count
and then reaches none of its flank, leaving no fillet at all — an ordinary
mistake here rather than an exotic one, which is why the shipped cutters sit
well below the shipped rings.

Both terms of `dC/dα_w` are positive (`Σz < 0`), so the gap rises strictly with
the operating pressure angle and hence with the offset. The root is unique, and
taking the larger of the two meshes' requirements is safe: opening the drive out
for the mesh that needs it gives the other one more as well. The two conditions
that bite *inside* a mesh — a tip reaching past a flank — belong to the pair and
are asked at [`ring::mesh_with`](#limits) rather than restated here.


## Trains

Per stage `i = z_out/z_in`; a worm's is `z_wheel/z_starts` and a planetary's
comes from its own kinematics. Total ratio is the product. Torque propagates with
efficiency always *reducing* delivered torque, in either direction:

```text
forward   T_{k+1} = T_k i_k η_k          backward  T_{k−1} = T_k η_k / i_k
```

**Backlash accumulates referred to the output shaft**, so the last stage
dominates:

```text
θ_out,total = Σ_k  j_θ,k / Π_{j>k} i_j
```

### Load cases

A train carries two loads and they are judged against different figures. Every
stress, cycle count and minimum face width is a `LoadCase<T>`:

```text
peak     max(|T_forward|, |T_backward|)   vs  ultimate_allowable
cyclic   |T_operating|                    vs  fatigue_allowable
```

`T_forward` propagates from the input as above. `T_operating` is an input in its
own right, clamped to the peak, and may be zero. Both scale every rating in
closed form — bending is linear in torque and contact goes as its square root —
so a second case is a scale, not a second solve.

**Back-driving.** `T_backward` is applied at the *output* shaft and works
upstream, referred to each stage's own input shaft and attenuated by that
stage's backward efficiency:

```text
T_k = T_out,k / i_k          the load stage k carries, at its input shaft
T_out,k−1 = T_k η_b,k        what reaches the stage above it
```

The walk stops at the first stage with `η_b ≤ 0`: that stage reacts the load and
everything upstream carries none of it. If the walk reaches the input still
nonzero, **nothing reacted it** — the train is back-drivable, the load simply
turns it, and the case is zero at every gear.

**Load sharing.** A stage input, `LoadSharing`, **off by default**. It reaches
bending alone — a contact rating is already taken where one tooth carries
everything, so sharing cannot move it — and where it is off the rating is
`bending_section`'s own answer rather than one that agrees with it. Switched on,
the mesh cycle is swept for the largest `Y_F · Y_S · share`; the share is
`contact::load_share`, in base pitches from the far end of the path, so the
transverse path and the virtual spur gear ask one function rather than two.
Above a virtual contact ratio of 2 there is no single-pair zone and the ramp is
extrapolating, which the stage reports.

**Automatic face width.** Four ratings, four toggles per gear; the width is the
largest any *enabled* rating asks for. Peak contact is off by default — see
[rationale](rationale.md#a-contact-pressure-is-not-a-tensile-stress). With none
enabled there is nothing to invert and the width is zero, which the stage reports
as a note.

**Where each figure lives.** At any one instant a mesh has **one** contact
pressure: the two flanks share a patch, a normal force and an `E*`, and the
individual radii reach Hertz only as `1/ρ = 1/ρ₁ + 1/ρ₂`. That shared figure is
reported per mesh, at the pitch point.

The two gears are nonetheless rated at **different points**, and so carry
different contact stresses:

```text
σ_H,i = max( σ_H(pitch point), σ_H(gear i's inner point of single-pair contact) )
```

`ρ₁` rises and `ρ₂` falls monotonically along the path, so gear 1's flank is at
its root at the low end and gear 2's at the high end — one relation, both mesh
kinds, since a ring's root is its *larger* radius. Pitting initiates in the
dedendum, so each gear is assessed where its own root carries the load alone.
This is ISO 6336-2's `Z_B` and `Z_D` (`max(1, M_i)` on the pitch-point stress,
pinion and wheel respectively), reached by evaluating the two points rather than
by quoting the factor.

Bending is per gear for the ordinary reason: each tooth has its own root section
and form factor.

**An automatic face width is sized to the mesh, not to one gear.** The narrower
face carries the pair, so each automatic width resolves to the largest ask any
member of that mesh has. A member in two meshes — a planet — answers to both.

A gear's reported `torque` is the one it carries **driving forward**; a
back-driving load is reported beside it, not folded into it. The peak *rating*
still uses whichever direction loads the teeth harder.

### Tooth cycles

Revolutions first. Intermittent: `(range/360) × Π(ratios between i and output)`
per actuation. Continuous: `rpm_i × 60 × hours`, where `rpm_i` is that shaft's
speed scaled from the peak the train was laid out at to the operating speed.
One engagement per revolution for a simple gear, `N_planets` for a sun or a
ring; a planet's rotation counts relative to its carrier.

Counts are then **whole numbers**, and where the rounding happens depends on
whether the drive reverses:

```text
not reversing   bending = contact = ceil(revolutions over the whole duty)
reversing       bending = ceil(revolutions per actuation) × actuations
                contact = bending / 2
```

A partial sweep still loads the teeth it reaches, so a reversing drive rounds
*within* one actuation rather than once over all of them; and its two flanks
share the engagements while the root takes every one of them. A planet's bending
is fully reversed whatever the drive does — the sun loads one flank and the ring
the other — and a reversing drive loads every root both ways. The two do not
stack: a member reverses if either says so.

**Whether that is corrected for is a train-wide input, off by default.**

```text
reversed_bending = false   the reversal is reported, member by member
reversed_bending = true    cyclic bending allowable × 0.7 for those members
```

Bending only: pitting is compressive on whichever flank carries it, so a contact
rating keeps the material's own allowable either way.

---

## Materials

Each entry describes a material in **one** state, named by its `condition`
field; a material in another state is another entry. Every value carries a
`basis` — `overridden`, `datasheet`, `derived`, `chart`, `estimated` — and
anything that is not a plain datasheet reading carries a note saying what it is.

```text
E*         1/E* = (1−ν₁²)/E₁ + (1−ν₂²)/E₂
allowables ultimate_allowable, fatigue_allowable — pairing with peak and cyclic torque
```

Stored SI (density in kg/m³) and displayed in the domain's own units, with the
two deliberate exceptions of mm for length and MPa for stress.

Overrides live in the input state, not the library, so outputs stay a pure
function of inputs. An overridden value loses its moisture states and its basis
becomes `overridden`, ordered *ahead* of `datasheet`.

---

## Export and import

**DXF**, ASCII, hand-written: the profile as a dense `LWPOLYLINE` with spacing
from the chord tolerance, its tip and root arcs carried as vertex **bulges** —
exact circular arcs rather than chords, in the one entity a designer can extrude
without joining anything first — and reference circles on a construction layer.
A ring also carries a rim circle at `r + 2 m_t` — a drawing convention with no
engineering meaning, and `Ring::rim_radius` is its one home.

**The file is written to the published R2000 minimum**, which is more than the
geometry. AC1015 is a graph rather than a list: six sections in order (`HEADER`,
`CLASSES`, `TABLES`, `BLOCKS`, `ENTITIES`, `OBJECTS`), nine symbol tables —
`VPORT`, `VIEW` and `UCS` may be empty, and are — with `ByBlock`/`ByLayer`/
`Continuous` line types, layer `0`, a `Standard` text style, an `ACAD` appid, a
`Standard` dimension style, and a `BLOCK_RECORD` for each of `*Model_Space` and
`*Paper_Space`; both spaces defined again as blocks; a root dictionary naming
`ACAD_GROUP`; and an owner handle on every record, entities included, since a
layout owns what is drawn in it. `DIMSTYLE` is the one record whose handle is
group code **105** rather than 5.

None of that draws anything, and leaving it out costs nothing until a reader
that does not rebuild what it is missing is asked to open the file. See
[corrections.md](corrections.md#a-reader-that-repairs-is-not-a-check).

**Geartrains and the material library**, TOML, the same shape as the input
structs. **Inputs only**, so files stay small and cannot go stale. A geartrain
document is `{ name, train }`; an unknown material is not an import failure, but
a train with no stages is refused.

---

## The boundary

Seventeen `#[wasm_bindgen]` entry points, JSON in and JSON out, all pure
functions. Five are not calculations: `defaults`, `strings`, `languages`,
`resolve_language`, and the geartrain document's two directions.

Every type that crosses is declared to TypeScript by `ts-rs` into
`web/src/wire`, generated rather than hand-copied.
`tools/check_bindings.sh` regenerates and requires no diff.

A value that does not exist crosses as `Maybe::Unavailable`, carrying a `Note` —
a stable key and the values its sentence needs — exactly as a clamp does.

### Languages

Every word the application shows lives in `crates/gear-io/data/strings_<code>.toml`,
one file per language, all compiled in. Five ship: `en`, `de`, `pt`, `zh-Hans`,
`zh-Hant`.

```text
languages()             [{ code, name, english }, …] — the name in itself, and in English
resolve_language(tag)   which of them a BCP 47 tag names
strings(tag)            that language's catalogue, filled in from English
```

`Language::resolve` matches widest-last: the exact tag, then Chinese by script
(`Hant`/`TW`/`HK`/`MO` traditional, otherwise simplified), then the primary
subtag, then English. A translated catalogue is layered **over** English, so a
key it lacks shows the English sentence rather than a bare key — a safety net,
not the plan: a test holds every shipped file to English's exact key set and to
the same placeholders in each message.

**Nothing the application says is written in the front end**, including the
words around a number: a stage heading, a gear's name, the two halves of a
directional efficiency, the word between the ends of a range, and the name a
fresh tab starts with. `tools/check_strings.py` holds the catalogue and the
sources to each other in both directions; what it cannot see is a sentence that
never became a key, so the front end is swept for bare English as well.

The picker sits under the title in the sidebar and stores its choice in
`localStorage`; it is a preference about reading, not an input to a calculation,
which is why it may outlive a session when nothing else in the application does.
Switching needs no reload — `t()` is a reactive read, so refilling the catalogue
re-renders every label.
