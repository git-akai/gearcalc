#!/usr/bin/env python3
"""A geartrain's speeds and torques, from rigid-body velocities alone.

`gear_core::kinematics` writes one row per mesh in the frame of the member
carrying its axes:

    z_a (w_a - w_f) + z_b (w_b - w_f) = 0

with the tooth counts signed, a ring's negative. That is compact, and compact is
exactly what cannot be checked by reading it: a sign or a frame put in wrong
still sums to zero, so the crate's own lock-up invariant is silent on both (it
was measured silent, which is why this file exists).

So this reaches the same answers from the other end and shares no expression
with them. Nothing below divides one tooth count by another, and nothing below
knows the words sun, ring or planet.

    tools/train_kinematics.py                # every topology
    tools/train_kinematics.py --verbose      # ...with the speeds printed

# What is derived here

**A layout, in millimetres.** Each gear has a pitch radius and an axis at some
offset from the axis of the frame that carries it. Two gears mesh when the
distance between their axes is r_a + r_b (external) or |r_a - r_b| (internal) --
which is *derived* from the layout rather than declared, so a topology whose
centre distances do not agree is refused here before any speed is asked for.
That is the same closure law the crate's epicyclic kinds solve a profile shift
to satisfy.

**A velocity, from rigid-body motion.** Gear a spins at w_a about an axis at
p_a, and that axis is carried by the frame, which spins at w_f about the origin.
The material point of gear a at the pitch point P therefore moves at

    v = w_f (z_hat x p_a) + w_a (z_hat x (P - p_a))

and the two gears' material points at P have the same velocity, because teeth in
contact do not slide along the line of the pitch circles. With every axis on the
x-axis this is one scalar equation per mesh. It contains no tooth count at all:
the counts enter only as radii, through `r = z/2` at one module.

**A torque, from virtual work.** For a lossless train the external torques do no
net work over any motion the structure allows, so `sum(T_i w_i) = 0` for every
solution of the velocity constraints. Written out for a basis of that solution
space, that is one equation per degree of freedom, and it determines the
reactions -- again with no formula from the crate in it.

# What this does not do, and what does

It is instantaneous: it writes the constraint at one position of the crank. The
hula stage's wobble is the case where that is worth doubting, and
`tools/hula_kinematics.py` is the answer to it -- that script integrates the
no-slip condition through a full revolution in 200_000 steps. The two ask
different questions and neither covers the other.

Losses are not here either. Efficiency is not a kinematic quantity, and the
crate's loss model is checked where it is written.
"""

import sys
from fractions import Fraction as F

# --------------------------------------------------------------- linear algebra
#
# Written out rather than imported, because a check that shares a solver with
# the thing it checks is checking half of it. Exact throughout: every quantity
# here is a quotient of integers, and a rank taken with a pivot tolerance is a
# rank somebody chose.


def rref(rows):
    """Reduced row echelon form. Returns (rows, pivot columns)."""
    rows = [list(r) for r in rows]
    width = len(rows[0]) - 1 if rows else 0
    pivots = []
    out = []
    for row in rows:
        for i, p in enumerate(pivots):
            if row[p]:
                f = row[p]
                row = [a - f * b for a, b in zip(row, out[i])]
        pivot = next((c for c in range(width) if row[c]), None)
        if pivot is None:
            if row[width]:
                raise ValueError("inconsistent")
            continue
        row = [a / row[pivot] for a in row]
        for i, r in enumerate(out):
            if r[pivot]:
                f = r[pivot]
                out[i] = [a - f * b for a, b in zip(r, row)]
        out.append(row)
        pivots.append(pivot)
    return out, pivots


def solve(rows, width):
    """A particular solution and a basis for what is left free."""
    reduced, pivots = rref(rows)
    particular = [F(0)] * width
    for i, p in enumerate(pivots):
        particular[p] = reduced[i][width]
    basis = []
    for c in range(width):
        if c in pivots:
            continue
        v = [F(0)] * width
        v[c] = F(1)
        for i, p in enumerate(pivots):
            v[p] = -reduced[i][c]
        basis.append(v)
    return particular, basis


# ------------------------------------------------------------------- the model


class Train:
    """Shafts, gears on them, and meshes between them -- as a layout."""

    def __init__(self):
        self.shafts = ["ground"]
        self.gears = []          # (name, shaft, frame, radius, offset)
        self.meshes = []         # (gear a, gear b)

    def shaft(self, name):
        self.shafts.append(name)
        return len(self.shafts) - 1

    def gear(self, name, shaft, frame, teeth, offset=F(0)):
        """A gear of `teeth` teeth at one module, on `shaft`, its axis carried
        by `frame` at `offset` from that frame's own axis -- a distance along
        the x-axis, or an `(x, y)` pair for an axis off it, which a Ravigneaux's
        second planet is."""
        at = tuple(F(v) for v in offset) if isinstance(offset, tuple) else (F(offset), F(0))
        self.gears.append((name, shaft, frame, F(teeth, 2), at))
        return len(self.gears) - 1

    def mesh(self, a, b):
        self.meshes.append((a, b))

    # -- geometry, which is where a mesh's kind comes from ------------------

    def pitch_point(self, a, b):
        """Where the two pitch circles touch, and a check that they touch at
        all.

        Both gears' axes are carried by the same frame, so their offsets are
        measured from one origin and the distance between the axes is the
        length of the difference. External contact wants r_a + r_b, internal
        |r_a - r_b|; the layout decides which, and a layout that is neither is
        refused. The distance is rational by construction of every layout
        here -- on the x-axis trivially, and off it by choosing counts whose
        triangle is Heronian -- so nothing is rounded.
        """
        (_, _, fa, ra, oa) = self.gears[a]
        (_, _, fb, rb, ob) = self.gears[b]
        if fa != fb:
            raise ValueError("a mesh's two gears must share a frame")
        d = (ob[0] - oa[0], ob[1] - oa[1])
        d2 = d[0] * d[0] + d[1] * d[1]
        if d2 == (ra + rb) ** 2:
            # External: the pitch point lies between the two axes.
            k = ra / (ra + rb)
            return (oa[0] + k * d[0], oa[1] + k * d[1])
        if d2 == (ra - rb) ** 2:
            # Internal: the smaller gear sits inside the larger, and the pitch
            # point is on the far side of it from the larger's axis.
            inner, outer = (a, b) if ra < rb else (b, a)
            (_, _, _, ri, oi) = self.gears[inner]
            (_, _, _, ro, oo) = self.gears[outer]
            if ro == ri:
                raise ValueError("an internal mesh of equal radii has no pitch point")
            k = ri / (ro - ri)
            return (oi[0] + k * (oi[0] - oo[0]), oi[1] + k * (oi[1] - oo[1]))
        raise ValueError(
            f"centre distance² {d2} is neither {(ra + rb) ** 2} nor {(ra - rb) ** 2}: "
            "the layout does not close"
        )

    # -- the constraints ----------------------------------------------------

    def rows(self):
        """One row per mesh, over the shaft speeds.

        `v = w_f (z x p) + w (z x (P - p))` for each gear at the pitch point,
        and the two are equal along the common tangent -- the only direction
        a material point at the pitch point of either gear can move, both
        `P - p` lying on the line of centres:

            [w_f * (z x o_a) + w_a * (z x (P - o_a))] . t
                = [w_f * (z x o_b) + w_b * (z x (P - o_b))] . t

        with `t = z x (P - o_a)`. With every axis on the x-axis every vector
        here points along y and the equation is the scalar one this file was
        first written with; off the axis it is the same equation with a dot
        product in it.
        """
        cross = lambda p: (-p[1], p[0])
        dot = lambda p, q: p[0] * q[0] + p[1] * q[1]
        out = []
        for (a, b) in self.meshes:
            (_, sa, fa, _, oa) = self.gears[a]
            (_, sb, _, _, ob) = self.gears[b]
            p = self.pitch_point(a, b)
            arm_a = cross((p[0] - oa[0], p[1] - oa[1]))
            arm_b = cross((p[0] - ob[0], p[1] - ob[1]))
            t = arm_a
            row = [F(0)] * (len(self.shafts) + 1)
            row[sa] += dot(arm_a, t)
            row[fa] += dot(cross(oa), t)
            row[sb] -= dot(arm_b, t)
            row[fa] -= dot(cross(ob), t)
            out.append(row)
        return out

    def speeds(self, conditions):
        """`conditions` maps a shaft index to a speed. Returns (particular,
        basis) over the shafts."""
        rows = self.rows()
        n = len(self.shafts)
        for shaft, value in conditions.items():
            row = [F(0)] * (n + 1)
            row[shaft] = F(1)
            row[n] = F(value)
            rows.append(row)
        return solve(rows, n)

    def torques(self, applied):
        """Reactions, from virtual work.

        `sum(T_i w_i) = 0` for every motion the structure allows, which is one
        equation per basis vector of the *unconditioned* velocity solution.

        `applied` maps a shaft to its **known** torque and every other shaft is
        solved for. A shaft carrying nothing is an explicit zero in it, not an
        omission: a first draft inferred "carries nothing" from "neither driven
        nor grounded", which puts a zero on the output shaft of every pair --
        the one place the load certainly is -- and the system came back
        inconsistent. *An absent thing is not a zero-length thing*
        (`docs/corrections.md`), met in a Python harness.
        """
        n = len(self.shafts)
        _, basis = solve(self.rows(), n)
        rows = []
        for v in basis:
            rows.append(list(v) + [F(0)])
        for shaft, value in applied.items():
            row = [F(0)] * (n + 1)
            row[shaft] = F(1)
            row[n] = F(value)
            rows.append(row)
        return solve(rows, n)


# ---------------------------------------------------------------- the crate's
#
# The relation `gear_core::kinematics` writes, restated so the two can be
# compared. This is the *only* place a tooth count appears as a coefficient.


def crate_rows(t, internal):
    """`z_a (w_a - w_f) + z_b (w_b - w_f) = 0`, the ring's count negative."""
    out = []
    for k, (a, b) in enumerate(t.meshes):
        (_, sa, fa, ra, _) = t.gears[a]
        (_, sb, _, rb, _) = t.gears[b]
        za, zb = ra * 2, rb * 2
        if k in internal:
            # Whichever of the two is the ring carries the negative count.
            if ra > rb:
                za = -za
            else:
                zb = -zb
        row = [F(0)] * (len(t.shafts) + 1)
        row[sa] += za
        row[sb] += zb
        row[fa] -= za + zb
        out.append(row)
    return out


def crate_speeds(t, internal, conditions):
    rows = crate_rows(t, internal)
    n = len(t.shafts)
    for shaft, value in conditions.items():
        row = [F(0)] * (n + 1)
        row[shaft] = F(1)
        row[n] = F(value)
        rows.append(row)
    return solve(rows, n)


def internal_meshes(t):
    """Which meshes the *layout* makes internal -- derived, not declared."""
    out = set()
    for k, (a, b) in enumerate(t.meshes):
        (_, _, _, ra, oa) = t.gears[a]
        (_, _, _, rb, ob) = t.gears[b]
        d2 = (ob[0] - oa[0]) ** 2 + (ob[1] - oa[1]) ** 2
        if d2 == (ra - rb) ** 2:
            out.add(k)
    return out


# ----------------------------------------------------------------- topologies


def fixed_pair(z1, z2, internal=False):
    t = Train()
    a, b = t.shaft("a"), t.shaft("b")
    offset = F(z2 - z1, 2) if internal else F(z1 + z2, 2)
    g1 = t.gear("1", a, 0, z1, 0)
    g2 = t.gear("2", b, 0, z2, offset)
    t.mesh(g1, g2)
    return t


def chain(z):
    """A run of fixed-axis pairs, each on its own shaft."""
    t = Train()
    at = F(0)
    gears = []
    for i, zi in enumerate(z):
        s = t.shaft(f"s{i}")
        gears.append(t.gear(f"g{i}", s, 0, zi, at))
        if i:
            at += F(z[i] + z[i + 1], 2) if i + 1 < len(z) else F(0)
    return t, gears


def simple_set(zs, zp, zr):
    """Sun, planet, ring, carrier. `z_r = z_s + 2 z_p` is not assumed: the
    carrier radius is taken from the sun mesh and the ring mesh has to agree."""
    t = Train()
    sun, carrier, ring, planet = (
        t.shaft("sun"),
        t.shaft("carrier"),
        t.shaft("ring"),
        t.shaft("planet"),
    )
    e = F(zs + zp, 2)
    gs = t.gear("s", sun, carrier, zs, 0)
    gp = t.gear("p", planet, carrier, zp, e)
    gr = t.gear("r", ring, carrier, zr, 0)
    t.mesh(gs, gp)
    t.mesh(gp, gr)
    return t, dict(sun=sun, carrier=carrier, ring=ring, planet=planet)


def compound_set(zs, zp1, zr1, zp2, zr2):
    """A stepped planet: one carrier, one sun, two rings, a planet shaft
    carrying two gears. The two meshes to the rings must sit at the same carrier radius --
    the closure law every epicyclic here obeys -- and the layout says so.

    **At zero profile shift that is a condition on the counts**, and a strict
    one: with z_s = 24 and z_p1 = 18 the carrier radius is 21, so a second
    planet of 17 teeth admits z_r2 = 59 and nothing else. A real Wolfrom is
    designed at 58 and absorbs the millimetre in profile shift, which is
    exactly the solve `gear_core::planetary` performs and which this file
    deliberately does not have. So a refusal here is the *zero-shift* layout
    refusing, not the design being impossible -- and it is worth having,
    because it is the one place this check can say anything about closure at
    all."""
    t = Train()
    sun, carrier, r1, r2, planet = (
        t.shaft("sun"),
        t.shaft("carrier"),
        t.shaft("ring1"),
        t.shaft("ring2"),
        t.shaft("planet"),
    )
    e = F(zs + zp1, 2)
    gs = t.gear("s", sun, carrier, zs, 0)
    gp1 = t.gear("p1", planet, carrier, zp1, e)
    gp2 = t.gear("p2", planet, carrier, zp2, e)
    gr1 = t.gear("r1", r1, carrier, zr1, 0)
    gr2 = t.gear("r2", r2, carrier, zr2, 0)
    t.mesh(gs, gp1)
    t.mesh(gp1, gr1)
    t.mesh(gp2, gr2)
    return t, dict(sun=sun, carrier=carrier, ring1=r1, ring2=r2, planet=planet)


def meshed_planets(zs, zp1, zp2, zr):
    """Sun - planet A - planet B - ring, the two planets on their own shafts.
    The gutter toggle's other position, and no model change."""
    t = Train()
    sun, carrier, ring, pa, pb = (
        t.shaft("sun"),
        t.shaft("carrier"),
        t.shaft("ring"),
        t.shaft("pa"),
        t.shaft("pb"),
    )
    ea = F(zs + zp1, 2)
    eb = ea + F(zp1 + zp2, 2)
    gs = t.gear("s", sun, carrier, zs, 0)
    g1 = t.gear("p1", pa, carrier, zp1, ea)
    g2 = t.gear("p2", pb, carrier, zp2, eb)
    gr = t.gear("r", ring, carrier, zr, 0)
    t.mesh(gs, g1)
    t.mesh(g1, g2)
    t.mesh(g2, gr)
    return t, dict(sun=sun, carrier=carrier, ring=ring)


def hula(z):
    """Gears 1 and 4 on the fixed axis, 2 and 3 on a wobble body carried by a
    crank. Both meshes internal, and both at the crank's one offset -- which is
    the same closure law as a compound planet's, met from the other side."""
    t = Train()
    g1s, crank, wob, g4s = (
        t.shaft("g1"),
        t.shaft("crank"),
        t.shaft("wobble"),
        t.shaft("g4"),
    )
    e = F(abs(z[0] - z[1]), 2)
    if e != F(abs(z[3] - z[2]), 2):
        return None, None
    g1 = t.gear("1", g1s, crank, z[0], 0)
    g2 = t.gear("2", wob, crank, z[1], e)
    g3 = t.gear("3", wob, crank, z[2], e)
    g4 = t.gear("4", g4s, crank, z[3], 0)
    t.mesh(g1, g2)
    t.mesh(g3, g4)
    return t, dict(g1=g1s, crank=crank, wobble=wob, g4=g4s)


def layshaft(z_in, pairs, engaged):
    """An input and an output shaft on one axis, a layshaft beside them, one
    pair per ratio at the one distance -- every disengaged pair's output-side
    gear idling on a shaft of its own. The same arrangement
    `gear_core::train::arrangements::layshaft` lays out."""
    t = Train()
    inp, out, lay = t.shaft("input"), t.shaft("output"), t.shaft("lay")
    e = F(z_in[0] + z_in[1], 2)
    t.mesh(t.gear("in", inp, 0, z_in[0], 0), t.gear("lay0", lay, 0, z_in[1], e))
    for i, (on_lay, on_out) in enumerate(pairs):
        shaft = out if i == engaged else t.shaft(f"idler{i}")
        t.mesh(t.gear(f"lay{i + 1}", lay, 0, on_lay, e), t.gear(f"out{i}", shaft, 0, on_out, 0))
    return t, dict(input=inp, output=out, lay=lay)


def planocentric(zp, zr):
    """One planet on an eccentric carrier, one ring; the planet's own turn is
    the output."""
    t = Train()
    carrier, ring, planet = t.shaft("carrier"), t.shaft("ring"), t.shaft("planet")
    gp = t.gear("p", planet, carrier, zp, F(zr - zp, 2))
    gr = t.gear("r", ring, carrier, zr, 0)
    t.mesh(gp, gr)
    return t, dict(carrier=carrier, ring=ring, planet=planet)


def ravigneaux(zs1, zs2, zpl, zps, zr):
    """A small sun meshing the long planet, which meshes the ring; a large
    sun meshing the short planet, which meshes the long one. The short
    planet's axis is **off the line of centres**: it stands where its two
    distances put it, and the counts are chosen so that point is rational
    (the triangle of the three distances is Heronian)."""
    t = Train()
    s1, carrier, s2, ring, pl, ps = (
        t.shaft("sun1"),
        t.shaft("carrier"),
        t.shaft("sun2"),
        t.shaft("ring"),
        t.shaft("long"),
        t.shaft("short"),
    )
    r_l = F(zs1 + zpl, 2)
    r_s = F(zs2 + zps, 2)
    d = F(zpl + zps, 2)
    x = (r_l * r_l + r_s * r_s - d * d) / (2 * r_l)
    y2 = r_s * r_s - x * x
    y = F(int(y2.numerator ** 0.5 + 0.5), int(y2.denominator ** 0.5 + 0.5))
    if y * y != y2:
        raise ValueError(f"the short planet's position is irrational at these counts: y² = {y2}")
    g_s1 = t.gear("s1", s1, carrier, zs1, 0)
    g_s2 = t.gear("s2", s2, carrier, zs2, 0)
    g_l = t.gear("pl", pl, carrier, zpl, r_l)
    g_s = t.gear("ps", ps, carrier, zps, (x, y))
    g_r = t.gear("r", ring, carrier, zr, 0)
    t.mesh(g_s1, g_l)
    t.mesh(g_l, g_r)
    t.mesh(g_s2, g_s)
    t.mesh(g_s, g_l)
    return t, dict(sun1=s1, carrier=carrier, sun2=s2, ring=ring)


# ---------------------------------------------------------------------- cases


def compare(label, t, conditions, report, verbose, fail):
    """Both derivations, on one topology, and the laws both must obey."""
    n = len(t.shafts)
    try:
        mine, basis = t.speeds(conditions)
    except ValueError as e:
        print(f"  {label:<38} LAYOUT  {e}")
        return fail + 1
    theirs, _ = crate_speeds(t, internal_meshes(t), conditions)
    ok = mine == theirs
    # **Lock-up**: a train locked solid turns as one body, which both
    # derivations must admit. Asserted on this side too, because it is a
    # property of the mechanism rather than of either way of writing it.
    ones = [F(1)] * n
    for row in t.rows():
        if sum(c * w for c, w in zip(row[:n], ones)) != 0:
            ok = False
            print(f"  {label:<38} LOCKUP  a row does not admit solid rotation")
    shown = " ".join(
        f"{name}={mine[i]}" for i, name in enumerate(t.shafts) if i in report.values()
    )
    extra = f"  +{len(basis)} free" if basis else ""
    print(f"  {label:<38}{'ok' if ok else 'DISAGREE':>9}   {shown}{extra}")
    if not ok:
        print(f"      velocities: {mine}")
        print(f"      tooth rows: {theirs}")
    if verbose and basis:
        for v in basis:
            print(f"      free direction: {v}")
    return fail + (not ok)


def power_balance(label, t, applied, conditions, fail):
    """`sum(T w) = 0`, with the torques solved from virtual work and the speeds
    from the velocity constraints -- two answers from one matrix, and the law
    that ties them.

    Ground's reaction comes back **zero** on a pure epicyclic, and that is
    right rather than a miss: nothing in such a set meshes against ground, so no
    row touches it and no torque can reach it. The reaction is on the shaft that
    is actually held. It reaches ground the moment something meshes against it
    -- which is what a fixed-axis pair does, and where the 17/43 line's -120/17
    comes from.

    **Ground is a reference, not a part.** A train here has no housing: an
    element is fixed to ground, carries a load, or is attached to another
    element. A frame need not stand still either -- in a compound set it is a
    carrier, and it turns."""
    n = len(t.shafts)
    speeds, basis = t.speeds(conditions)
    if basis:
        print(f"  {label:<38} SKIP    not determined")
        return fail
    torques, free = t.torques(applied)
    if free:
        print(f"  {label:<38} SKIP    torques not determined")
        return fail
    power = sum(tq * w for tq, w in zip(torques, speeds))
    total = sum(torques)
    ok = power == 0 and total == 0
    print(
        f"  {label:<38}{'ok' if ok else 'FAIL':>9}   "
        f"power {power}  sum {total}  reaction at ground {torques[0]}"
    )
    return fail + (not ok)


def main():
    verbose = "--verbose" in sys.argv
    fail = 0

    print("\nfixed-axis pairs -- the mesh kind comes out of the layout\n")
    for (z1, z2, internal) in [(17, 43, False), (9, 37, False), (20, 60, True)]:
        t = fixed_pair(z1, z2, internal)
        report = {"a": 1, "b": 2}
        fail = compare(
            f"{z1}/{z2} {'internal' if internal else 'external'}",
            t,
            {0: 0, 1: 1},
            report,
            verbose,
            fail,
        )

    print("\nan epicyclic set, in all six arrangements\n")
    t, s = simple_set(24, 18, 60)
    for held in ("sun", "carrier", "ring"):
        for driven in ("sun", "carrier", "ring"):
            if held == driven:
                continue
            fail = compare(
                f"{driven} driven, {held} held",
                t,
                {0: 0, s[held]: 0, s[driven]: 1},
                s,
                verbose,
                fail,
            )
    print("\n...and with nothing held, which is a differential\n")
    fail = compare("sun driven, nothing held", t, {0: 0, s["sun"]: 1}, s, verbose, fail)

    print("\ncompound and meshed planets -- no model change, only ticks\n")
    # A stepped planet: one planet shaft, two gears, two rings at one carrier
    # radius. z_r = z_s + 2 z_p on the first half; the second ring answers to
    # the same carrier radius, which is what makes `z_r2` the design
    # variable. (A Wolfrom proper — one planet gear, two rings — closes only
    # by profile shift, which this zero-shift layout cannot write; the
    # crate's fixture carries it and the stepped rows are the same law.)
    t, s = compound_set(24, 18, 60, 17, 59)
    fail = compare(
        "stepped planet, ring1 held, sun driven",
        t,
        {0: 0, s["ring1"]: 0, s["sun"]: 1},
        s,
        verbose,
        fail,
    )
    t, s = meshed_planets(24, 18, 18, 96)
    fail = compare(
        "meshed planets, ring held, sun driven",
        t,
        {0: 0, s["ring"]: 0, s["sun"]: 1},
        s,
        verbose,
        fail,
    )

    print("\nthe arrangements the shape reaches -- the same rows, more of them\n")
    t, s = layshaft((17, 43), [(19, 41), (31, 29), (43, 17)], 1)
    fail = compare("layshaft, second pair engaged", t, {0: 0, s["input"]: 1}, s, verbose, fail)
    t, s = planocentric(30, 33)
    fail = compare(
        "planocentric 30/33, carrier in, ring held",
        t,
        {0: 0, s["ring"]: 0, s["carrier"]: 1},
        s,
        verbose,
        fail,
    )
    t, s = ravigneaux(18, 30, 22, 18, 62)
    for driven, held in (("sun1", "ring"), ("sun2", "ring"), ("sun1", "sun2")):
        fail = compare(
            f"ravigneaux, {driven} driven, {held} held",
            t,
            {0: 0, s[held]: 0, s[driven]: 1},
            s,
            verbose,
            fail,
        )

    print("\na hula stage -- two internal meshes on one crank\n")
    for z in ([65, 61, 57, 61], [18, 17, 17, 18], [19, 18, 17, 16]):
        t, s = hula(z)
        if t is None:
            print(f"  {str(z):<38} SKIP    the two meshes want different offsets")
            continue
        fail = compare(
            f"z {z}, gear 1 held, crank driven",
            t,
            {0: 0, s["g1"]: 0, s["crank"]: 1},
            s,
            verbose,
            fail,
        )

    print("\ntorque from virtual work, and the power it must balance\n")
    t = fixed_pair(17, 43)
    fail = power_balance("17/43 pair, 2 Nm in", t, {1: 2}, {0: 0, 1: 1}, fail)
    t, s = simple_set(24, 18, 60)
    for held in ("sun", "carrier", "ring"):
        for driven in ("sun", "carrier", "ring"):
            if held == driven:
                continue
            fail = power_balance(
                f"set, {driven} driven at 1, {held} held",
                t,
                {s[driven]: 1, s["planet"]: 0},
                {0: 0, s[held]: 0, s[driven]: 1},
                fail,
            )

    print()
    if fail:
        print(f"{fail} disagreement(s) between the two derivations")
        return 1
    print("every topology: rigid-body velocities and the tooth-count rows agree")
    return 0


if __name__ == "__main__":
    sys.exit(main())
