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
p_b = π m cos α                                             plane.rs
```

The two that carry an angle between the normal and transverse planes are
`crate::plane`'s, in one place: each was written out nine or ten times across
the crate, which is the shape of a defect this project has recorded more than
once. A spur gear is `β = 0`, where both reduce exactly.

**The base pitch is there for the same reason**, at six sites rather than
nineteen — and it carries the plane trap those two do not: its module and its
pressure angle have to be in the *same* plane, and nothing in a signature of two
numbers can insist on it. Which plane is wanted follows from what is being
counted in it. A contact ratio divides a transverse path, so it takes `m_t` and
`α_t`; a bending section measures the **virtual spur gear**, whose own
transverse plane *is* the normal plane, so it passes the pair it has and gets
`p_bn` by construction rather than by remembering to.

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

<!-- figures-exempt: input bounds are a specification of what the generator accepts, held in `params::guard`, not a figure any command prints -->
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

**The five degeneracy fractions are conventions, and they are kept as such.**
`params::guard` holds them — a smallest cutter depth and a largest, a smallest
tooth thickness and a largest, and a smallest pressure angle — and each sits
*inside* the limit where the shape actually stops existing: the depth limit is
the axis, the thickness limits are zero and the whole pitch. Read against
[could this gear exist?](rationale.md#an-input-limit-means-could-this-gear-exist)
every one of them could be widened.

They are not, and the reason is that **this tool searches**. The optimum of a
shift search sits *against* a bound rather than in a bowl — the loss falls
monotonically with the length of the path, so the least-loss pair is always the
one whose teeth barely reach, and every row of every table in
[Efficiency](#efficiency-parallel-axes) reports which bound stopped it. Widening
a guard therefore does not merely admit a shape a designer might type; it moves
the answer the tool *returns* onto a thinner tooth, a deeper cut, a root nearer
the axis. That trades a design somebody could make for one nobody would, and it
does it silently, on a control the designer did not touch.

So the rule these follow is narrower than the one an input field follows: a
*given* number is held to what can exist, and a *chosen* one is held to what can
be made. Where those differ the guard is the second, and it is a convention with
its reason written down rather than a degeneracy tolerance — which is what
`params::guard`'s own preamble had called the whole group.

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

### Which of the three numbers is given, and which follows

A centre distance is the **true** distance and a clearance says what portion of
it is clearance, so

```text
centre distance = zero-backlash distance + clearance
zero-backlash distance = f(the shifts)
```

Two relations in three unknowns, so **any two of {distance, clearance, shifts}
are given and the third follows**. That is three working modes, and they are
modes of one model rather than three behaviours:

| | Given | Derived |
|---|---|---|
| **1** | clearance, shifts | the distance: nominal + clearance |
| **2** | the distance, shifts | the clearance: distance − nominal |
| **3** | the distance, clearance | the shifts, solved to reach `distance − clearance` |

**Mode 3 does not need an optimiser.** The distance fixes the shift *sum*
exactly (`mesh::shift_sum_for`, closed); what it leaves open is the division
between the two members, and that is a separate question with its own answer.
Optimising for efficiency is one answer to it. With nothing being optimised the
division is the **evenest one the members allow** — the even split projected
onto each member's admissible interval, so a member against its undercut floor
holds there while the other absorbs, until the two are level and thereafter move
together (`auto::divide_shift_sum`).

A shift a designer **gave** is never one of the numbers being chosen: it stands,
and the other member takes the whole of the rest.

**Where no admissible pair of shifts reaches the distance, the stage says so.**
It still answers — at the shifts it would have built anyway, because the gears
are cuttable and it is the *assembly* that is impossible, which rule 5 calls a
clamp rather than a refusal — and `stage.centre_distance_not_reached` names both
distances so the clearance readout is not the only trace.

At the tight end there is a second thing to say. A distance short enough puts the
running centres **inside** the pair's own zero-backlash distance, which is teeth
overlapping at rest: nothing can be assembled there and every figure taken at it
describes nothing. `stage.clearance_negative` says that, and it is a stage's
finding rather than a mesh's because it is the *assembly* that fails.

**And a third thing a stage says about its shifts: whether the optimiser found
anything to choose.** Turning it on and seeing no shift move means one of two
opposite things — the search ran and *agreed*, the optimum being on the floor the
stage already sits at, which is the ordinary answer wherever loss falls toward
the shortest admissible path; or the search ran and found **nothing admissible at
all**, so there was no answer and the stage kept what it had. The first is the
tool working; the second is a design with no room in it.

The shifts cannot tell them apart, so `stage.optimiser_found_nothing` does. A
hula stage at a one-tooth difference is the case: it opens to about 45° of
operating pressure angle to clear itself, sits at `ε ≈ 1.02`, and every split of
both meshes is refused.

Both of the distance findings belong to every kind that has a centre distance — a crank offset included,
that being what a hula stage calls one — and both are `train::distance_notes`, so
they are said in the same words wherever they arise.

**The clearance is an `Auto` like the distance, because it is the same kind of
number.** Either may be the one given and the other the one derived — that is
what modes 1 and 2 are — and a plain number could not say which it was, so the
box was read in some states and silently disregarded in others. What it cannot
be is **derived at the same time as the distance**: a distance is nominal +
clearance and a derived clearance is distance − nominal, so with both automatic
neither has anything to derive from.

**A planetary set counts one relation more, and it is real geometry.** Its two
centre distances must agree — one relation among its three shifts, whatever they
agree at — so two shifts are a design and the third is what they leave. Give it a
distance as well and there is a **second** relation, since each mesh must now
reach *that* distance rather than merely match the other, and only **one** shift
stays free. Both of those per-mesh conditions are `shift_sum_for`, so a target
makes the layout easier: the Newton iteration on the planet's shift disappears
and the sun's and the ring's shifts are read off the planet's in closed form.

**A pair has a fifth input in the relation, its *size*.** The additional helix,
the first member's helix angle and the first member's pitch diameter are three
readings of one number (`d = z m_n / cos β`, `FirstMemberSizing`), and every
pair — spur, helical, crossed or worm — relates `{a, clearance, x₁, x₂, size}`
by one equation, so four may be given. Which absorbs a given distance is a
preference rather than a law, and it is the same on every kind: **the shifts
do wherever one of them is free, and the size only when both are pinned**,
because a shift moves the teeth where a size changes them. On a crossed mesh
the shift enters as a rack's does, `a₀ = a_ref + (x₁ + x₂) m_n` exactly
([Crossed axes](#crossed-axes)), so the sum is one subtraction; on a parallel
one it is `shift_sum_for` as before.

The worm kind sets the convention of worm practice as inputs: its worm's shift
is pinned at zero — the worm is the tool its wheel is cut by — so a given
distance moves the **wheel's** shift, as DIN 3975 has it, and pinning the
wheel's too is what makes the worm's diameter absorb it. That last case is
where the size is the answer to a housing, and it has an answer only
sometimes, and a *pair* of answers often. A screw pair's reference distance is
`(d₁ + d₂)/2` and the two move opposite ways as the worm is resized —
`d₁ = z₁ m_n / sin γ` shrinks as the thread steepens while `d₂ = z₂ m_n / cos β₂`
grows — so the distance has a **minimum**:

```text
z₂ sin β₂ / cos²β₂ = z₁ cos γ / sin²γ         β₂ = Σ − 90° + γ
tan γ = (z₁/z₂)^⅓                             at Σ = 90°, in closed form
```

Above that minimum two different worms reach the same centres — a thin one with
a fast lead and a fat one with a slow one — and the tool takes the branch the
designer's own number is on, which is the only choice under which nudging the
target moves the answer smoothly. Below it there is no worm at all, and the
stage says so. On parallel shafts there is no turning point: the distance only
grows with the helix, and a helical pair cut to fit a standard centre distance
is the same request with one branch.

That is one of the two bounds `train::FreedomGroup` carries. The other counts how
many may be *given*, and the pair of them is what makes an over- or
under-determined stage resolve itself: too many given turns one automatic, too
many automatic pins one, in the order the stage declares. A kind with **no**
distance input says `automatic_at_most = 0` for its clearance and so can never
derive it — which is the planetary today, and is the same statement counted
rather than special-cased.

The objective and the constraints are not the same kind of thing, and failing at
one must not discard the other: where the optimiser's own conditions — a minimum
contact ratio, a tool that leaves the members alone — admit nothing, the stage
falls back to what the *constraints* imply, not to what it would have built with
no distance given at all.

### Interference: a tip reaching past the flank it meshes with

A flank is involute only between its tip and the radius where it hands over to
its fillet. If the **other** member's tip contacts outside that span, the contact
is on a fillet rather than on a conjugate surface — the teeth interfere.

One relation answers it for both arrangements, and it is
[`mesh::conjugate_radius`]: the mate's radius fixes its own `ρ`, `ρ` fixes `ξ`,
and `ξ` fixes this member's radius. With `r_b2` signed that is

```text
external   ρ_here = a_w sin α_w − ρ_mate
internal   ρ_ring = a_w sin α_w + ρ_pinion
```

without either being written out. The comparison then flips with the kind for the
same reason — a ring's flank runs *outwards* from its tip:

```text
interference   ⟺   σ_i (r_i − r_j,i) < 0        σ₁ = 1,  σ₂ = the kind's
```

**The literature names the two internal cases and not the external one.** A ring's
flank reached by the pinion's tip is *trochoid* interference and a pinion's flank
reached by the ring's tip is *involute* interference; an external pair's is
simply "interference", and is what a long addendum on a small pinion does. They
are one condition asked of each member in turn, so `MeshReport` reports it as
`flank_interference[member]` and the classical names live here.

It is **not** the same question as undercut, though it has the same remedy.
Undercut is what the *cutter* does to one gear on its own; interference is what a
particular *mate* does to it. A 9-tooth pinion at its undercut floor is clear of
both; unshifted it is clear of neither.

The one condition that does not generalise is two tips fouling **away from the
line of action**, which is `TipRoom`: an external pair's tip circles cross on the
line of centres or not at all, so the question does not arise.

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

<!-- figures: gear-cli shifts 9 37 -->
<!-- figures: gear-cli shifts 17 43 -->

| pair | least loss | least shift that clears undercut |
|---|---|---|
| 9/37 | **97.678 %** at `Σx = 1.4078`, ε 1.2929 | 97.561 % at `Σx = 0.4736`, ε 1.3280 |
| 17/43 | **98.488 %** at `Σx = 1.2566`, ε 1.4626 | 98.345 % at `Σx = 0.0057`, ε 1.5993 |

**Where the optimum sits is the pair's own answer, not a rule.** On 17/43 it is
*interior* — every neighbouring shift, in either member or both, is worse and
buildable, so nothing holds it there but the loss turning over. On 9/37 it is
**against a constraint**: every direction that would improve it describes a pair
that cannot run, and the search stops where the geometry runs out rather than
where the derivative vanishes.

Which constraint, on 9/37, is **interference**: the wheel's tip reaching past the
end of the pinion's usable flank. That pair used to be answered at `Σx = 1.6697`
for 97.706 %, and those teeth foul — the condition was asked of internal meshes
under two classical names and of external ones not at all. Nought point nought
three of a point is what the honest answer costs. That difference is why choosing the shifts is a
search and not a solve, and [rationale.md](rationale.md#and-the-one-thing-in-the-crate-that-is-none-of-the-above)
argues it at length.

This matters because the automatic shift this crate has always offered is the
*undercut* one, the least that clears, and that is a **floor rather than an
answer**. A pinion small enough to need shift to exist does not change the
direction, only where the shift goes — at 9 teeth the floor pins `x₁` at +0.47
and the optimum puts a comparable amount on its wheel.

What it buys the efficiency with is contact ratio, and that is a trade a designer
may not want: fewer teeth sharing the load, and a noisier pair. The tool reports
both and decides neither.

**The same question, asked with the centre distance given**, must give the same
answer at the distance the free search chose — it fixes the shift *sum* and
leaves only the division, which is the search's own second coordinate.
`gear-cli shifts` prints that sweep beside the table, and
`a_given_distance_gets_the_gears_the_free_search_would_choose` holds it.

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

**It is a derived fact and not what the tool does.** The shift optimiser searches
the division alongside the sum; nothing in production calls the solver. Two
reasons, and [rationale.md](rationale.md#and-the-one-thing-in-the-crate-that-is-none-of-the-above)
argues them: the expression above holds where each tip moves at `m` per unit of
shift, which the **default** tip-width cap makes false — a capped tip moves at
about half that, and differently on each member, so the factor stops cancelling —
and the corrected condition supplies only the interior candidates where the
optimum is as often at an end of the admissible interval.

<!-- figures: gear-cli shifts 17 43 -->
<!-- figures: gear-cli shifts 13 61 -->
**What it is worth depends entirely on what the mesh feeds.** On an ordinary
pair it is six to fourteen hundredths of a point — 98.345 % to 98.488 % on
17/43, 98.223 % to 98.287 % on 13/61 — which is real and small. On a stage whose reduction
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
| a crank offset | the same, on each of the hula stage's two meshes |

A pair has two shifts to choose, so any two of `{a, x₁, x₂}` fix the third and
pinning all three is a contradiction rather than a tighter specification. The
front end relieves it visibly — the input furthest from what was just touched
returns to automatic — rather than accepting a number and disregarding it.

Two bounds have never had to bite before, because near zero shift they do not:

- **A contact ratio floor.** Loss falls monotonically with the length of the
  path, so the least-loss pair is always the one whose teeth barely reach and the
  floor is the answer rather than a guard. It is therefore a stage input. 1.2 is
  the usual design minimum for a pair; the hula stage defaults to continuous
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
| hula | each mesh's division of its shift | the two meshes' product |
| worm, crossed | — | the search is the parallel-axis mesh's; a crossed pair asked to optimise takes what the constraints imply and says so |

The searches share `auto::maximise`: what differs between stages is how many
numbers are free and what they are worth, not how to look for them, and
`auto::Freedoms` is the one mapping from what a search hands back onto the full
set. Each rates the geometry the stage would *build* — the addendum held to the
tip width it has to keep, at the distance the pair runs at rather than its
zero-backlash one — so what is optimised is what is reported.

**The same four questions of every chosen shift** (`auto::member_is_buildable`):
the shift is at or above the least that clears undercut, the flank is not
undercut anyway, the tooth does not come to a point before its tip, and the root
round asked for still fits the space — which shrinks as the shift rises, since
the cutter bites less deep and the space narrows. A ring is not asked: its root
and fillet are its shaper's rather than inputs of its own, and it answers of its
**cutter** instead — did the tool leave the shape the shift asked for.

**And an internal mesh is asked three more**, which belong to the pair rather
than to either member: the two interference conditions below and the tip margin
beside them. They are the same three whatever is turning around the mesh, so a
hula pair, an epicyclic set's planet-ring mesh and an ordinary internal pair are
held to them alike — a search may not choose a mesh that fouls, and every mesh
that has a ring in it reports what it found (`train::TipRoom`).

They were once asked stage by stage, which meant a bound reached the search it
was written in and no other: the root round bounded a pair and not an epicyclic
set, the hula stage was choosing a pinion nobody could cut and taking 1.9 points
of efficiency less for it, and the tip room was a hula stage's row alone while
the set with the same ring mesh in it reported nothing at all.

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
true minimum let the hula stage's split walk out to −1.79 and come back
with less stage efficiency than it started with.

A ring has neither control's second half: its flank is its shaper's rather than
a rack's, so it is given or it is the stage's to choose, and undercut is not a
question that can be asked of it.

**The other end of the tooth is the same shape.** An addendum's only automatic
value was ever the tallest tooth that keeps a tip `min_tip_width` wide — a
*bound on the number* wearing a source's clothes, and one that was read only
while the toggle was on, so an addendum a designer typed went unbounded and a
tooth could come to a point with nothing said. It is `no sharp tip` now: the
number is always the designer's, and the bound holds it down to the tallest
tooth that keeps the tip. It bites exactly — the tallest that clears, not an
arbitrary shorter one — and says so when it does.

**Not every bound an input creates needs a solver behind it.** The hula
stage solves its crank offset from a gap written in the tips, in closed form
with an analytic derivative; an addendum that moved with the shift — which moves
with the offset — would put a tip-width solve inside that root-find and take the
derivative away with it. So there the bound *reports*: it says what the tooth
would have to be and leaves the number alone. One control, one meaning, honoured
by whatever the stage is able to move.

**A note that names an input is drawn under that input.** A shift raised to
clear undercut and an addendum held down to keep a tip both name a field and
carry a number, so they reach the reader beside that field rather than in a list
at the foot of the stage to be matched back up by tooth count. The stage's list
keeps what is about the stage.

**And where a bound cannot reach, the finding is reported instead.** `no
undercut` bounds a shift somebody *chooses*; a shift a relation leaves over is
chosen by nobody — an epicyclic absorber, or a hula pinion whose ring was pinned
— and answers to no bound at all. The control can be on, the tooth undercut, and
the two never meet. So every rack-cut stage member says whether its flank has
been eaten into, which is a remark about the part rather than a clamp: nothing
was altered, and the tooth is exactly the one the inputs describe. Severing is
the other side of it and *is* a clamp, since it truncates the profile.

**Where a clearance is read.** A clearance is taken by whatever is free to
absorb it, and each stage reports what it took rather than leaving a reader to
work it out. The centre distance absorbs it when the distance is automatic; the
shifts absorb it when they are being chosen, closing the pair to zero backlash a
clearance *inside* a given housing; and with neither free the input goes unread
and the answer says zero. The hula stage's minimum clearance is the same
question asked of its crank: it is what *sets* the offset, so a given offset
leaves it unread.

## Crossed axes

One model covers a worm stage and a crossed helical pair, and one stage type
too: a worm is a `PairStage` whose first member is sized by pitch diameter
rather than by helix angle, with the shift, addendum, dedendum and root round
every other member has. The *kind* — spur or worm — is a layer over that: a
preset, the words *starts* and *wheel*, which inputs a panel shows, and the
conventional proportions a worm's faces take. Nothing in the mathematics reads
it.

```text
sin γ = z m_n / d            exact, no iteration
γ = 90° − β    ⟹    sin γ = cos β
β₁ = Σ/2 + β_add      β₂ = Σ/2 − β_add      so β₁ + β₂ = Σ
```

**A profile shift enters a crossed mesh as a rack's does.** The line of action's
direction is fixed by the base helices and the shaft angle and cannot turn
(below), so a flank thickened by a shift `x` is the same involute helicoid
rotated about its axis, which moves it along that fixed normal by
`x m_n sin α_n` everywhere; separating the axes by `Δa` moves the flanks by
`Δa sin α_n` along the same normal. So

```text
a₀ = (d₁ + d₂)/2 + (x₁ + x₂) m_n         exact for involute helicoids
```

with no involute function and no operating pressure angle: the normal pressure
angle at the contact is `α_n` at any shift, because the normal is. The parallel
pair is the degeneracy — its line turns with the centres and `inv α_w` carries
the difference — and the two laws part company at second order in the shift,
the same step at `Σ = 0` the backlash projection has. Measured off the tooth
generator, which knows nothing of a screw pair, at five helix angles including a
worm's 82°. A consequence worth knowing: a shifted pair's contact is off the
common perpendicular even at its own zero-backlash distance, since contact on
the perpendicular is possible at the reference radii and nowhere else, and the
face widths are sized for where the contact actually is.

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
zone   = both members with ρ_n ≤ √(r_a² − r_b²)/cos β_b, each running one way
         from its tangency point toward the other's
ε      = zone length / (π m_n cos α_n)     the NORMAL base pitch
travel = zone length · sin β_b             along each member's own axis
```

**Interference is the same question the parallel mesh asks**, along the line:
a member's flank runs one way from its tangency point, and past that point
there is no involute to touch, so a mate whose reach crosses it is fouling
rather than in a longer zone. `CrossedPath::contact_radius_at` is
`mesh::conjugate_radius` with the tangency span in place of `a_w sin α_w`, and
the verdict — reached below where the involute hands over to the fillet, or
not touching involute at all — meets the parallel one a hundredth of a degree
off parallel on a grid with fouling cases in it. An earlier reading took both
sides of each tangency point and could count a fouling tip as contact.

The parallel case is a **degeneracy**, not a value: at `Σ = 0` the two conditions
on `n̂` collapse into one, the line becomes a plane, and contact spreads from a
point to a line. `path_of_contact` returns `None` there.

Which of the eight lines is the mesh is settled once at the **reference**
distance — where the reference cylinders touch and the pitch point lies on the
line — and carried, since which flanks face each other is not a function of
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

locked forwards  ⟺ μ ≥ cos α_n cot γ₁
locked backwards ⟺ μ ≥ cos α_n tan γ₁      ("self-locking")
```

**Both directions lock, and the two thresholds are one construction.** Each is
the friction at which the tangential force reaching the member the power *leaves
by* falls to zero — the wheel driving forward, the worm being back-driven — so
they are the same expression with the members swapped, and `Screw::locking_friction`
returns both. Only the backward one has a common name. A **negative** threshold
means no friction locks the pair that way, which is the usual answer forwards;
it is a value, not a missing one.

**Two friction coefficients.** Whether a stage turns at all is decided at rest
against a **static** coefficient; how well it turns once moving is decided
against the **sliding** one. `Directional::once_moving` is the whole rule, and
the static figure is never itself reported.

**The flank load comes from the torque the stage was given**, on the member it
was given on, in the direction that gives it. `T₁` and `T₂` above are one
balance, so the input torque read on the worm and the output torque read on the
wheel are the same normal force everywhere the pair transmits. They part company
at the one place that matters: a pair whose forward efficiency is **clamped to
zero** has an output torque of zero and flanks that are pressed just as hard by
whatever is holding it. Reading it on the driving member is also the only
self-consistent way to rate *along the path*, since the input torque is what the
shaft delivers at every instant while the output torque is a consequence that
varies with the local moment arm. Driving backward loads the other flank, which
flips the normal term and leaves the friction term alone — half a percent on the
shipped worm, and it is the same swap `η_backward` above is written from.

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

Not offered for a crossed gear pair, which has nothing wrapped round anything —
and it is the **kind** that says which, since the same 17/23 pair at 45° is a
worm drive if a designer calls it one and a gear pair otherwise. A gear pair's
automatic face is continuity's; a worm's is these, reported beside whatever
width is in use.

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
**Both the fillet and the flank are searched and the weaker tangency wins** —
Savage, Rubadeux & Coe: "both involute and trochoid geometry are used in checking
for the smallest inscribed parabola", and "the smaller x coordinate identifies
the weaker inscribed parabola", `x = s_Fn²/(4 h_Fe)`. Both candidates share a
load point and so share `cos α_Fen`, which makes a smaller `x` exactly a larger
`Y_F`. A ring's tangency is on the flank every time, which is one of the two
cases the model is told to search rather than a departure from it.
`CriticalSection::TangentAngle` retains the ISO tangent for a
standards-comparable number — **30° on an external tooth and 60° on a ring's**
(ISO 6336-3:2019, 6.1). Not because the tooth points the other way round — a
ring's tooth widens from tip to root as an external one does — but because the
shaper leaves it a fillet that curls into the rim, and the tooth flares into it
faster. Both angles are conventions, neither derived.

**Where the internal/external difference lives, in each construction.** The
tangent method reads the angle off `ToothOutline`, so its two members share one
solve, one bracket and one monotone condition and differ in that single number —
not in a test inside the search. The parabola reads no angle at all: its
tangency condition `X·Y′ + 2X′(y_v − Y) = 0` is odd in `y`, so negating the
frame leaves its zero set untouched and the *same* equation serves a tooth
pointing either way. So the default path does not consult the angle, and a
ring's 60° is reachable only by asking for the tangent construction explicitly.

**Two fillet radii, and each fit reads its own.** `ρ_F` is the radius **at the
critical section** — ISO's definition, and what `Y_S` was fitted to; on a flank
tangency it falls back to the fillet junction. `ρ_f` is the **minimum** over the
whole fillet — Dolan and Broghamer's definition, and what `K_f` was fitted to;
it is defined wherever the section ended up and needs no fallback. They are not
close: the junction is the flattest point the fillet has and the root the
tightest, and they differ by 1.4–4.1× on an external tooth and 2.1–6.3× on a
ring. `RootSection` carries both so that neither fit can be fed the other's.

**The ISO factors, for the comparable number.**

```text
σ_F0 = F_t / (b · m_n) · Y_F · Y_S · Y_β · Y_B · Y_DT
q_s  = s_Fn / (2 ρ_F)
```

**The default is not this product.** It is Savage, Rubadeux & Coe's, which is
the Lewis parabola section this crate already computes together with the notch
factor that belongs to it:

```text
σ_F = F_t / (b · m_n) · Y_F · K_f · Y_B
K_f = H + (s_Fn/ρ_f)^L · (s_Fn/h_Fe)^M
H = 0.331 − 0.436·α_n    L = 0.324 − 0.492·α_n    M = 0.261 + 0.545·α_n
```

`α_n` in radians, and **`ρ_f` is the minimum radius of curvature of the fillet
curve**, not the radius at the section.

**The load acts along the line of action, not across the tooth**, and both terms
say so. Its across-tooth component bends the root — that is `Y_F` — and its
along-tooth component pushes the tooth into its rim, relieving the tension
fillet by order 10 %. The two are the `6h/t_c² − tan φ_C/t_c` of Savage's `J`.
Carrying them separately is what lets the ISO set omit the second, as ISO does;
`RootStressModel` names the pair, not just the notch factor, for that reason.

The ISO product below is the other coherent set, reached by asking for the
tangent section and `Y_S` together.

| | | |
|---|---|---|
| `Y_F` | form factor | Measured off the generated profile, not ISO's Method B closed form |
| `Y_S` | stress correction | ISO 6336-3 7.2, over the band below. **Not the default** — see above |
| `Y_β` | helix angle | **Not applied** — half of a pair the 2019 edition revised together; see below |
| `Y_B` | rim thickness | ISO 6336-3 9.3, where a rim thickness was given |
| `Y_DT` | deep tooth | **Not applied.** `f_ε`, inside ISO's own `Y_F`, likewise |

Every declined factor's formulae and bands are recorded in
[`state.md`](state.md), so the decision can be revisited without the standard.

The `Y_S` fit is stated over `1 ≤ q_s < 8`, and outside it the correction is
taken at the boundary. **No stage reports that any more**, because no stage
applies `Y_S`: the band belongs to a fit reached only by asking for the ISO set
explicitly, and a note about a factor the rating does not use is worse than no
note. `K_f` states no band. The
same clause (7.1) says the fit is derived from **external spur gears at
`α_n = 20°`** and gives "approximate values for internal gears and for gears
having other pressure angles"; this crate applies it to both, which the standard
sanctions, and to a section located by the parabola rather than the tangent it
was calibrated against, which is this crate's own departure — see
[`rationale.md`](rationale.md#no-isoagma-correction-factors). None of the three
has an edge to report, so unlike `q_s` they are stated here rather than raised
per gear.

**`Y_β` and `f_ε` are one revision, and neither is taken.** The 2019 edition
modified `Y_β` and `Y_F` together; `f_ε` lives inside that `Y_F`, is `≤ 1`, and
cancels most of the `1/cos³β` that makes `Y_β` exceed 1. Their product is
0,73–0,79 at full axial overlap, so applying `Y_β` alone reports a *higher*
stress than the standard it was taken from. Against ISO 2019 this tool reads
1,26–1,36× at full overlap and 0,78–0,94× below `ε_β = 0,3` at high helix —
`tools/iso_6336_3_stack.py`, and
[rationale](rationale.md#the-helix-factors-are-a-pair-and-this-tool-can-take-neither).

**`Y_B`**, where a rim thickness `s_R` was given, is `a·ln(c/ratio)` never below
1 — `(1,6 · 2,242)` against the backup ratio `s_R/h_t` for a rack-cut member and
`(1,15 · 8,324)` against `s_R/m_n` for a ring. The clause's own breakpoints fall
out of the fit (`a ln(c/ratio) = 1` at 1,20005 and 3,48887 against its stated 1,2
and 3,5), so there is no breakpoint in the code. Below a backup ratio of 0,5, or
1,75 modules on a ring, the standard says the design shall be avoided; the
figure is still reported and the member says where it stands. A rim nobody
described is `Y_B = 1` and is **not** the same claim as a thick rim: only the
former cannot be told its rim is thin.

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

**What is left that differs between an external tooth and a ring.** One
construction, one notch factor, one load-point rule and one width law serve both;
the list below is everything the code still branches on, and each entry is a
*value* on [`ToothOutline`](../crates/gear-core/src/strength.rs) or a fact about
the part rather than a second model.

| | external | ring |
|---|---|---|
| Frame | `y` is the radius | `y` is the **negated** radius (`flip_y`) — and the parabola's tangency condition is odd in `y`, so the same equation serves both |
| Tangent angle | 30° | 60° — read only by `CriticalSection::TangentAngle`; the parabola asks for no angle |
| Cut by | a rack | a pinion shaper, so the fillet is a different trochoid and `Cutter` describes the tool |
| Module the section is measured in | normal module | transverse module; the helical conversion is the caller's, as it is for a rack-cut tooth |
| Fillet bracket | `(s_j, 0)`, root at 0 | `(min(s_root, s_j), max(...))`, root at `s_root` — which is why `fillet_root()` is a method and not an endpoint a caller picks |
| Load point travels | **down** in roll from the tip | **up** — `MeshKind::sign`, not a second construction |
| Rateable at all | a severed tooth is not | a cut that left **no fillet** is not — no fillet, no `ρ_f` |
| Undercut | asked, and bounded by `no undercut` | **not asked** — a ring's flank is its shaper's, and undercut is not a question that can be put to it |
| Rim factor `Y_B` | measured against the whole tooth depth `s_R/h_t` | against the normal module `s_R/m_n` — the clause's two references, one fit |
| Span over teeth | reported | not derived; between-pins only |
| Own buildable range | shown | not shown — a rack's range is not a ring's |

Everything else is **one code path taking both**: the parabola search, the
weaker-tangency rule, `ρ_f` at the fillet's minimum, `K_f`, the axial term, the
load-sharing sweep, the width law and the reversal rule. So is the entry point —
`bending_section_shared` is generic over `ToothOutline`, where it used to be two
near-identical functions, and `train::Bending::of` likewise. What a new kind of
member has to supply to be rated is that trait and nothing else.

One row that used to be here is gone. A ring carried a *generated* roll range
that an external tooth did not, on the reading that only a shaper-cut flank
stops before its bracket does. Both stop, and at the two ends of the same
bracket: an external involute runs from its fillet junction to its tip, a ring's
from its tip to the generation limit. `flank_bracket` was always the answer for
both, and the separate range was a duplicate of it for one.

**Three of the rows above are one fact.** Where the tip is on the flank bracket
(`tip_at_high_roll`) says where the load point is counted from, which way it
travels, and therefore which end the rating walks toward — stated once rather
than three times, which is [find the parameter, not the
branch](rationale.md#find-the-parameter-not-the-branch) applied to the one
asymmetry that is real.

**Minimum face width**, closed form, since `σ_F ∝ 1/b` and `σ_H ∝ 1/√b`:

```text
b_min,bending = b σ_F / σ_allow          b_min,contact = b (σ_H / σ_allow)²
```

**The `b` cancels**, and every factor of `σ_F0` this tool applies is independent
of the face width. That is worth stating because it was briefly untrue: `Y_β`
depends on `b` through the overlap ratio, and carrying it turned this division
into a two-branch solve. Declining `Y_β` gave the invariant back.


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

The first of the two is the reason internal gears are not built full depth. The
ring's tip can only touch the pinion's involute while
`√(r_a2² − r_b2²) ≥ a sin α_w`, and a standard ring misses it — a 60-tooth ring
on a 20-tooth pinion by 0.009 mm, and every pinion from 20 to 40 teeth by more.
The remedy is the ring's addendum or a shift, both of which are inputs; the
shipped epicyclic set is full depth at zero ring shift and **says so** on its
planet-ring mesh.

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

It is the condition that decides a one-tooth difference. On a hula stage at
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
by which pinning one of a hula stage's two members names the other as the
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

## The hula stage

Four gears in two pairs, all on one crank. Gears 1 and 4 sit on the fixed axis —
1 grounded, 4 the output — while gears 2 and 3 ride a body carried on an
eccentric, so **both pairs are separated by the same distance**, the crank's
offset, and that shared number is what makes the arrangement one mechanism
rather than two independent meshes.

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
([Limits](#limits)) is another; both rise with the offset, so the stage opens out
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
geometry claims. So the stage has exactly three unknowns, the offset and one
split per mesh, and each names what decides it; a system where every unknown
carries its own source cannot be over- or under-determined, which is why there is
no constraint count to check and no solve order to choose.

**Why the far-side gap is the constraint.** At one tooth of difference a ring and
its pinion very nearly fill each other, and with ordinary proportions their tip
circles *overlap* on the side away from the mesh — at `h = 0.8` and no shift the
gap is `−0.6 m`. A wobble body cannot orbit through that, so the gap is what the
shift is spent on, and it is why a stage of this kind runs at operating pressure
angles no ordinary pair would: 57° at `z = 18`, one tooth of difference and half
a millimetre of gap. Two teeth of difference is far kinder — 26° for the same gap
— at a quarter of the ratio, since `D = 4`.

**Two efficiencies, because one is not the other.** Each pair's own comes from
[`contact::efficiency`](#efficiency-parallel-axes) with the crank held, and the
two multiply — 99.15 % together on the shipped counts. The *stage's* comes from
the three-shaft power flow ([Planetary sets](#planetary-sets)) at
`i₀ = z₂z₄/(z₁z₃)`, and it is nowhere near the first, because power circulates:

<!-- figures-by-test: the_documented_tables_are_the_ones_this_code_prints -->
| reduction | meshes, crank held | the stage |
|---|---|---|
| 144 | 98.80 % | 36.8 % |
| 324 | 99.15 % | 26.6 % |
| 900 | 99.48 % | 17.5 % |
| 2500 | 99.68 % | 11.1 % |

The nearer the two meshes come to cancelling — which is what buys the reduction —
the more power goes round between them before any reaches the output, so a
*better* pair of meshes at a *higher* ratio is a worse stage. Every one of these
is self-locking: below half efficiency forward, the reversed flow has no state
where the output absorbs.

**The figure is the mechanism, not the model**, and the same code says so. Of
the sixteen arrangements, those whose wobble body carries two faces of the *same*
kind cancel, reduce by `z²`, and keep about a quarter; those carrying one of each
cancel nothing, reduce by about `z/2`, and keep ninety-odd percent — on the same
two meshes, losing the same 0.85 % between them.

<!-- figures-by-test: the_four_hula_studies_are_the_ones_this_code_prints -->
| arrangement | `D` | ratio | meshes | the stage |
|---|---|---|---|---|
| `N+1/N/N−1/N` | 1 | 324 | 99.15 % | 26.6 % |
| `N/N+1/N/N−1` | −1 | −323 | 99.15 % | 26.4 % |
| `N+1/N/N/N−1` | −36 | −8.5 | 99.15 % | 92.4 % |
| `N/N+1/N/N+1` | 37 | 9.8 | 99.17 % | 93.2 % |

**What the tool ships with is `N ± 4` about 61**, reducing 232.6:1 and keeping
79.6 % forward and 74.3 % back. Four teeth of difference cancels less than one does,
and that is the whole of the difference: the same code, the same two meshes, and
a stage that keeps two and a half times what the `N ± 1` arrangement does at a
comparable size.

It carries a shorter tooth with it — 0.7 module rather than the 0.8 an `N ± 1`
stage takes — because **the addendum belongs to the *difference* rather than to
the stage**. The operating pressure angle at four teeth is far lower than at
one, so a tooth that cleared the involute interference limit at one tooth of
difference reaches past it here. Measured across the proportion, on `N ± 4`
about 61:

<!-- figures-by-test: the_four_hula_studies_are_the_ones_this_code_prints -->
| `h_a` | involute interference | ε_α | the stage |
|---|---|---|---|
| 0.60 | clear | 1.19 | 88.0 % |
| 0.65 | clear | 1.28 | 83.5 % |
| 0.70 | clear | 1.37 | 79.6 % |
| 0.75 | **fouls** | 1.46 | 76.2 % |
| 0.80 | **fouls** | 1.54 | 73.1 % |

The threshold sits between 0.70 and 0.75, so 0.7 is the last proportion that
ships clean — and the taller tooth costs efficiency on the way as well, since a
longer path is a dearer one ([Efficiency](#efficiency-parallel-axes)). That
coupling is reported rather than assumed, and a stage taken to another
difference will want its own proportion.

This is the published behaviour of a Wolfrom set — efficiency falls as the
reduction rises, because the reduction *is* the cancellation — and it is why such
stages of this family are used where their efficiency does not matter. A **three-ring reducer**
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

<!-- figures: gear-cli hulaband 18 -->
| d | z | module | meshes | the stage | α_w | backlash out |
|---|---|---|---|---|---|---|
| 1 | 18 | 1.000 | 99.350 % | **32.3 %** | 45.1° | 0.378° |
| 2 | 36 | 0.500 | 99.767 % | **57.1 %** | 33.8° | 0.149° |
| 3 | 54 | 0.333 | 99.942 % | **84.2 %** | 26.2° | 0.079° |
| 4 | 72 | 0.250 | 99.980 % | **93.9 %** | 20.0° | 0.047° |
| 6 | 108 | 0.167 | 99.983 % | **94.7 %** | 17.8° | 0.028° |
| 9 | 162 | 0.111 | 99.920 % | **79.5 %** | 12.8° | 0.014° |

**These are optimised divisions, and they sit on a bound rather than at an
optimum.** The *sum* of a mesh's two shifts is never free — the crank offset is
what it is — so only the division is, and searching it (with the addendum and the
shaper) is what the rows above report. The stationary point of the loss is not
where they land: on these stages the mesh loses least at divisions of `+2.85`,
`+2.05` and `−2.55` for one, two and four teeth of difference, and none of the
three is admissible, because contact has gone discontinuous or the tips have
fouled well before. The rows through `d = 4` sit at `ε` between 1.01 and 1.03
with the tip margin at zero instead; from `d = 5` the winning row's contact
ratio rises again — 1.09, then 1.32 — which is the turn-over the last rows
show. The loss is still falling when the geometry runs out, so what a designer
wants to know is which bound stops it — and that is what these are.

Three times better at four teeth of difference than at one, on the same
reduction in the same envelope — and an eighth of the backlash. The mesh
figures explain it: a one-tooth pair has to be opened to 45° of operating
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

<!-- figures-by-test: the_four_hula_studies_are_the_ones_this_code_prints -->
| d | reduction | α_w | the pair keeps | the stage keeps |
|---|---|---|---|---|
| 2 | 324 | 33.7° | 99.891 % | 58.1 % |
| 3 | 144 | 25.9° | 99.965 % | 90.4 % |
| 4 | 81 | 21.3° | 99.958 % | 93.1 % |
| 5 | 52 | 19.2° | 99.948 % | 94.3 % |

**The two pairs land at the same operating pressure angle** — to a hundredth of
a degree, at every difference — not because the meshes are tied but because a
stage's two pairs are near twins, `(z+d, z)` against `(z, z−d)`, whose
independent optima coincide. That near-symmetry is why equal modules cost
nothing, and the table further down is what departing from them costs.

*(An earlier version of this table was written against a search that has since
been fixed four times over — F50, F52, F53 and F54 in `AUDIT.md` — and quoted a
module ratio the code never searched for. The figures here are the ones the test
named above holds.)*

**Against the lowest shifts that clear.** The default rule — take the least shift
that produces non-interfering geometry — gets the *sum* right and the *division*
wrong:

<!-- figures-by-test: the_four_hula_studies_are_the_ones_this_code_prints -->
| d | least loss (Σx, x_ring, x_pinion) | least shift | stage, best | stage, least |
|---|---|---|---|---|
| 2 | −0.17, +0.45, +0.28 | −0.19, +0.19, +0.00 | 58.08 % | 52.77 % |
| 3 | −0.08, +0.58, +0.50 | −0.10, +0.10, +0.00 | 90.39 % | 77.26 % |
| 4 | −0.02, +0.46, +0.44 | −0.04, +0.04, +0.00 | 93.09 % | 90.58 % |
| 5 | +0.01, +0.07, +0.08 | +0.01, −0.01, +0.00 | 94.25 % | 94.17 % |

The **sum is nearly the same either way** — within 0.03 at every difference —
because the crank offset sets it and the offset is solved from the far-side
clearance, which the shifts move only through the tip geometry. The whole
difference is the *division*: the optimum raises both shifts together, by about
0.3 to 0.5 at `d = 2..4`, while holding their difference, and the default has no
reason to. That is worth **13 points** of stage efficiency at `d = 3` and next to
nothing at `d = 5`, where the optimum has come down almost to the floor. The
stationary condition for a division is derived and verified where it applies
([Efficiency](#efficiency-parallel-axes)); on these stages the answer is as often
a bound as a stationary point, which is why it is searched.

**The two modules are separate inputs and want to be equal.** Nothing in the
arithmetic ties them — a pair's module is its own — so it is worth knowing that
moving them apart only costs. Both meshes run at one offset and each needs
`e ≥ a_ref cos α_t`, so below equality the larger mesh still binds, the offset
does not move, and the smaller mesh's reference distance falls away from it;
above equality the enlarged mesh binds instead and drags the offset up, pushing
the *other* mesh's angle out by rather more than the first one gained. At
`z = 18`, one tooth of difference, `h_a = 0.8` and 0.3 mm of gap:

<!-- figures-by-test: the_documented_tables_are_the_ones_this_code_prints -->
| m₁/m₂ | 0.80 | 0.90 | **1.00** | 1.10 | 1.30 |
|---|---|---|---|---|---|
| offset, mm | 0.807 | 0.807 | **0.807** | 0.879 | 1.022 |
| α_w, mesh 1 | 62.2° | 58.4° | **54.4°** | 54.0° | 53.3° |
| α_w, mesh 2 | 54.4° | 54.4° | **54.4°** | 57.7° | 62.6° |

Equal modules is a corner where both bounds are active at once, and the stage
efficiency falls away either side of it — 26.9 % at equality against 23.1 % at
0.8 and 25.5 % at 1.1 on the same search. So the design space is
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
tool: a third of a percent of mesh loss is 20 points of stage efficiency at these
ratios, which is the published 68.5 % → 89.0 % and matches the sensitivity this
model shows — half a point of mesh efficiency at 324:1 takes the stage from
26.6 % to 46.8 %.

**The stage builds what the arrangement describes.** A stage that closes
algebraically can still be one whose teeth foul, so each ring is cut by its
shaper and each pair is asked what it thinks: the contact ratio, the two
interference conditions, and the gap measured on the tips as cut beside the one
the arrangement solved for — they part company exactly where a tip was clamped.
Backlash is the pair's own, through the same `Mesh` relation every stage here
uses rather than a second one written for this arrangement.

**And it rates them like any other stage's members.** Four gears in a
`GearResult` each and two meshes in a `MeshReport` each — the same types a
planetary set reports in — so there is no figure a hula gear has that a spur
gear does not, and no arithmetic written twice to produce it
([Load cases](#load-cases)).

What the *arrangement* decides is where the load comes from, and it is not read
off "this stage's input" the way a pair's is: four gears sit on three shafts,
and only two of those shafts carry a torque the power flow reports. **Each mesh
is loaded by whichever of its members sits on the fixed axis** — mesh A by the
grounded gear's reaction, mesh B by the output's — and the member riding the
wobble body takes the same mesh force at its own radius. The three shaft torques
sum to zero, which is what says the two readings agree.

That load is the whole reason the ratings are worth having here. A reduction
multiplies torque as surely as it divides speed, so a stage turning 2 N·m into
195 puts its output pair under a load nothing about the input suggests, and the
grounded member reacts nearly all of it.

**No member of this stage is structurally reversed.** The wobble body carries
two gears rather than one and each of them meshes once, so — unlike a planet,
which the sun drives on one flank and the ring on the other — every root here is
loaded one way unless the drive itself reverses.

**A shaper has to be smaller than the ring it cuts**, and these rings are small.
A tool larger than its workpiece is clamped down to the ring's own tooth count
and then reaches none of its flank, leaving no fillet at all — an ordinary
mistake here rather than an exotic one, which is why the shipped cutters sit
well below the shipped rings.

Both terms of `dC/dα_w` are positive (`Σz < 0`), so the gap rises strictly with
the operating pressure angle and hence with the offset. The root is unique, and
taking the larger of the two meshes' requirements is safe: opening the stage out
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

**Load sharing.** A stage input, `LoadSharing`, **off by default**, on every
kind that reports a bending stress. It reaches bending alone — a contact rating
is already taken where one tooth carries everything, so sharing cannot move it —
and where it is off the rating is `bending_section`'s own answer rather than one
that agrees with it. Switched on, the mesh cycle is swept for the largest
`Y_F · K_f · share`; the share is `contact::load_share`, in base pitches from
the far end of the path, so the transverse path and the virtual spur gear ask
one function rather than two.

**One sweep, both kinds of member.** A ring's load point travels *up* in roll
away from its tip where an external tooth's travels down, and its flank stops at
the generation limit — which is the mesh kind's sign again rather than a second
construction. A ring had no shared section at all before, so a set that switched
the model on rated one member of an internal mesh under it and the other
without.

Above a virtual contact ratio of 2 there is no single-pair zone, the ramp never
reaches a full share, and it moves the figure by up to a quarter in *either*
direction — which the
stage reports, per mesh, since a set can have one mesh in the band and one out.

The share is `RAMP_MIN + (RAMP_MAX − RAMP_MIN)·t` with
`t = min(d, ε_αn − d)/(ε_αn − 1)` clamped to `[0,1]`, and it is written as that
`min` rather than as two ramps because above `ε_αn = 2` the two overlap and do
**not** meet where they are split. It is the entering and leaving ramps of one
expression, continuous everywhere, and identical to the two-branch form below
`ε_αn = 2` — where the single-pair plateau covers the whole region they could
differ in.
**Below it the model changes nothing**, and that is the model rather than a
plumbing fault: the single-pair boundary is in the sweep with a share of exactly
1, so the maximum is the point the unshared rating already took. A hula stage
cannot reach the band at any proportion it can be built at — its meshes run just
above continuous contact by construction — so the control is offered there and
provably cannot bite.

**Automatic face width.** Four ratings, four toggles per gear; the width is the
largest any *enabled* rating asks for. Peak contact is off by default — see
[rationale](rationale.md#a-contact-pressure-is-not-a-tensile-stress). With none
enabled there is nothing to invert, so the width **stands at the number in its
box** and the stage says so: an automatic value with nothing to choose between
has nothing to choose, and the alternative is a zero every rating is then divided
by.

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

**A member is rated over every mesh it is in, and the worst one answers.** Most
members are in one; a planet is in two, and a stage kind not yet written may put
one in more — so this is a fold over a list rather than a pair of names. The two
meshes a planet is in carry the *same* tangential force, since it is the same
planet transmitting through, and what separates them is the section each mesh's
contact ratio puts the load at and the width that mesh carries it over — the
narrower of its own pair. A narrow ring is the ordinary way for the second mesh
to be the worse one: at a 3 mm ring against 10 mm elsewhere, the ring mesh loads
the planet's root to 45.6 MPa where the sun mesh gives 16.6.

**An automatic face width is sized to the mesh, not to one gear.** The narrower
face carries the pair, so each automatic width resolves to the largest ask any
member of that mesh has. A member in two meshes — a planet — answers to both.

**And a member is *rated* at its mesh's width too**, not at its own. The load is
spread over the width the pair actually shares, so that is the width the stress
belongs to and the width the minimum is inverted at. Rating a member at its own
tells one that is wider than its mate that it needs face in proportion to how
much wider it is.

A gear's reported `torque` is the one it carries **driving forward**; a
back-driving load is reported beside it, not folded into it. Each is that
direction's own construction with the roles swapped — so a member at the far end
of a mesh carries the load referred by the ratio and cut by the loss the mesh
takes carrying it *that* way, which for a locked mesh is nought.

The peak *rating* uses whichever direction loads the teeth harder, **member by
member and mesh by mesh** — the maximum is taken after each direction's
distribution, never before it. A parallel-axis pair distributes one tangential
force the same way whichever end drives, so for it the two orders agree; a screw
pair and an epicyclic set distribute differently, and for them they do not.

The operating case may legitimately be **zero**, and a stage carrying nothing
rates at nothing rather than refusing to answer.

### Tooth cycles

Revolutions first. Intermittent: `(range/360) × Π(ratios between i and output)`
per actuation. Continuous: `rpm_i × 60 × hours`, where `rpm_i` is that shaft's
speed scaled from the peak the train was laid out at to the operating speed.

Then engagements, and **one rule covers every arrangement here**: a member's
teeth are engaged once per revolution *relative to the carrier of its mesh*,
once for each parallel mesh path.

```text
engagements_m = |ω_m − ω_carrier| / |ω_input| × N        per input revolution
```

A simple pair has no carrier and one path, so this is the member's own
revolutions and nothing more. An epicyclic set has both: in the carrier's frame
the arm stands still and everything else turns past it, which is what makes the
relative speed the one that counts — for a sun, a ring, a planet, a hula stage's
grounded gear and its wobble body alike. The consequence worth stating is the
one a per-member reading cannot: **a shaft that does not turn is still loaded.**
A held ring meets a planet once per *carrier* revolution, which is
`z_s/(z_s + z_r)` of the input's rather than none.

A hula stage is the same statement with one wobble body: `N = 1`, and the crank
is both the carrier and the shaft the revolutions were counted on, so each gear
counts how far it turns against the crank.

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

**A `null` that crosses is not always a `None`.** `serde_json` writes an
infinity and a NaN as `null`, which is indistinguishable from a field that
honestly has no value and draws as the same blank — so a figure that has gone
non-finite arrives looking exactly like one that was never available. The whole
result is therefore walked in the tests, and a `null` is allowed only at a field
named as being able to have none: a bending stress with no notch, a
back-driving load nothing reacts, a bound that does not exist on this geometry.
Anything else is a number that stopped being one.

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
