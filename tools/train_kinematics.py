#!/usr/bin/env python3
"""A geartrain's speeds, from rigid-body velocities alone.

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

**Held to the crate's output, not to a copy of its formula.** Every topology
`gear-cli kinematics` prints is rebuilt here as a layout, and each body's
exact speed in `tools/golden/kinematics.txt` -- the corpus `check_golden.sh`
holds to what the harness prints -- must equal the rigid-body one as a
rational. Nothing here restates the crate's row formula: a transcription of
it would check the transcription, and the corpus is what checks the crate.

    tools/train_kinematics.py                # every topology; exits 1 on a disagreement
    tools/train_kinematics.py --verbose      # ...with the speeds printed

# What is derived here

**A layout, in millimetres, and base circles.** Each gear has a base radius
`r_b = z m cos(alpha) / 2` and an axis at some offset from the axis of the
frame that carries it. Profile shift moves the cutting tool -- it changes
tooth thickness, tip and root, and where the operating pitch point falls --
and does not touch the base circle; involute flanks are generated from that
circle, and conjugate action between two involutes is insensitive to centre
distance, which is the defining property of the form. So nothing here has to
close: a layout is a layout, and the only thing a mesh asks of it is that the
two base circles admit the common tangent that is its line of action. Whether
that tangent is the one that crosses between the centres or the one that does
not -- an external mesh or an internal one -- is read off the base circles
themselves: disjoint circles mesh externally, intersecting ones internally.
The mesh sense is therefore a choice of tangent, not an arithmetic sign, and
shares no expression with `MeshKind::sign`.

**A velocity, from rigid-body motion.** Gear a spins at w_a about an axis at
p_a, and that axis is carried by the frame, which spins at w_f about the origin.
A material point of gear a at a point X on the line of action moves at

    v = w_f (z_hat x p_a) + w_a (z_hat x (X - p_a))

and the two flanks in contact at X have the same velocity component along the
line of action -- the involute's law -- which is one scalar equation per mesh
whatever X is, since `(X - p_a) . n_hat` is the base radius wherever X sits on
the tangent. It contains no tooth count at all: the counts enter only as base
radii. The line is built in floating point; each row is then read back as the
rational it is to a part in 10^9, and every solve below is exact.

# What this does not do, and what does

It is instantaneous: it writes the constraint at one position of the crank.
For involute teeth that is the whole answer -- the velocity ratio of an
involute pair is constant through the engagement, so a relation true at one
position is true at every one.

A crossed mesh (the worm, and the worm in `wormpair` and `mixed`) has no
planar line of action; its row is `w_b = r w_a` with `r` from the contact
condition on the flanks' common normal (`crossed_ratio`), its sign carried
from the parallel pair.

Losses are not here either. Efficiency is not a kinematic quantity, and the
crate's loss model is checked where it is written.
"""

import re
import sys
from fractions import Fraction as F
from pathlib import Path

GOLDEN = Path(__file__).resolve().parent / "golden" / "kinematics.txt"

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

# The pressure angle every layout here is cut at. Only the base radii depend
# on it, and every row is a ratio of base radii, so the choice cancels; it is
# here so the tangent construction is the real one and not a stand-in.
PRESSURE_ANGLE_DEG = 20.0


def base_radius(teeth):
    """`r_b = z m cos(alpha) / 2` at one module."""
    import math

    return teeth * math.cos(math.radians(PRESSURE_ANGLE_DEG)) / 2.0


def exact(row, unit):
    """A floating row read back as the rational it is, each coefficient over
    `unit` -- gear a's base radius -- so that `cos(alpha)` cancels and what is
    left is a small rational. Refused rather than rounded where a coefficient
    is not one to a part in 10^9."""
    out = []
    for c in row:
        q = F(c / unit).limit_denominator(10**6)
        if abs(float(q) - c / unit) > 1e-9:
            raise ValueError(f"a row coefficient {c / unit} is not a rational of small height")
        out.append(q)
    return out


class Train:

    """Shafts, gears on them, and meshes between them -- as a layout."""

    def __init__(self):
        self.shafts = ["ground"]
        self.gears = []          # (name, shaft, frame, teeth, offset)
        self.meshes = []         # (gear a, gear b)
        self.crossings = []      # (shaft a, shaft b, w_b / w_a), fixed axes

    def shaft(self, name):
        self.shafts.append(name)
        return len(self.shafts) - 1

    def gear(self, name, shaft, frame, teeth, offset=F(0)):
        """A gear of `teeth` teeth at one module, on `shaft`, its axis carried
        by `frame` at `offset` from that frame's own axis -- a distance along
        the x-axis, or an `(x, y)` pair for an axis off it, which a Ravigneaux's
        second planet is."""
        at = tuple(F(v) for v in offset) if isinstance(offset, tuple) else (F(offset), F(0))
        self.gears.append((name, shaft, frame, teeth, at))
        return len(self.gears) - 1

    def mesh(self, a, b):
        self.meshes.append((a, b))

    def crossed(self, a, b, ratio):
        """A crossed-axis mesh between two fixed shafts, `w_b = ratio w_a`,
        the ratio from `crossed_ratio`."""
        self.crossings.append((a, b, ratio))

    # -- geometry, which is where a mesh's kind comes from ------------------

    def line_of_action(self, a, b):
        """The common tangent to the two base circles that is the mesh's line
        of action, as a unit normal `n` with the line `{x : n . (x - p_a) =
        r_ba}`, and whether the mesh is internal.

        Both gears' axes are carried by the same frame, so their offsets are
        measured from one origin. Two base circles that do not meet admit the
        tangents that cross between the centres, and one of those is an
        external mesh's line of action; two that intersect admit only the
        tangents that do not cross, and one of those is an internal mesh's.
        Nothing about closure is asked: any distance at which the tangent
        exists is a distance the pair runs at, which is the involute's whole
        point. One inside the other admits no tangent and is no mesh.
        """
        (_, _, fa, za, oa) = self.gears[a]
        (_, _, fb, zb, ob) = self.gears[b]
        if fa != fb:
            raise ValueError("a mesh's two gears must share a frame")
        r_a, r_b = base_radius(za), base_radius(zb)
        d = (float(ob[0] - oa[0]), float(ob[1] - oa[1]))
        dist = (d[0] ** 2 + d[1] ** 2) ** 0.5
        e = (d[0] / dist, d[1] / dist)
        e_perp = (-e[1], e[0])
        if dist >= r_a + r_b:
            internal = False
            # Circle b on the far side of the line: n . (p_b - p_a) = r_a + r_b.
            c = (r_a + r_b) / dist
        elif dist > abs(r_b - r_a):
            internal = True
            # Both circles on the same side: n . (p_b - p_a) = r_a - r_b.
            c = (r_a - r_b) / dist
        else:
            raise ValueError(
                f"base circles of {r_a:.4f} and {r_b:.4f} at {dist:.4f}: one inside "
                "the other, no common tangent, no mesh"
            )
        s = (1.0 - c * c) ** 0.5
        n = (c * e[0] + s * e_perp[0], c * e[1] + s * e_perp[1])
        return n, internal

    def rows(self):
        """One row per mesh, over the shaft speeds -- exact, read back from
        the line of action.

        `v = w_f (z x p) + w (z x (X - p))` for each gear at a point X on the
        line of action, and the two are equal along the line, `t = z x n`:

            [w_f (z x o_a) + w_a (z x (X - o_a))] . t
                = [w_f (z x o_b) + w_b (z x (X - o_b))] . t

        and `(z x u) . (z x n) = u . n`, so the row is

            w_f (o_a . n) + w_a r_ba  =  w_f (o_b . n) + w_b (n . (X - o_b))

        with `n . (X - o_b)` the base radius of b, negative where the tangent
        crosses between the centres. The frame's coefficient is the difference
        of the two axes' distances to the line, which is what makes the row
        exact once it is read back: every coefficient is a base radius or a
        sum of two, and the common `cos(alpha)` cancels.
        """
        out = []
        for (a, b) in self.meshes:
            (_, sa, fa, za, oa) = self.gears[a]
            (_, sb, _, zb, ob) = self.gears[b]
            n, internal = self.line_of_action(a, b)
            r_a, r_b = base_radius(za), base_radius(zb)
            on_a = r_a
            on_b = r_b if internal else -r_b
            frame = (float(oa[0]) * n[0] + float(oa[1]) * n[1]) - (
                float(ob[0]) * n[0] + float(ob[1]) * n[1]
            )
            row = [0.0] * (len(self.shafts) + 1)
            row[sa] += on_a
            row[sb] -= on_b
            row[fa] += frame
            out.append(exact(row, on_a))
        for a, b, ratio in self.crossings:
            row = [F(0)] * (len(self.shafts) + 1)
            row[a] = ratio
            row[b] = F(-1)
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

# ----------------------------------------------------------------- topologies


def fixed_pair(z1, z2, internal=False):
    t = Train()
    a, b = t.shaft("a"), t.shaft("b")
    offset = F(z2 - z1, 2) if internal else F(z1 + z2, 2)
    g1 = t.gear("1", a, 0, z1, 0)
    g2 = t.gear("2", b, 0, z2, offset)
    t.mesh(g1, g2)
    return t


def crossed_ratio(z1, z2, sigma_deg):
    """`w2 / w1` for a crossed pair of `z1` and `z2` teeth (a worm's starts
    and its wheel's teeth) on shafts at `sigma_deg`, exact, from the contact
    condition on the flanks' common normal: the two material points at the
    pitch point move alike along it, `w1 (a1 x X).n = w2 (a2 x (X - A)).n`.
    The normal and the point come from `crossed_path.py`'s construction off
    the involute helicoids, the helices of one hand splitting the shaft angle.

    **The sign is carried from the parallel pair.** At a shaft angle near
    nought the pair is an external parallel pair and turns against itself;
    the shaft angle is then opened in small steps to the one asked, each step
    taking whichever of the two lines through the pitch point (drive and
    coast flank) continues the last step's ratio, so the answer is the
    parallel pair's sense carried continuously, not a convention written
    here. Only the magnitude is read at the end, as a small rational."""
    import math

    import numpy as np

    import crossed_path as cp

    alpha = math.radians(PRESSURE_ANGLE_DEG)

    def ratios(sigma):
        g1, g2 = cp.geometry(z1, sigma / 2, 1.0, alpha), cp.geometry(z2, sigma / 2, 1.0, alpha)
        a = g1["r"] + g2["r"]
        out = []
        for line in cp.lines_of_action(g1, g2, sigma):
            if line["off"] > 1e-9:
                continue
            n, p = line["n"], line["P"]
            x = p + (np.dot(np.array([g1["r"], 0.0, 0.0]) - p, n)) * n
            num = np.dot(np.cross(cp.AXIS_1, x), n)
            den = np.dot(np.cross(cp.axis_2(sigma), x - np.array([a, 0.0, 0.0])), n)
            out.append(num / den)
        return out

    target = math.radians(sigma_deg)
    first = math.radians(0.01)
    last = -z1 / z2  # the parallel external pair, which turns against itself
    last = min(ratios(first), key=lambda r: abs(r - last))
    for k in range(1, 201):
        sigma = first + (target - first) * k / 200
        last = min(ratios(sigma), key=lambda r: abs(r - last))
    q = F(float(last)).limit_denominator(10**6)
    if abs(float(q) - last) > 1e-9:
        raise ValueError(f"crossed ratio {last} is not a small rational")
    return q


def simple_set(zs, zp, zr, e=None):
    """Sun, planet, ring, carrier. `z_r = z_s + 2 z_p` is not assumed: the
    carrier radius is taken from the sun mesh, or given, and the other mesh
    runs at it; nothing has to close, only each pair of base circles admit
    its tangent."""
    t = Train()
    sun, carrier, ring, planet = (
        t.shaft("sun"),
        t.shaft("carrier"),
        t.shaft("ring"),
        t.shaft("planet"),
    )
    e = F(zs + zp, 2) if e is None else e
    gs = t.gear("s", sun, carrier, zs, 0)
    gp = t.gear("p", planet, carrier, zp, e)
    gr = t.gear("r", ring, carrier, zr, 0)
    t.mesh(gs, gp)
    t.mesh(gp, gr)
    return t, dict(sun=sun, carrier=carrier, ring=ring, planet=planet)


def compound_set(zs, zp1, zr1, zp2, zr2):
    """A stepped planet: one carrier, one sun, two rings, a planet shaft
    carrying two gears, every mesh at the one carrier radius. Nothing has
    to close: the radius is the sun mesh's reference one, and the two ring
    meshes run at it whatever their reference distances would be, as the
    involute lets them -- the profile shift that absorbs the difference is
    the crate's business, and the ratio is not its."""
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


def wolfrom(zp, zr1, zr2):
    """A Wolfrom proper: one planet gear meshing two rings a tooth apart at
    one carrier radius, the first ring held, the carrier in, the second ring
    out. The two rings' reference distances to the planet differ by half a
    module, so no zero-shift layout closes; the base circles do not care,
    and the row is the same law as the stepped planet's."""
    t = Train()
    carrier, r1, r2, planet = (
        t.shaft("carrier"),
        t.shaft("ring1"),
        t.shaft("ring2"),
        t.shaft("planet"),
    )
    e = F(zr1 - zp, 2)
    gp = t.gear("p", planet, carrier, zp, e)
    gr1 = t.gear("r1", r1, carrier, zr1, 0)
    gr2 = t.gear("r2", r2, carrier, zr2, 0)
    t.mesh(gp, gr1)
    t.mesh(gp, gr2)
    return t, dict(carrier=carrier, ring1=r1, ring2=r2, planet=planet)


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
    crank. Both meshes internal, and both at the crank's one offset."""
    t = Train()
    g1s, crank, wob, g4s = (
        t.shaft("g1"),
        t.shaft("crank"),
        t.shaft("wobble"),
        t.shaft("g4"),
    )
    # The crank's one offset, from the first pair; the second runs at it
    # whatever its own reference offset, as the involute lets it.
    e = F(abs(z[0] - z[1]), 2)
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
    `gear_core::train::arrangements::layshaft` lays out, and each pair is
    written the way round that one lists its members: the gear on the
    centreline, then its mate on the layshaft."""
    t = Train()
    inp, out, lay = t.shaft("input"), t.shaft("output"), t.shaft("lay")
    e = F(z_in[0] + z_in[1], 2)
    t.mesh(t.gear("in", inp, 0, z_in[0], 0), t.gear("lay0", lay, 0, z_in[1], e))
    for i, (on_out, on_lay) in enumerate(pairs):
        shaft = out if i == engaged else t.shaft(f"idler{i}")
        t.mesh(t.gear(f"out{i}", shaft, 0, on_out, 0), t.gear(f"lay{i + 1}", lay, 0, on_lay, e))
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
    reference distances put it."""
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
    y = F(float(r_s * r_s - x * x) ** 0.5)
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


def compound_chain(pairs):
    """Fixed-axis pairs in series, each pair's driven gear on one shaft with
    the next pair's driver."""
    t = Train()
    shafts = [t.shaft("s0")]
    at = F(0)
    for i, (za, zb) in enumerate(pairs):
        shafts.append(t.shaft(f"s{i + 1}"))
        a = t.gear(f"a{i}", shafts[i], 0, za, at)
        at += F(za + zb, 2)
        b = t.gear(f"b{i}", shafts[i + 1], 0, zb, at)
        t.mesh(a, b)
    return t, shafts


def pair_then_set(z1, z2, zs, zp, zr, e=None):
    """A pair whose driven shaft is an epicyclic set's sun, the ring held."""
    t = Train()
    s1, s2 = t.shaft("s1"), t.shaft("s2")
    carrier, ring, planet = t.shaft("carrier"), t.shaft("ring"), t.shaft("planet")
    t.mesh(t.gear("1", s1, 0, z1, 0), t.gear("2", s2, 0, z2, F(z1 + z2, 2)))
    gs = t.gear("s", s2, carrier, zs, 0)
    gp = t.gear("p", planet, carrier, zp, F(zs + zp, 2) if e is None else e)
    gr = t.gear("r", ring, carrier, zr, 0)
    t.mesh(gs, gp)
    t.mesh(gp, gr)
    return t, dict(s1=s1, s2=s2, carrier=carrier, ring=ring, planet=planet)


def set_then_pair(zs, zp, zr, z1, z2):
    """An epicyclic set whose ring shaft drives a fixed-axis pair."""
    t = Train()
    sun, carrier, ring, planet, out = (
        t.shaft("sun"), t.shaft("carrier"), t.shaft("ring"), t.shaft("planet"), t.shaft("out"),
    )
    gs = t.gear("s", sun, carrier, zs, 0)
    gp = t.gear("p", planet, carrier, zp, F(zs + zp, 2))
    gr = t.gear("r", ring, carrier, zr, 0)
    t.mesh(gs, gp)
    t.mesh(gp, gr)
    t.mesh(t.gear("1", ring, 0, z1, 0), t.gear("2", out, 0, z2, F(z1 + z2, 2)))
    return t, dict(sun=sun, carrier=carrier, ring=ring, planet=planet, out=out)


# ------------------------------------------------------- against the crate

# Sections this does not derive, by name, with the reason. None today; the
# count is held, so one added fails until it is derived or listed here.
UNDERIVED = {}


def golden_speeds(text=None):
    """`{section: {(label, where): speed}}`, exact, from the corpus's
    `graph` blocks: `b3     ring 2    of part 1  speed 16/3721`; and
    `{section: (from, to, total)}`: the headline path's end bodies (its first
    `path` line) and the `total ratio` the graph prints."""
    out, ratios, section, numbered = {}, {}, None, {}
    for line in (GOLDEN.read_text() if text is None else text).splitlines():
        m = re.match(r"^== (.+) ==$", line)
        if m:
            section = m.group(1)
            out[section] = {}
            numbered = {}
            continue
        m = re.match(r"^  path (b\d+)\s+-> (b\d+)\s", line)
        if m and section and section not in ratios:
            ratios[section] = [m.group(1), m.group(2), None, numbered]
        m = re.search(r"total ratio (.+?)\s*$", line)
        if m and section:
            ratios.setdefault(section, [None, None, None, numbered])[2] = m.group(1)
        if section and re.match(r"^  graph\s+no motion", line):
            out[section] = None
            continue
        m = re.match(
            r"^\s+b\d+\s+(.+?)\s+of (part \d+|the train)\s+speed (\S+)"
            r"(?: \+ (\S+) × ω\(b\d+ (.+)\))?$",
            line,
        )
        if m and section:
            body = (m.group(1), m.group(2))
            numbered[line.split()[0]] = body
            # A family: a speed plus a multiple of a free body's speed.
            out[section][body] = (F(m.group(3)), F(m.group(4) or 0), m.group(5))
    return out, ratios


def crate_cases():
    """Per section: the layout, which of its shafts each crate body is, and
    the speeds given -- the case `gear-cli kinematics` states, by body."""
    P1 = "part 1"
    cases = {}
    for name in ("pair", "helical"):
        t = fixed_pair(17, 43)
        cases[name] = (t, {("first", P1): 1, ("second", P1): 2}, {("first", P1): 1})
    t, s = simple_set(12, 30, 72)
    names = {(k, P1): v for k, v in s.items()}
    for driven in ("sun", "carrier", "ring"):
        for held in ("sun", "carrier", "ring"):
            if driven != held:
                cases[f"set-{driven}-{held}"] = (t, names, {(driven, P1): 1, (held, P1): 0})
    t, s = hula([65, 61, 57, 61])
    cases["hula"] = (
        t,
        {("carrier", P1): s["crank"], ("ring 1", P1): s["g1"], ("ring 2", P1): s["g4"],
         ("planet 1", P1): s["wobble"]},
        {("carrier", P1): 1, ("ring 1", P1): 0},
    )
    t, s = layshaft((17, 43), [(41, 19), (29, 31), (17, 43)], 1)
    cases["layshaft"] = (
        t,
        {("member 1", P1): s["input"], ("member 2", P1): s["lay"], ("member 5", P1): s["output"],
         ("member 3", P1): t.shafts.index("idler0"), ("member 7", P1): t.shafts.index("idler2")},
        {("member 1", P1): 1},
    )
    t, s = wolfrom(18, 60, 61)
    cases["wolfrom"] = (
        t,
        {("carrier", P1): s["carrier"], ("ring 1", P1): s["ring1"], ("ring 2", P1): s["ring2"],
         ("planet", P1): s["planet"]},
        {("carrier", P1): 1, ("ring 1", P1): 0},
    )
    t, s = compound_set(24, 18, 60, 17, 59)
    cases["stepped"] = (
        t,
        {("sun", P1): s["sun"], ("ring 1", P1): s["ring1"], ("ring 2", P1): s["ring2"],
         ("carrier", P1): s["carrier"], ("planet 1", P1): s["planet"]},
        {("sun", P1): 1, ("ring 1", P1): 0},
    )
    t, s = planocentric(30, 33)
    cases["planocentric"] = (
        t,
        {("carrier", P1): s["carrier"], ("ring", P1): s["ring"], ("planet", P1): s["planet"],
         # The output, joined to the planet by an offset coupling: its speed.
         ("coupled", P1): s["planet"]},
        {("carrier", P1): 1, ("ring", P1): 0},
    )
    t, s = meshed_planets(24, 18, 18, 96)
    cases["meshed-planets"] = (
        t,
        {("sun", P1): s["sun"], ("carrier", P1): s["carrier"], ("ring", P1): s["ring"],
         ("planet 1", P1): t.shafts.index("pa"), ("planet 2", P1): t.shafts.index("pb")},
        {("sun", P1): 1, ("ring", P1): 0},
    )
    t, s = ravigneaux(18, 30, 22, 18, 62)
    names = {("sun 1", P1): s["sun1"], ("carrier", P1): s["carrier"], ("sun 2", P1): s["sun2"],
             ("ring", P1): s["ring"], ("planet 1", P1): t.shafts.index("long"),
             ("planet 2", P1): t.shafts.index("short")}
    for name, given in (
        ("ravigneaux-small-sun", {("sun 1", P1): 1, ("ring", P1): 0}),
        ("ravigneaux-large-sun", {("sun 2", P1): 1, ("ring", P1): 0}),
        ("ravigneaux-ring-free", {("sun 1", P1): 1, ("sun 2", P1): 0}),
    ):
        cases[name] = (t, names, given)
    t, sh = compound_chain([(17, 43), (13, 31)])
    cases["chain"] = (
        t,
        {("first", P1): sh[0], ("second", P1): sh[1], ("second", "part 2"): sh[2]},
        {("first", P1): 1},
    )
    t, s = simple_set(12, 30, 72)
    names = {(k, P1): v for k, v in s.items()}
    cases["ring-released"] = (t, names, {("sun", P1): 1})
    cases["conflict"] = (t, names, {("sun", P1): 1, ("carrier", P1): 0, ("ring", P1): 0})
    # The set no planet shift closes (17/17/80): the carrier radius is the
    # ring mesh's, and the sun mesh runs there -- the base circles admit both
    # tangents, which is all the involute asks.
    t, s = simple_set(17, 17, 80, e=F(80 - 17, 2))
    cases["unclosed"] = (t, {(k, P1): v for k, v in s.items()}, {("sun", P1): 1, ("ring", P1): 0})
    t, s = pair_then_set(17, 43, 17, 17, 80, e=F(80 - 17, 2))
    cases["chain-unclosed"] = (
        t,
        {("first", P1): s["s1"], ("second", P1): s["s2"], ("carrier", "part 2"): s["carrier"],
         ("ring", "part 2"): s["ring"], ("planet", "part 2"): s["planet"]},
        {("first", P1): 1, ("ring", "part 2"): 0},
    )
    worm = crossed_ratio(1, 40, 90.0)
    t = Train()
    a, b = t.shaft("first"), t.shaft("second")
    t.crossed(a, b, worm)
    cases["worm"] = (t, {("first", P1): a, ("second", P1): b}, {("first", P1): 1})
    t = Train()
    a, w, o = t.shaft("first"), t.shaft("wheel"), t.shaft("out")
    t.crossed(a, w, worm)
    t.mesh(t.gear("1", w, 0, 17, 0), t.gear("2", o, 0, 43, F(17 + 43, 2)))
    cases["wormpair"] = (
        t, {("first", P1): a, ("second", P1): w, ("second", "part 2"): o}, {("first", P1): 1}
    )
    t, s = pair_then_set(17, 43, 12, 30, 72)
    cases["pair-then-set"] = (
        t,
        {("first", P1): s["s1"], ("second", P1): s["s2"], ("carrier", "part 2"): s["carrier"],
         ("ring", "part 2"): s["ring"], ("planet", "part 2"): s["planet"]},
        {("first", P1): 1, ("carrier", "part 2"): 0},
    )
    t, s = set_then_pair(12, 30, 72, 17, 43)
    cases["set-then-pair"] = (
        t,
        {("sun", P1): s["sun"], ("carrier", P1): s["carrier"], ("ring", P1): s["ring"],
         ("planet", P1): s["planet"], ("second", "part 2"): s["out"]},
        {("sun", P1): 1, ("carrier", P1): 0},
    )
    t, s = pair_then_set(17, 43, 12, 30, 72)
    out = t.shaft("worm wheel")
    t.crossed(s["carrier"], out, worm)
    cases["mixed"] = (
        t,
        {("first", P1): s["s1"], ("second", P1): s["s2"], ("carrier", "part 2"): s["carrier"],
         ("ring", "part 2"): s["ring"], ("planet", "part 2"): s["planet"],
         ("second", "part 3"): out},
        {("first", P1): 1, ("ring", "part 2"): 0},
    )
    return cases


# Each section's headline path, input body to output body: the case's
# driven end and the end it is loaded at. Held to the corpus's first `path`
# line where it prints one, and the total ratio read along it where not.
P1, P2, P3 = "part 1", "part 2", "part 3"
SET_OUT = {"sun": {"carrier": "ring", "ring": "carrier"},
           "carrier": {"sun": "ring", "ring": "sun"},
           "ring": {"sun": "carrier", "carrier": "sun"}}
ENDS = {
    "pair": (("first", P1), ("second", P1)),
    "helical": (("first", P1), ("second", P1)),
    "worm": (("first", P1), ("second", P1)),
    **{f"set-{d}-{h}": ((d, P1), (o, P1)) for d, hs in SET_OUT.items() for h, o in hs.items()},
    "hula": (("carrier", P1), ("ring 2", P1)),
    "layshaft": (("member 1", P1), ("member 5", P1)),
    "wormpair": (("first", P1), ("second", P2)),
    "wolfrom": (("carrier", P1), ("ring 2", P1)),
    "stepped": (("sun", P1), ("ring 2", P1)),
    "planocentric": (("carrier", P1), ("coupled", P1)),
    "meshed-planets": (("sun", P1), ("carrier", P1)),
    "ravigneaux-small-sun": (("sun 1", P1), ("carrier", P1)),
    "ravigneaux-large-sun": (("sun 2", P1), ("carrier", P1)),
    "ravigneaux-ring-free": (("sun 1", P1), ("carrier", P1)),
    "chain": (("first", P1), ("second", P2)),
    "mixed": (("first", P1), ("second", P3)),
    "set-then-pair": (("sun", P1), ("second", P2)),
    "pair-then-set": (("first", P1), ("ring", P2)),
    "unclosed": (("sun", P1), ("carrier", P1)),
    "chain-unclosed": (("first", P1), ("carrier", P2)),
    "ring-released": None,
    "conflict": None,
}


def against_crate(verbose, text=None):
    """Every body speed the corpus records, against the rigid-body one."""
    fail = 0
    recorded, ratios = golden_speeds(text)
    cases = crate_cases()
    underived = 0
    for section, bodies in recorded.items():
        if section in UNDERIVED:
            print(f"  {section:<38}{'NOT':>9}   derived: {UNDERIVED[section]}")
            underived += 1
            continue
        if section not in cases:
            print(f"  {section:<38}{'NEW':>9}   the corpus has a section this does not build")
            fail += 1
            continue
        t, names, given = cases[section]
        conditions = {0: F(0)}
        for body, value in given.items():
            conditions[names[body]] = F(value)
        if bodies is None:
            # The crate finds no motion; the layout must find the conditions
            # inconsistent too.
            try:
                t.speeds(conditions)
                wrong, n = ["the crate finds no motion and the layout finds one"], 0
            except ValueError:
                wrong, n = [], 0
            print(f"  {section:<38}{'ok' if not wrong else 'DISAGREE':>9}   no motion either way")
            for w in wrong:
                print(f"      {w}")
            fail += bool(wrong)
            continue
        # A free body the crate names is solved at 0 and at 1: the particular
        # speed and the direction it adds.
        free = {f for (_, _, f) in bodies.values() if f}
        at = {}
        for w in (0, 1):
            c = dict(conditions)
            for label in free:
                c[names[next(b for b in names if b[0] == label)]] = F(w)
            speeds, basis = t.speeds(c)
            if basis:
                break
            at[w] = speeds
        if len(at) < 2:
            print(f"  {section:<38}{'FAIL':>9}   the given speeds leave it free")
            fail += 1
            continue
        wrong = []
        for body, (speed, per_free, _) in bodies.items():
            if body not in names:
                wrong.append(f"{body[0]} ({body[1]}): not in the layout")
                continue
            mine = (at[0][names[body]], at[1][names[body]] - at[0][names[body]])
            if mine != (speed, per_free):
                wrong.append(
                    f"{body[0]} ({body[1]}): crate {speed} + {per_free} w, "
                    f"rigid body {mine[0]} + {mine[1]} w"
                )
        # The headline ratio, the input's speed over the output's, along the
        # ends this file states and the corpus's first path, where it prints
        # one, names too.
        start, end, total, numbered = ratios.get(section, [None, None, None, {}])
        ends = ENDS.get(section, "missing")
        if ends == "missing":
            wrong.append("no ends stated for this section")
        elif total is None:
            wrong.append("the corpus prints no total ratio")
        elif total == "a family":
            if not free or ends is not None:
                wrong.append("the corpus calls the ratio a family where this finds one")
        elif ends is None:
            wrong.append(f"the corpus prints a total ratio {total} where this states no ends")
        else:
            if start is not None and (numbered.get(start), numbered.get(end)) != ends:
                wrong.append(f"the corpus's path is {numbered.get(start)} -> {numbered.get(end)}, "
                             f"not {ends[0]} -> {ends[1]}")
            a, b = names[ends[0]], names[ends[1]]
            if at[0][b] == 0 or at[0][a] / at[0][b] != F(total):
                got = "locked" if at[0][b] == 0 else at[0][a] / at[0][b]
                wrong.append(f"total ratio: crate {total}, rigid body {got}")
        ok = not wrong
        shown = " ".join(f"{b[0]}={v[0]}" for b, v in bodies.items()) if verbose else ""
        print(f"  {section:<38}{'ok' if ok else 'DISAGREE':>9}   {len(bodies)} bodies {shown}")
        for w in wrong:
            print(f"      {w}")
        fail += not ok
    for section in cases:
        if section not in recorded:
            print(f"  {section:<38}{'GONE':>9}   the corpus no longer has this section")
            fail += 1
    print(f"\n  {underived} section(s) not derived, of {len(UNDERIVED)} listed with a reason")
    return fail


# Faults planted in the recorded corpus, each of which must fail.
PLANTED = [
    ("a total ratio with no path line", "total ratio -4171/289", "total ratio -4171/290"),
    ("a worm's total ratio", "total ratio -40\n", "total ratio -41\n"),
    ("a worm wheel's sense", "second    of part 1  speed -1/40\n", "second    of part 1  speed 1/40\n"),
    ("an unclosed set's ratio", "total ratio 97/17", "total ratio 97/18"),
    ("a planet's speed", "planet    of part 1  speed -2/5\n", "planet    of part 1  speed 2/5\n"),
]


def self_test():
    """The recorded corpus passes, and each planted fault fails."""
    import contextlib
    import io

    text = GOLDEN.read_text()
    bad = 0
    with contextlib.redirect_stdout(io.StringIO()):
        clean = against_crate(False, text)
    if clean:
        print("the recorded corpus fails")
        bad += 1
    for name, old, new in PLANTED:
        assert old in text, name
        with contextlib.redirect_stdout(io.StringIO()):
            caught = against_crate(False, text.replace(old, new, 1))
        print(f"{'caught' if caught else 'MISSED'}  {name}")
        bad += not caught
    print("every planted fault fails" if not bad else f"{bad} problem(s)")
    return 1 if bad else 0


def main():
    if "--self-test" in sys.argv:
        return self_test()
    verbose = "--verbose" in sys.argv
    print("\nthe crate, as `gear-cli kinematics` recorded it (tools/golden/kinematics.txt)\n")
    fail = against_crate(verbose)
    print()
    if fail:
        print(f"{fail} section(s) where the crate and rigid-body velocities disagree")
        return 1
    print("every recorded section: the crate's speeds are the rigid-body ones")
    return 0


if __name__ == "__main__":
    sys.exit(main())
