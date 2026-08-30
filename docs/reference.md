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
m_t = m / cos β              tan α_t = tan α_n / cos β
r   = m_t z / 2              r_b     = r cos α_t
```

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
σ_H = max( σ_elliptical , σ_line )
σ_line = √( (F_n/b) (1/ρ₁ ± 1/ρ₂) E* / π )
```

collapses to the line term for every parallel-axis mesh, without a branch.
`PARALLEL_AXES` is the named zero.

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
```

Below the generation limit the ring's flank is not cut by an involute at all —
the internal analogue of undercut, and a property of the *pair*. It bites on
ordinary designs: 0.08 mm on a 43-tooth ring cut by a 20-tooth cutter.

Two mesh interference conditions come off the same conjugate relation, forwards
and backwards: the ring's tip cannot touch the pinion where that would fall
inside its base circle, and the pinion's tip cannot reach past where the ring's
flank ends.

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

**Automatic face width.** Four ratings, four toggles per gear; the width is the
largest any *enabled* rating asks for. With none enabled there is nothing to
invert and the width is zero, which the stage reports as a note.

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
the other — so it is rated against a reversed allowable, and that derate does
**not** stack with a reversing drive.

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
from the chord tolerance, tip and root arcs as true `ARC` entities where the
geometry is genuinely circular, and reference circles on a construction layer.
A ring also carries a rim circle at `r + 2 m_t` — a drawing convention with no
engineering meaning, and `Ring::rim_radius` is its one home.

**Geartrains and the material library**, TOML, the same shape as the input
structs. **Inputs only**, so files stay small and cannot go stale. A geartrain
document is `{ name, train }`; an unknown material is not an import failure, but
a train with no stages is refused.

---

## The boundary

Fifteen `#[wasm_bindgen]` entry points, JSON in and JSON out, all pure
functions. Three are not calculations: `defaults`, `strings`, and the geartrain
document's two directions.

Every type that crosses is declared to TypeScript by `ts-rs` into
`web/src/wire`, generated rather than hand-copied.
`tools/check_bindings.sh` regenerates and requires no diff.

A value that does not exist crosses as `Maybe::Unavailable`, carrying a `Note` —
a stable key and the values its sentence needs — exactly as a clamp does. Every
word the application shows lives in `crates/gear-io/data/strings_en.toml`, one
file per language.
